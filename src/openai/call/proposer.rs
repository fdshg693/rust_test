use crate::config::Config;
use crate::openai::tools::ToolDefinition;
use async_openai::types::{
    ChatCompletionRequestMessage,
    ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs,
    CreateChatCompletionRequestArgs,
};
use async_openai::Client;
use color_eyre::Result;
use tokio::runtime::Runtime;
use tracing::{debug, info, instrument};

use super::types::ToolCallDecision;

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

    let tools_for_api: Vec<_> = tools.iter().map(|t| t.as_chat_tool()).collect();

    let mut messages: Vec<ChatCompletionRequestMessage> = Vec::with_capacity(1 + history.len() + 1);
    messages.push(system.into());
    messages.push(user.into());
    messages.extend_from_slice(history);

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
