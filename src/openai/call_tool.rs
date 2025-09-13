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
        .content("あなたは簡潔な日本語で答えるアシスタントです。利用可能なら関数を使うか検討しますが、ここでは実行しません。")
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
