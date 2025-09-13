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

/// ツール（tools）を渡して、AIの「ツール呼び出し提案」または通常のテキスト回答を取得する（ツール実行はしない）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolCallDecision {
    /// モデルがそのまま返したテキスト回答
    Text(String),
    /// モデルが提案したツール呼び出し（最初の1件）
    ToolCall { name: String, arguments: String },
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
