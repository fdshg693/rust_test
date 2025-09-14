//! OpenAI APIワーカー（TUIとは別スレッドで動く）
//!
//! 以前は旧 function calling API (`functions` フィールド) を直接扱っていたが、
//! 現在は `call_tool` モジュールの `propose_tool_call` と `tool` モジュールの
//! `ToolDefinition` を用いて 2 ステップ (提案→実行→最終回答) を実装する。

use crate::config::{Config};
use crate::openai::{
    propose_tool_call,
    ToolResolution,
    resolve_and_execute_tool_call,
    build_tavily_search_tool,
};
use async_openai::types::{
    ChatCompletionRequestFunctionMessageArgs,
    ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs,
    CreateChatCompletionRequestArgs,
};
use async_openai::Client;
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
        // build_get_constants_tool(X, Y),
        // build_read_doc_tool(),
        build_tavily_search_tool(),
    ];
    // 2. ツール呼び出し提案フェーズ（ToolDefinition のスライスを直接渡す）
    let decision = match propose_tool_call(prompt, &tools_defs, config).await {
        Ok(d) => d,
        Err(e) => {
            error!(target: "openai", "propose_tool_call_error: {e}");
            return format!("APIエラー(提案): {e}");
        }
    };
    // Display 実装を用いて改行をエスケープせず出力
    debug!(target: "openai", decision = %decision, "tool_call_decision");

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

    let resolution = resolve_and_execute_tool_call(decision, &tools_defs);
    debug!(target: "openai", resolution = %resolution, "tool_resolution");
    match resolution {
        ToolResolution::ModelText(t) => t,
        ToolResolution::ToolNotFound { requested } => format!("未知のツール要求: {requested}"),
        ToolResolution::ArgumentsParseError { name, raw: _, error } => format!("{name} の引数JSONパース失敗: {error}"),
        ToolResolution::ExecutionError { name, error } => format!("{name} 実行エラー: {error}"),
        ToolResolution::Executed { name, result } => {
            // 関数結果を function message として 2 回目呼び出し
            let function_message = match ChatCompletionRequestFunctionMessageArgs::default()
                .name(&name)
                .content(result.to_string())
                .build() {
                    Ok(m) => m,
                    Err(e) => return format!("関数結果メッセージ構築エラー: {e}"),
                };
            let second_req = match CreateChatCompletionRequestArgs::default()
                .model(&config.model)
                .messages([
                    system.into(),
                    user.into(),
                    function_message.into(),
                ])
                .max_tokens(config.max_tokens)
                .build() {
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
    use crate::openai::build_get_constants_tool;
    use serde_json::json;

    #[test]
    fn get_constants_tool_executes() {
        let t = build_get_constants_tool(1, 2);
        let out = t.execute(&json!({})).unwrap();
        assert_eq!(out["X"], 1);
        assert_eq!(out["Y"], 2);
    }
}
