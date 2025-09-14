use crate::config::Config;
use crate::openai::ToolDefinition; // 引数型を ToolDefinition に変更
use async_openai::types::{
    ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs,
    CreateChatCompletionRequestArgs,
};
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

/// 非同期版: ツールを渡しても実行せず、モデルの提案だけ返す
#[instrument(name = "propose_tool_call", skip(config, tools))]
pub async fn propose_tool_call(
    prompt: &str,
    tools: &[ToolDefinition],
    config: &Config,
)
-> Result<ToolCallDecision> {
    let client = Client::new();

    let system = ChatCompletionRequestSystemMessageArgs::default()
        .content("あなたは簡潔な日本語で答えるアシスタントです。利用可能なら関数を使うか検討します。")
        .build()?;
    let user = ChatCompletionRequestUserMessageArgs::default()
        .content(prompt)
        .build()?;

    // ToolDefinition から ChatCompletionTool へ変換
    let tools_for_api: Vec<_> = tools.iter().map(|t| t.as_chat_tool()).collect();

    let req = CreateChatCompletionRequestArgs::default()
        .model(&config.model)
        .messages([system.into(), user.into()])
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
#[instrument(name = "propose_tool_call_blocking", skip(config, tools))]
pub fn propose_tool_call_blocking(
    prompt: &str,
    tools: &[ToolDefinition],
    config: &Config,
) -> Result<ToolCallDecision> {
    let rt = Runtime::new()?;
    rt.block_on(propose_tool_call(prompt, tools, config))
}

/// `propose_tool_call` と `resolve_and_execute_tool_call` を組み合わせ、
/// ツール呼び出し提案が行われなくなる (純テキスト応答になる) まで最大 N 回ループする。
/// 返り値は最終テキスト回答と、途中で発生した各ステップ (`ToolResolution`) の履歴。
///
/// 方式:
/// 1. 現在のプロンプト文字列に対して `propose_tool_call` を実行
/// 2. ツール提案なら `resolve_and_execute_tool_call` で実行し、結果 JSON を次のプロンプトへ組み込み
/// 3. テキストなら終了
/// 4. ループ上限 (既定 5) 到達で打ち切り
///
/// 注意: シンプル実装のため会話履歴は system + 逐次構築した 1 本のユーザープロンプト文字列のみ。
/// 真の Chat 履歴管理が必要になったら、`propose_tool_call` を汎用化して messages ベクタを受け取る形へ拡張すること。
#[instrument(name = "multi_step_tool_answer", skip(tools, config))]
pub async fn multi_step_tool_answer(
    original_user_prompt: &str,
    tools: &[ToolDefinition],
    config: &Config,
    max_loops: Option<usize>,
) -> Result<MultiStepAnswer> {
    let max_loops = max_loops.unwrap_or(5);
    let mut steps: Vec<ToolResolution> = Vec::new();
    let mut current_prompt = original_user_prompt.to_string();
    let mut truncated = false;

    for iteration in 1..=max_loops {
        debug!(target: "openai", iteration, "multi_step_iteration_start");
        let decision = propose_tool_call(&current_prompt, tools, config).await?;
        match decision {
            ToolCallDecision::Text(text) => {
                debug!(target: "openai", iteration, "multi_step_text_final");
                return Ok(MultiStepAnswer { final_answer: text, steps, iterations: iteration, truncated });
            }
            ToolCallDecision::ToolCall { name, arguments } => {
                debug!(target: "openai", iteration, tool = %name, "multi_step_tool_call" );
                let resolution = resolve_and_execute_tool_call(
                    ToolCallDecision::ToolCall { name, arguments },
                    tools,
                );
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
                    let final_answer = format!(
                        "途中でツール実行に失敗したためここまでの情報で回答します。\n元の質問: {original_user_prompt}\n{executed_json_for_next}"
                    );
                    return Ok(MultiStepAnswer { final_answer, steps, iterations: iteration, truncated });
                }

                // 成功したので次のプロンプトを組み立てる
                current_prompt = format!(
                    concat!(
                        "ユーザー最初の質問:\n{q}\n\n",
                        "これまでに実行したツール結果一覧 (最新が下):\n",
                        "{history}\n\n",
                    ),
                    q = original_user_prompt,
                    history = steps.iter().enumerate().map(|(i,s)| format!("[{}] {}", i+1, s)).collect::<Vec<_>>().join("\n"),
                );
            }
        }
    }

    truncated = true; // ループ上限
    let final_answer = format!(
        "最大ループ回数({})に達したため打ち切りました。ここまでのツール結果を要約して回答してください。(実装側で要約はしていません)\n履歴:\n{}",
        max_loops, steps.iter().enumerate().map(|(i,s)| format!("[{}] {}", i+1, s)).collect::<Vec<_>>().join("\n")
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
