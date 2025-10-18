//! シンプルな単発問い合わせAPI

use crate::config::OpenAIConfig;
use crate::openai::ConversationHistory;
use async_openai::Client;
use async_openai::types::{
    ChatCompletionTool
};
use color_eyre::Result;
use tokio::runtime::Runtime;
use tracing::{instrument};
use crate::openai::call::request_chat_completion;

/// 単純な1回の問い合わせでAIの回答を取得する（関数呼び出しやワーカーループなし）
#[instrument(name = "get_ai_answer_once", skip(config))]
pub async fn get_ai_answer_once(prompt: &str, config: &OpenAIConfig) -> Result<String> {
    let client = Client::new();

    // シンプルなsystem + user構成をConversationHistoryで構築
    let mut history = ConversationHistory::with_default_system();
    history.add_user(prompt);

    let empty_tools: Vec<ChatCompletionTool> = Vec::new();

    let req = request_chat_completion(&history, config, &empty_tools, "auto").await?;

    let resp = client.chat().create(req).await?;

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

