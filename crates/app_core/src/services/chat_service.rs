//! ChatService
//! 
//! OpenAI API連携のビジネスロジック層。
//! UI層（TUI/Web）から独立した形でチャット機能を提供。

use crate::config::OpenAIConfig;
use crate::openai::call::{multi_step_tool_answer_with_logger, MultiStepAnswer};
use crate::openai::tools::build_number_guess_tool;
use color_eyre::Result;

/// チャットサービス
pub struct ChatService {
    config: OpenAIConfig,
}

impl ChatService {
    /// 新しいChatServiceインスタンスを作成
    pub fn new(config: OpenAIConfig) -> Self {
        Self { config }
    }

    /// デフォルト設定でChatServiceを作成
    pub fn default() -> Self {
        Self::new(OpenAIConfig::default())
    }

    /// プロンプトを送信してAI回答を取得（ツールあり版）
    /// 
    /// # Arguments
    /// * `prompt` - ユーザーからの入力プロンプト
    /// 
    /// # Returns
    /// AI応答文字列
    pub async fn get_response(&self, prompt: &str) -> Result<String> {
        // ツールあり版を使用（Web/CLI両対応）
        let tools = vec![build_number_guess_tool(8, 10)];
        
        let answer = multi_step_tool_answer_with_logger(
            prompt, 
            &tools, 
            &self.config, 
            Some(10),
            |ev| {
                // Sendなクロージャ（キャプチャなし or Send型のみキャプチャ）
                tracing::info!(target: "chat_service", event=%ev, "step");
            }
        ).await?;
        
        tracing::info!(target: "chat_service", "AI response received with tools");
        Ok(answer.final_answer)
    }

    /// カスタムツールセットでレスポンスを取得
    /// 
    /// # Arguments
    /// * `prompt` - ユーザーからの入力プロンプト
    /// * `tools` - 使用するツールのリスト
    /// * `max_steps` - 最大ステップ数（Noneの場合はデフォルト10）
    /// 
    /// # Returns
    /// 完全なMultiStepAnswer（中間ステップ情報含む）
    pub async fn get_response_with_tools<F>(
        &self, 
        prompt: &str,
        tools: &[crate::openai::tools::ToolDefinition],
        max_steps: Option<usize>,
        logger: F,
    ) -> Result<MultiStepAnswer> 
    where 
        F: FnMut(&crate::openai::call::MultiStepLogEvent) + Send + 'static 
    {
        multi_step_tool_answer_with_logger(
            prompt, 
            tools, 
            &self.config, 
            max_steps,
            logger
        ).await
    }

    /// ストリーミング版（将来的にWebSocketで使用）
    /// TODO: OpenAI APIのstreaming機能を利用した実装
    pub async fn get_response_stream<F>(
        &self, 
        prompt: &str, 
        callback: F
    ) -> Result<String>
    where 
        F: Fn(&str) + Send + 'static 
    {
        // 現在は通常版を呼び出してコールバックで全体を返す
        let response = self.get_response(prompt).await?;
        callback(&response);
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_service_creation() {
        let service = ChatService::default();
        assert_eq!(service.config.model, "gpt-5");
    }

    #[test]
    fn test_chat_service_with_custom_config() {
        let mut config = OpenAIConfig::default();
        config.model = "gpt-4o-mini".to_string();
        let service = ChatService::new(config);
        assert_eq!(service.config.model, "gpt-4o-mini");
    }
}
