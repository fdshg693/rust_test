//! シンプルな単発問い合わせAPI

use crate::config::OpenAIConfig;
use crate::openai::ConversationHistory;
use async_openai::types::{
    CreateChatCompletionRequestArgs,
};
use async_openai::Client;
use color_eyre::Result;
use tokio::runtime::Runtime;
use tracing::{info, debug, instrument};

/// 単純な1回の問い合わせでAIの回答を取得する（関数呼び出しやワーカーループなし）
#[instrument(name = "get_ai_answer_once", skip(config))]
pub async fn get_ai_answer_once(prompt: &str, config: &OpenAIConfig) -> Result<String> {
    let client = Client::new();

    // シンプルなsystem + user構成をConversationHistoryで構築
    let mut history = ConversationHistory::with_default_system();
    history.add_user(prompt);

    let req = CreateChatCompletionRequestArgs::default()
        .model(&config.model)
        .messages(history.as_slice_with_system())
        .max_completion_tokens(config.max_completion_tokens)
        // .max_tokens(config.max_tokens)
        .build()?;

    info!(target: "openai", "simple_request: model={}, max_tokens={}", config.model, config.max_tokens);
    let resp = client.chat().create(req).await?;
    debug!(target: "openai", "simple_response_choices: {}", resp.choices.len());

    let text = resp
        .choices
        .first()
        .and_then(|c| c.message.content.clone())
        .unwrap_or_else(|| "(空の応答)".to_string());
    Ok(text)
}

/// ランタイムを内部で作成してブロッキングで1回の回答を取得するヘルパー
#[instrument(name = "get_ai_answer_once_blocking", skip(config))]
pub fn get_ai_answer_once_blocking(prompt: &str, config: &OpenAIConfig) -> Result<String> {
    let rt = Runtime::new()?;
    rt.block_on(get_ai_answer_once(prompt, config))
}

