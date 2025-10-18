use crate::config::OpenAIConfig;
use crate::openai::tools::ToolDefinition;
use crate::openai::ConversationHistory;
use crate::openai::call::request_chat_completion;
use async_openai::types::{
    ChatCompletionRequestMessage,
    ChatCompletionTool
};
use async_openai::Client;
use color_eyre::Result;
use tokio::runtime::Runtime;
use tracing::{instrument};

use super::types::ToolCallDecision;

#[instrument(name = "propose_tool_call", skip(config, tools, history), fields(history_len = history.len()))]
pub async fn propose_tool_call(
    history: &[ChatCompletionRequestMessage],
    prompt: &str,
    tools: &[ToolDefinition],
    config: &OpenAIConfig,
) -> Result<ToolCallDecision> {
    let client = Client::new();

    // ConversationHistoryを使用してメッセージを構築
    let mut full_history = ConversationHistory::with_default_system();
    if !prompt.is_empty() {
        full_history.add_user(prompt);
    }
    // 既存の会話履歴をマージ
    for msg in history {
        full_history.push(msg.clone());
    }

    let tools_for_api: Vec<ChatCompletionTool> = tools.iter().map(|t| t.as_chat_tool()).collect();

    let req = request_chat_completion(&full_history, config, &tools_for_api, "auto").await?;

    let resp = client.chat().create(req).await?;

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

#[instrument(name = "propose_tool_call_blocking", skip(config, tools, history))]
pub fn propose_tool_call_blocking(
    history: &[ChatCompletionRequestMessage],
    prompt: &str,
    tools: &[ToolDefinition],
    config: &OpenAIConfig,
) -> Result<ToolCallDecision> {
    let rt = Runtime::new()?;
    rt.block_on(propose_tool_call(history, prompt, tools, config))
}
