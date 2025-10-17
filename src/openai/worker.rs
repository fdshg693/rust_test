//! OpenAI APIワーカー（TUIとは別スレッドで動く）
//!
//! 以前は旧 function calling API (`functions` フィールド) を直接扱っていたが、
//! 現在は `call_tool` モジュールの `propose_tool_call` と `tool` モジュールの
//! `ToolDefinition` を用いて 2 ステップ (提案→実行→最終回答) を実装する。

use crate::config::{Config};
use crate::openai::MultiStepAnswer;
use std::sync::mpsc::{Receiver, Sender};
use tokio::runtime::Runtime;
use tracing::{info};
use super::call::{multi_step_tool_answer_with_logger};
use super::tools::{
    build_number_guess_tool,
};

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
            while let Ok(prompt) = rx_prompt.recv() {
                info!(target: "openai", "prompt_received: {}", prompt);

                let tools = vec![
                    build_number_guess_tool(8, 10),
                ];

                // Use the user's prompt as-is; log multi-step events.
                let answer = multi_step_tool_answer_with_logger(&prompt, &tools, &config, Some(10), |ev| {
                    tracing::info!(target="live_test", event=%ev, "multi_step_event");
                }).await
                .unwrap_or_else(|_| MultiStepAnswer {
                    iterations: 0,
                    final_answer: "エラーが発生しました。".to_string(),
                    steps: vec![],
                    truncated: false,
                }).final_answer;
                info!(target: "openai", "answer_ready: {}", answer);
                let _ = tx_answer.send(answer);
            }
        });
    });
}

