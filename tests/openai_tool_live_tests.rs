use rust_test::config::Config;
use rust_test::openai::{propose_tool_call_blocking, ToolCallDecision};
mod common;

// Load .env before tests in this integration test binary
#[ctor::ctor]
fn _init() { common::init(); }

// Helper: skip test when no API key
fn skip_if_no_api_key() -> bool {
    if std::env::var("OPENAI_API_KEY").is_err() {
        tracing::warn!(target="live_test", "[skip] OPENAI_API_KEY not set; skipping live OpenAI test");
        true
    } else { false }
}

/// Live test: with no tools provided, model must return a text answer
#[test]
#[ignore]
fn live_tool_call_none_tools_returns_text() -> Result<(), Box<dyn std::error::Error>> {
    if skip_if_no_api_key() { return Ok(()); }

    let cfg = Config::new();
    let prompt = "1+1は？短く答えて。";
    let empty: [rust_test::openai::ToolDefinition; 0] = [];
    let history: [async_openai::types::ChatCompletionRequestMessage; 0] = [];
    let decision = propose_tool_call_blocking(&history, prompt, &empty, &cfg)?;
    tracing::info!(target="live_test", decision=?decision, "tool call decision");

    match decision {
        ToolCallDecision::Text(t) => assert!(!t.trim().is_empty(), "expected non-empty text"),
        ToolCallDecision::ToolCall { .. } => panic!("unexpected ToolCall when no tools were provided"),
    }
    Ok(())
}

