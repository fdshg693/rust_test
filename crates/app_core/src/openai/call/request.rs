use crate::config::OpenAIConfig;
use crate::openai::ConversationHistory;
use async_openai::types::{
    CreateChatCompletionRequestArgs,
    CreateChatCompletionRequest,
    ChatCompletionTool
};
use color_eyre::Result;
use tracing::{debug};

/// トークン制限戦略を表現する列挙型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenLimitStrategy {
    /// `max_tokens` を使用（4oモデル向け）
    MaxTokens,
    /// `max_completion_tokens` を使用（5系モデル向け）
    MaxCompletionTokens,
}

/// モデル名からトークン制限戦略を判定する
/// 
/// # Arguments
/// * `model` - モデル名
/// 
/// # Returns
/// トークン制限戦略
fn determine_token_limit_strategy(model: &str) -> TokenLimitStrategy {
    if model.contains("4o") {
        debug!(model = %model, strategy = "MaxTokens", "モデルは4oファミリー");
        TokenLimitStrategy::MaxTokens
    } else {
        debug!(model = %model, strategy = "MaxCompletionTokens", "モデルは5系ファミリー");
        TokenLimitStrategy::MaxCompletionTokens
    }
}

/// 会話履歴とモデル設定からChatCompletionリクエストを構築する
/// 
/// # Arguments
/// * `history` - 会話履歴
/// * `model` - モデル名
/// * `strategy` - トークン制限戦略
/// * `config` - OpenAI設定
/// 
/// # Returns
/// 構築されたChatCompletionリクエスト
fn build_request_with_strategy(
    history: &ConversationHistory,
    tools: &[ChatCompletionTool],
    tool_choice: &str,
    model: &str,
    strategy: TokenLimitStrategy,
    config: &OpenAIConfig,
) -> Result<CreateChatCompletionRequest> {
    let mut builder = CreateChatCompletionRequestArgs::default();
    builder
        .model(model)
        .messages(history.as_slice_with_system())
        .tools(tools)
        .tool_choice(tool_choice);

    // トークン制限戦略に応じてリクエストを構築
    let req = match strategy {
        TokenLimitStrategy::MaxTokens => {
            debug!(max_tokens = config.max_tokens, "max_tokensを適用します");
            builder.max_tokens(config.max_tokens).build()?
        }
        TokenLimitStrategy::MaxCompletionTokens => {
            debug!(max_completion_tokens = config.max_completion_tokens, "max_completion_tokensを適用します");
            builder.max_completion_tokens(config.max_completion_tokens).build()?
        }
    };

    Ok(req)
}

/// OpenAI ChatCompletion APIリクエストを構築する
/// 
/// # Arguments
/// * `history` - 会話履歴
/// * `config` - OpenAI設定
/// 
/// # Returns
/// 構築されたChatCompletionリクエスト
pub async fn request_chat_completion(
    history: &ConversationHistory,
    config: &OpenAIConfig,
    tools: &[ChatCompletionTool],
    tool_choice: &str,
) -> Result<CreateChatCompletionRequest> {
    // ステップ1: モデルからトークン制限戦略を判定
    let strategy = determine_token_limit_strategy(&config.model);
    debug!("トークン制限戦略を判定しました: {:?}", strategy);
    
    // ステップ2: 戦略に応じてリクエストを構築
    let req = build_request_with_strategy(history, tools, tool_choice, &config.model, strategy, config)?;
    
    debug!("ChatCompletionリクエストを構築しました");
    Ok(req)
}