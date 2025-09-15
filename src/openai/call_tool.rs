use crate::config::Config;
use crate::openai::ToolDefinition; // 引数型を ToolDefinition に変更
use async_openai::types::{
    ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs,
    ChatCompletionRequestMessage,
    CreateChatCompletionRequestArgs,
};
use crate::openai::ConversationHistory;
use async_openai::Client;
use color_eyre::Result;
use tokio::runtime::Runtime;
use tracing::{info, debug, instrument};
use serde_json::Value;
use std::fmt::{self, Display};

/// ツール（tools）を渡して、AIの「ツール呼び出し提案」または通常のテキスト回答を取得する（ツール実行はしない）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolCallDecision {
    /// モデルがそのまま返したテキスト回答
    Text(String),
    /// モデルが提案したツール呼び出し（最初の1件）
    ToolCall { name: String, arguments: String },
}

/// ツール提案を実際に実行した結果（最終回答を組み立てる前段階）
/// これは 2 ステップ function calling の "関数実行" 部分だけを共通化する目的。
#[derive(Debug, Clone, PartialEq)]
pub enum ToolResolution {
    /// そもそもツール提案でなく、モデルテキストが最終候補
    ModelText(String),
    /// ツールが存在し、引数JSONがパースされ、正常実行された: name と結果JSON
    Executed { name: String, result: Value },
    /// ツール名が一致しなかった
    ToolNotFound { requested: String },
    /// 引数JSONのパースに失敗（空オブジェクトで継続した場合など）
    ArgumentsParseError { name: String, raw: String, error: String },
    /// 実行中に handler がエラーを返した
    ExecutionError { name: String, error: String },
}

impl ToolResolution {
    /// 実行成功か判定用ヘルパ
    pub fn is_executed(&self) -> bool {
        matches!(self, ToolResolution::Executed { .. })
    }
}

// ----- Display 実装 (ログで ?debug ではなく %display を使い、生の改行をそのまま出したい) -----
impl Display for ToolCallDecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToolCallDecision::Text(t) => write!(f, "Text(len={}):\n{}", t.len(), t),
            ToolCallDecision::ToolCall { name, arguments } => {
                write!(f, "ToolCall name={} args={} (len={})", name, arguments, arguments.len())
            }
        }
    }
}

impl Display for ToolResolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToolResolution::ModelText(t) => write!(f, "ModelText(len={}):\n{}", t.len(), t),
            ToolResolution::Executed { name, result } => {
                // result をコンパクト表示 (pretty にすると非常に長い行が増える可能性があるので JSON そのまま)
                write!(f, "Executed name={} result={} (json)", name, result)
            }
            ToolResolution::ToolNotFound { requested } => write!(f, "ToolNotFound requested={}", requested),
            ToolResolution::ArgumentsParseError { name, raw, error } => {
                write!(f, "ArgumentsParseError name={} error={} raw={}", name, error, raw)
            }
            ToolResolution::ExecutionError { name, error } => {
                write!(f, "ExecutionError name={} error={}", name, error)
            }
        }
    }
}

/// マルチステップ実行結果: 途中の各ステップ(ツール解決)と最終回答テキスト
#[derive(Debug, Clone)]
pub struct MultiStepAnswer {
    pub final_answer: String,
    pub steps: Vec<ToolResolution>,
    pub iterations: usize,
    pub truncated: bool, // ループ上限で打ち切った場合 true
}

/// マルチステップ内部の進行状況を外部（テスト等）に通知するためのイベント
#[derive(Debug, Clone)]
pub enum MultiStepLogEvent {
    /// 各イテレーション開始
    IterationStart { iteration: usize },
    /// モデルのツール提案 or テキスト決定
    Proposed { iteration: usize, decision: ToolCallDecision },
    /// ツール解決（実行/失敗/テキスト委譲）
    Resolved { iteration: usize, resolution: ToolResolution },
    /// function メッセージを履歴に追加（成功時のみ）
    HistoryFunctionAppended { iteration: usize, name: String, result: Value },
    /// テキスト最終化
    FinalText { iteration: usize, text: String },
    /// 途中失敗により打ち切り
    EarlyFailure { iteration: usize, resolution: ToolResolution },
    /// ループ上限到達
    Truncated { max_loops: usize },
}

impl Display for MultiStepLogEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MultiStepLogEvent::IterationStart { iteration } => write!(f, "IterationStart #{}", iteration),
            MultiStepLogEvent::Proposed { iteration, decision } => write!(f, "Proposed @{} => {}", iteration, decision),
            MultiStepLogEvent::Resolved { iteration, resolution } => write!(f, "Resolved @{} => {}", iteration, resolution),
            MultiStepLogEvent::HistoryFunctionAppended { iteration, name, result } => write!(f, "HistoryFunctionAppended @{} name={} result={}", iteration, name, result),
            MultiStepLogEvent::FinalText { iteration, text } => write!(f, "FinalText @{} len={}", iteration, text.len()),
            MultiStepLogEvent::EarlyFailure { iteration, resolution } => write!(f, "EarlyFailure @{} => {}", iteration, resolution),
            MultiStepLogEvent::Truncated { max_loops } => write!(f, "Truncated after {} loops", max_loops),
        }
    }
}

/// 非同期版: ツールを渡しても実行せず、モデルのツール呼び出し提案 (function calling) またはテキスト回答を 1 回取得する。
///
/// # 引数
/// * `history` - 直近の会話履歴 (system を含めない)。`ChatCompletionRequestMessage` は
///   async-openai が提供する enum で user/assistant/function 等を表現できる。ここに積んだ順番は
///   そのまま送信順になる。空スライスなら「新規会話」とみなされる。
/// * `prompt`  - 今回ユーザーが追加で入力したい内容。内部で user メッセージとして末尾に 1 件追加される。
/// * `tools`   - 利用可能なツール定義一覧。
/// * `config`  - モデル名や max_tokens 等の設定。
///
/// # System メッセージ
/// 関数内部で定型の system メッセージを先頭に 1 件だけ追加する。`history` に system を重ねると二重になるため
/// 含めない運用とする (必要なら将来引数で system を差し替える拡張を検討)。
///
/// # 互換性
/// 以前は `(prompt, tools, config)` だけ受け取っていたが、会話文脈を保持するため `history` を先頭に追加した。
/// 既存呼び出しは `&[]` を与えれば従来と同じ挙動になる。
///
/// # 補助構造体
/// 頻繁に履歴を構築する場合は `ConversationHistory` ( `openai::ConversationHistory` ) を利用して
/// `history.as_slice()` もしくは `propose_tool_call_with_history_vec` で呼び出すと便利。
#[instrument(name = "propose_tool_call", skip(config, tools, history), fields(history_len = history.len()))]
pub async fn propose_tool_call(
    history: &[ChatCompletionRequestMessage],
    prompt: &str,
    tools: &[ToolDefinition],
    config: &Config,
) -> Result<ToolCallDecision> {
    let client = Client::new();

    let system = ChatCompletionRequestSystemMessageArgs::default()
        .content("あなたは簡潔な日本語で答えるアシスタントです。")
        .build()?;
    let user = ChatCompletionRequestUserMessageArgs::default()
        .content(prompt)
        .build()?;

    // ToolDefinition から ChatCompletionTool へ変換
    let tools_for_api: Vec<_> = tools.iter().map(|t| t.as_chat_tool()).collect();

    // メッセージ組み立て: system + history + current user
    let mut messages: Vec<ChatCompletionRequestMessage> = Vec::with_capacity(1 + history.len() + 1);
    messages.push(system.into());
    messages.extend_from_slice(history);
    messages.push(user.into());

    let req = CreateChatCompletionRequestArgs::default()
        .model(&config.model)
        .messages(messages)
        .tools(tools_for_api)
        .tool_choice("auto")
        .max_tokens(config.max_tokens)
        .build()?;

    info!(target: "openai", "propose_tool_call_request: model={}, max_tokens={}", config.model, config.max_tokens);
    let resp = client.chat().create(req).await?;
    debug!(target: "openai", "propose_tool_call_response_choices: {}", resp.choices.len());

    let choice = match resp.choices.first() {
        Some(c) => c,
        None => return Ok(ToolCallDecision::Text("(応答なし)".to_string())),
    };

    if let Some(tool_calls) = &choice.message.tool_calls {
        if let Some(first) = tool_calls.first() {
            let name = first.function.name.clone();
            let arguments = first.function.arguments.clone();
            return Ok(ToolCallDecision::ToolCall { name, arguments });
        }
    }

    let text = choice
        .message
        .content
        .clone()
        .unwrap_or_else(|| "(空の応答)".to_string());
    Ok(ToolCallDecision::Text(text))
}

/// Vec<ChatCompletionRequestMessage> をそのまま渡したい場合の薄いシンタックスシュガー。
/// 参照をスライスに変換して `propose_tool_call` を呼ぶだけ。
pub async fn propose_tool_call_with_history_vec(
    history_vec: &Vec<ChatCompletionRequestMessage>,
    prompt: &str,
    tools: &[ToolDefinition],
    config: &Config,
) -> Result<ToolCallDecision> {
    propose_tool_call(history_vec.as_slice(), prompt, tools, config).await
}

/// 与えられた `ToolCallDecision` を利用可能な `ToolDefinition` の集合に対して解決し、
/// 実際にツールを実行して `ToolResolution` を返すユーティリティ。
/// - `ToolCallDecision::Text` の場合は `ToolResolution::ModelText`
/// - `ToolCallDecision::ToolCall` の場合は: ツール検索→JSONパース→execute の順
/// 失敗時も panic せず詳細を enum で表現。
pub fn resolve_and_execute_tool_call(
    decision: ToolCallDecision,
    tools: &[ToolDefinition],
) -> ToolResolution {
    match decision {
        ToolCallDecision::Text(t) => ToolResolution::ModelText(t),
        ToolCallDecision::ToolCall { name, arguments } => {
            let tool = match tools.iter().find(|d| d.name == name) {
                Some(t) => t,
                None => return ToolResolution::ToolNotFound { requested: name },
            };
            // 引数 JSON パース
            let parsed: Value = match serde_json::from_str(&arguments) {
                Ok(v) => v,
                Err(e) => {
                    return ToolResolution::ArgumentsParseError {
                        name: tool.name.to_string(),
                        raw: arguments,
                        error: e.to_string(),
                    };
                }
            };
            match tool.execute(&parsed) {
                Ok(v) => ToolResolution::Executed { name: tool.name.to_string(), result: v },
                Err(e) => ToolResolution::ExecutionError { name: tool.name.to_string(), error: e.to_string() },
            }
        }
    }
}

/// ブロッキング版: ランタイムを内部で作って同期的に実行
#[instrument(name = "propose_tool_call_blocking", skip(config, tools, history))]
pub fn propose_tool_call_blocking(
    history: &[ChatCompletionRequestMessage],
    prompt: &str,
    tools: &[ToolDefinition],
    config: &Config,
) -> Result<ToolCallDecision> {
    let rt = Runtime::new()?;
    rt.block_on(propose_tool_call(history, prompt, tools, config))
}

/// `propose_tool_call` と `resolve_and_execute_tool_call` を組み合わせ、
/// モデルがツール呼び出しを提案しなくなる (テキスト回答を返す) まで最大 N 回ループする。
/// 返り値は最終テキスト回答と途中ステップ (`ToolResolution`) の履歴。
///
/// # 新実装 (ConversationHistory ベース)
/// 以前は単一の巨大 user プロンプト文字列を毎回再構築していたが、
/// 今回 `ConversationHistory` (system を除く履歴) を使い、
/// 1. 最初に user: original_user_prompt
/// 2. 各ツール実行成功後に function メッセージを append (name=resultJSON)
/// 3. 次 iteration では history + （必要なら追加の user 補助プロンプトなしで）再度提案を呼ぶ
/// とする。モデルは function messages を見て再度ツールが必要か判断する。
///
/// 補足: 失敗系 (ToolNotFound, ArgumentsParseError, ExecutionError) は即終了し、暫定回答を構築。
/// function message の content はそのまま JSON 文字列 (短縮/要約なし)。
#[instrument(name = "multi_step_tool_answer", skip(tools, config))]
pub async fn multi_step_tool_answer(
    original_user_prompt: &str,
    tools: &[ToolDefinition],
    config: &Config,
    max_loops: Option<usize>,
) -> Result<MultiStepAnswer> {
    multi_step_tool_answer_with_logger_internal(
        original_user_prompt,
        tools,
        config,
        max_loops,
        None,
    ).await
}

/// ロガー（コールバック）を指定可能な拡張版
#[instrument(name = "multi_step_tool_answer_with_logger", skip(tools, config, logger))]
pub async fn multi_step_tool_answer_with_logger(
    original_user_prompt: &str,
    tools: &[ToolDefinition],
    config: &Config,
    max_loops: Option<usize>,
    logger: impl FnMut(&MultiStepLogEvent),
) -> Result<MultiStepAnswer> {
    // ユーザーのロガーに加えて tracing にも流す
    let mut user_logger = logger;
    let mut log_and_forward = |ev: &MultiStepLogEvent| {
        debug!(target: "openai", event = %ev, "multi_step_event");
        user_logger(ev);
    };
    // `dyn FnMut` への可変参照を一時変数に保持してライフタイムを安定化
    let mut opt_logger: Option<&mut dyn FnMut(&MultiStepLogEvent)> = Some(&mut log_and_forward);
    multi_step_tool_answer_with_logger_internal(
        original_user_prompt,
        tools,
        config,
        max_loops,
        opt_logger.as_deref_mut(),
    ).await
}

// 内部共通実装（logger を Option で受ける）
async fn multi_step_tool_answer_with_logger_internal(
    original_user_prompt: &str,
    tools: &[ToolDefinition],
    config: &Config,
    max_loops: Option<usize>,
    mut logger: Option<&mut dyn FnMut(&MultiStepLogEvent)>,
) -> Result<MultiStepAnswer> {
    let max_loops = max_loops.unwrap_or(5);
    let mut steps: Vec<ToolResolution> = Vec::new();
    let mut truncated = false;
    let mut history = ConversationHistory::new();
    history.add_user(original_user_prompt);

    for iteration in 1..=max_loops {
        debug!(target: "openai", iteration, "multi_step_iteration_start");
        if let Some(cb) = logger.as_deref_mut() { cb(&MultiStepLogEvent::IterationStart { iteration }); }
        // 会話履歴 (system 除く) をそのまま用いてツール提案
        let decision = propose_tool_call(history.as_slice(), "", tools, config).await?; // 今回の追加 user 発話は空 (新情報なし)
        if let Some(cb) = logger.as_deref_mut() { cb(&MultiStepLogEvent::Proposed { iteration, decision: decision.clone() }); }
        match decision {
            ToolCallDecision::Text(text) => {
                debug!(target: "openai", iteration, "multi_step_text_final");
                if let Some(cb) = logger.as_deref_mut() { cb(&MultiStepLogEvent::FinalText { iteration, text: text.clone() }); }
                return Ok(MultiStepAnswer { final_answer: text, steps, iterations: iteration, truncated });
            }
            ToolCallDecision::ToolCall { name, arguments } => {
                debug!(target: "openai", iteration, tool = %name, "multi_step_tool_call" );
                let resolution = resolve_and_execute_tool_call(
                    ToolCallDecision::ToolCall { name, arguments },
                    tools,
                );
                if let Some(cb) = logger.as_deref_mut() { cb(&MultiStepLogEvent::Resolved { iteration, resolution: resolution.clone() }); }
                let executed_json_for_next = match &resolution {
                    ToolResolution::Executed { name, result } => {
                        // result をインラインで埋め込む
                        format!("ツール {name} の結果 JSON: {result}")
                    }
                    ToolResolution::ModelText(t) => format!("モデルテキスト: {t}"), // 通常ここには来ない想定
                    ToolResolution::ToolNotFound { requested } => {
                        format!("要求されたツール {requested} は存在しません。")
                    }
                    ToolResolution::ArgumentsParseError { name, raw, error } => {
                        format!("ツール {name} の引数パース失敗: {error}. RAW: {raw}")
                    }
                    ToolResolution::ExecutionError { name, error } => {
                        format!("ツール {name} 実行エラー: {error}")
                    }
                };
                // 履歴 push
                steps.push(resolution.clone());

                // 失敗系ならここで最終回答化して終了
                if !resolution.is_executed() {
                    if let Some(cb) = logger.as_deref_mut() { cb(&MultiStepLogEvent::EarlyFailure { iteration, resolution: resolution.clone() }); }
                    let final_answer = format!(
                        "途中でツール実行に失敗したためここまでの情報で回答します。\n元の質問: {original_user_prompt}\n{executed_json_for_next}"
                    );
                    return Ok(MultiStepAnswer { final_answer, steps, iterations: iteration, truncated });
                }

                // ツール成功結果を function メッセージとして履歴に追加
                if let ToolResolution::Executed { name, result } = &resolution {
                    history.add_function(name, result.to_string());
                    if let Some(cb) = logger.as_deref_mut() { cb(&MultiStepLogEvent::HistoryFunctionAppended { iteration, name: name.clone(), result: result.clone() }); }
                }
            }
        }
    }

    truncated = true; // ループ上限
    if let Some(cb) = logger.as_deref_mut() { cb(&MultiStepLogEvent::Truncated { max_loops }); }
    let final_answer = format!(
        "最大ループ回数({})に達したため打ち切りました。これまでの function 結果(JSON)を参考に最終回答をまとめてください。",
        max_loops
    );
    Ok(MultiStepAnswer { final_answer, steps, iterations: max_loops, truncated })
}

/// ブロッキング版 (同期): Tokio ランタイムを内部生成
#[instrument(name = "multi_step_tool_answer_blocking", skip(tools, config))]
pub fn multi_step_tool_answer_blocking(
    original_user_prompt: &str,
    tools: &[ToolDefinition],
    config: &Config,
    max_loops: Option<usize>,
) -> Result<MultiStepAnswer> {
    let rt = Runtime::new()?;
    rt.block_on(multi_step_tool_answer(original_user_prompt, tools, config, max_loops))
}

/// ブロッキング版（ロガー付き）
#[instrument(name = "multi_step_tool_answer_blocking_with_logger", skip(tools, config, logger))]
pub fn multi_step_tool_answer_blocking_with_logger(
    original_user_prompt: &str,
    tools: &[ToolDefinition],
    config: &Config,
    max_loops: Option<usize>,
    logger: impl FnMut(&MultiStepLogEvent),
) -> Result<MultiStepAnswer> {
    // 開始ログ（既存スタイルに合わせて target:"openai"）
    let max_loops_val = max_loops.unwrap_or(5);
    info!(target: "openai", model = %config.model, max_tokens = config.max_tokens, max_loops = max_loops_val, "multi_step_blocking_request");

    // ユーザーのロガーに加えて、tracing にもイベントを流すアダプタを用意
    let mut user_logger = logger;
    let mut log_and_forward = |ev: &MultiStepLogEvent| {
        // 文字列表現は Display 実装を利用
        debug!(target: "openai", event = %ev, "multi_step_event");
        user_logger(ev);
    };

    let mut opt_logger: Option<&mut dyn FnMut(&MultiStepLogEvent)> = Some(&mut log_and_forward);
    let rt = Runtime::new()?;
    let result = rt.block_on(multi_step_tool_answer_with_logger_internal(
        original_user_prompt,
        tools,
        config,
        max_loops,
        opt_logger.as_deref_mut(),
    ))?;

    // 終了ログ
    info!(target: "openai", iterations = result.iterations, truncated = result.truncated, steps = result.steps.len(), final_len = result.final_answer.len(), "multi_step_blocking_done");
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openai::build_get_constants_tool;

    #[test]
    fn resolve_text_passthrough() {
        let res = resolve_and_execute_tool_call(ToolCallDecision::Text("hello".into()), &[]);
        assert!(matches!(res, ToolResolution::ModelText(ref t) if t == "hello"));
    }

    #[test]
    fn resolve_tool_executes() {
        let tools = vec![build_get_constants_tool(3, 5)];
        let decision = ToolCallDecision::ToolCall { name: "get_constants".into(), arguments: "{}".into() };
        let res = resolve_and_execute_tool_call(decision, &tools);
        match res {
            ToolResolution::Executed { name, result } => {
                assert_eq!(name, "get_constants");
                assert_eq!(result["X"], 3);
                assert_eq!(result["Y"], 5);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn resolve_tool_not_found() {
        let decision = ToolCallDecision::ToolCall { name: "nope".into(), arguments: "{}".into() };
        let res = resolve_and_execute_tool_call(decision, &[]);
        assert!(matches!(res, ToolResolution::ToolNotFound { .. }));
    }

    #[test]
    fn resolve_args_parse_error() {
        let tools = vec![build_get_constants_tool(1, 2)];
        let decision = ToolCallDecision::ToolCall { name: "get_constants".into(), arguments: "{not json}".into() };
        let res = resolve_and_execute_tool_call(decision, &tools);
        assert!(matches!(res, ToolResolution::ArgumentsParseError { .. }));
    }
}
