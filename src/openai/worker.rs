//! OpenAI APIワーカー（TUIとは別スレッドで動く）
//!
//! 以前は旧 function calling API (`functions` フィールド) を直接扱っていたが、
//! 現在は `call_tool` モジュールの `propose_tool_call` と `tool` モジュールの
//! `ToolDefinition` を用いて 2 ステップ (提案→実行→最終回答) を実装する。

use crate::config::{Config, X, Y};
use crate::openai::{propose_tool_call, ToolCallDecision, build_get_constants_tool, build_read_doc_tool};
use async_openai::types::{
    ChatCompletionRequestFunctionMessageArgs,
    ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs,
    CreateChatCompletionRequestArgs,
};
use async_openai::Client;
use serde_json::{json, Value};
use std::sync::mpsc::{Receiver, Sender};
use tokio::runtime::Runtime;
use tracing::{info, debug, error, instrument};

/// OpenAI APIワーカーを開始
pub fn start_openai_worker(
    rx_prompt: Receiver<String>,
    tx_answer: Sender<String>,
    config: Config,
) {
    std::thread::spawn(move || {
        // 専用スレッド内でTokioランタイムを構築
        let rt = Runtime::new().expect("tokio runtime");
        rt.block_on(async move {
            let client = Client::new(); // OPENAI_API_KEYを環境変数から読み取り

            while let Ok(prompt) = rx_prompt.recv() {
                info!(target: "openai", "prompt_received: {}", prompt);
                let answer = process_prompt(&client, &prompt, &config).await;
                info!(target: "openai", "answer_ready: {}", answer);
                let _ = tx_answer.send(answer);
            }
        });
    });
}

/// プロンプトを処理し、必要ならツールを実行して最終回答を得る。
#[instrument(name = "process_prompt", skip(client, config), fields(prompt_len = prompt.len()))]
async fn process_prompt(
    client: &Client<async_openai::config::OpenAIConfig>,
    prompt: &str,
    config: &Config,
) -> String {
    // 1. 利用可能ツール定義（将来的に増えるなら別関数化）
    let tools_defs = vec![
        build_get_constants_tool(X, Y),
        build_read_doc_tool(),
    ];
    let tools_for_api: Vec<_> = tools_defs.iter().map(|t| t.as_chat_tool()).collect();

    // 2. ツール呼び出し提案フェーズ
    let decision = match propose_tool_call(prompt, tools_for_api, config).await {
        Ok(d) => d,
        Err(e) => {
            error!(target: "openai", "propose_tool_call_error: {e}");
            return format!("APIエラー(提案): {e}");
        }
    };
    debug!(target: "openai", ?decision, "tool_call_decision");

    // 3. system/user メッセージを再構築（propose_tool_call 内と同様の方針で統一）
    let system = ChatCompletionRequestSystemMessageArgs::default()
        .content("あなたは簡潔な日本語で答えるアシスタントです。利用可能なら関数を使って事実を取得してください。")
        .build();
    let user = ChatCompletionRequestUserMessageArgs::default()
        .content(prompt)
        .build();
    let (system, user) = match (system, user) {
        (Ok(s), Ok(u)) => (s, u),
        (Err(e), _) | (_, Err(e)) => return format!("メッセージ構築エラー: {e}"),
    };

    match decision {
        ToolCallDecision::Text(t) => t, // そのまま回答
        ToolCallDecision::ToolCall { name, arguments } => {
            // 4. 対応ツールを探索
            let tool = match tools_defs.iter().find(|d| d.name == name) {
                Some(t) => t,
                None => return format!("未知のツール要求: {name}"),
            };

            // 5. 引数JSON をパース（失敗したら空オブジェクト）
            let args_val: Value = match serde_json::from_str(&arguments) {
                Ok(v) => v,
                Err(e) => {
                    debug!(target = "openai", "arguments_parse_error: {e}; using empty object");
                    json!({})
                }
            };

            // 6. ツール実行
            let tool_result = match tool.execute(&args_val) {
                Ok(v) => v,
                Err(e) => {
                    error!(target: "openai", "tool_execute_error: {e}");
                    json!({"error": format!("tool execution failed: {e}")})
                }
            };
            debug!(target: "openai", tool_name = tool.name, result = %tool_result, "tool_executed");

            // 7. 関数結果メッセージを組み立て  -> 最終回答取得
            let function_message = ChatCompletionRequestFunctionMessageArgs::default()
                .name(tool.name)
                .content(tool_result.to_string())
                .build();
            let function_message = match function_message {
                Ok(m) => m,
                Err(e) => return format!("関数結果メッセージ構築エラー: {e}"),
            };

            let second_req = CreateChatCompletionRequestArgs::default()
                .model(&config.model)
                .messages([
                    system.into(),
                    user.into(),
                    function_message.into(),
                ])
                .max_tokens(config.max_tokens)
                .build();

            let second_req = match second_req {
                Ok(r) => r,
                Err(e) => return format!("2回目リクエスト構築エラー: {e}"),
            };

            match client.chat().create(second_req).await {
                Ok(resp2) => resp2
                    .choices
                    .first()
                    .and_then(|c2| c2.message.content.as_ref())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "(空の応答)".to_string()),
                Err(e) => {
                    error!(target: "openai", "second_call_error: {e}");
                    format!("APIエラー(2回目): {e}")
                }
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::openai::build_get_constants_tool;

    #[test]
    fn get_constants_tool_executes() {
        let t = build_get_constants_tool(1, 2);
        let out = t.execute(&json!({})).unwrap();
        assert_eq!(out["X"], 1);
        assert_eq!(out["Y"], 2);
    }
}
