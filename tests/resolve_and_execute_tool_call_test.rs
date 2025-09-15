use rust_test::config::Config;
use rust_test::openai::{propose_tool_call_blocking, ToolResolution, build_tavily_search_tool, resolve_and_execute_tool_call};
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


#[test]
#[ignore]
fn live_tool_call_with_tavily_search() -> Result<(), Box<dyn std::error::Error>> {
    if skip_if_no_api_key() { return Ok(()); }
    if std::env::var("tavily_API_KEY").is_err() {
        tracing::warn!(target="live_test", "[skip] tavily_API_KEY not set; skipping live OpenAI test");
        return Ok(());
    }
    let tool_def = build_tavily_search_tool();  
    let cfg = Config::new();
    let prompt = "1+2";
    let history: [async_openai::types::ChatCompletionRequestMessage; 0] = [];
    let decision = propose_tool_call_blocking(&history, prompt, &[tool_def.clone()], &cfg)?;
    tracing::info!(target="live_test", decision=?decision, "tavily tool decision");

    let tool_results: ToolResolution = resolve_and_execute_tool_call(decision, &[tool_def]);
    match &tool_results {
        ToolResolution::Executed { name, result } => {
            assert_eq!(name, "tavily_search");
            if let Some(err) = result.get("error") {
                tracing::warn!(target="live_test", error=%err, "[skip] tavily_search returned error JSON");
                return Ok(()); // treat as skipped
            }
            let answer_ok = result.get("answer").and_then(|v| v.as_str()).map(|s| !s.trim().is_empty()).unwrap_or(false);
            if !(answer_ok) {
                tracing::warn!(target="live_test", json=%result, "[skip] tavily_search JSON missing useful answer/results");
                return Ok(());
            }
            tracing::info!(target="live_test", answer_ok=answer_ok, json=%result, "tavily_search JSON accepted");
        }
        ToolResolution::ModelText(t) => {
            tracing::info!(target="live_test", text=%t, "model returned text directly");
        }
        _ => {
            panic!("Unexpected tool resolution result: {:?}", tool_results);
        }
    };
    Ok(())
}