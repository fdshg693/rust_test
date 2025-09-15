use rust_test::config::{Config, X, Y};
use rust_test::openai::{multi_step_tool_answer_blocking_with_logger, build_get_constants_tool, build_add_tool, ToolResolution};
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

/// Live multi-step function calling test (sample tools):
/// Use two tools from `sample_tools` to compute X+Y.
/// Flow:
/// 1) Model calls `get_constants` to retrieve `{ X, Y }` (provided by config constants)
/// 2) Model calls `add` with those numbers to compute the sum
/// 3) Model returns a concise Japanese explanation including the result
/// Ignored by default. Run with: `cargo test --test multi_step_read_docs_live_test -- --ignored`
#[test]
#[ignore]
fn live_multi_step_calculates_x_plus_y() -> Result<(), Box<dyn std::error::Error>> {
    if skip_if_no_api_key() { return Ok(()); }

    let cfg = Config::new();
    let tools = vec![
        build_get_constants_tool(X, Y),
        build_add_tool(),
    ];

    // Instruct model to fetch constants, then add them using tools, and answer in Japanese.
    let prompt = "提供されているツールだけを使い、まず定数 X と Y を取得し、その後それらを足し算して結果を日本語で簡潔に答えてください。";

    let answer = multi_step_tool_answer_blocking_with_logger(prompt, &tools, &cfg, Some(4), |ev| {
        tracing::info!(target="live_test", event=%ev, "multi_step_event");
    })?; // allow a few loops
    tracing::info!(target="live_test", final_answer=%answer.final_answer, iterations=answer.iterations, truncated=answer.truncated, steps=?answer.steps, "multi-step X+Y completed");

    // Basic sanity: final answer not empty
    assert!(!answer.final_answer.trim().is_empty(), "final answer should not be empty");

    // We expect at least one executed tool step; ideally two (get_constants then add). The model may choose a different order.
    let executed: Vec<&ToolResolution> = answer.steps.iter().filter(|s| matches!(s, ToolResolution::Executed { .. })).collect();
    assert!(!executed.is_empty(), "expected at least one executed tool step");

    // Check that the executed steps include our expected tools
    let used_get_constants = answer.steps.iter().any(|s| matches!(s, ToolResolution::Executed { name, .. } if name == "get_constants"));
    let used_add = answer.steps.iter().any(|s| matches!(s, ToolResolution::Executed { name, .. } if name == "add"));
    if !(used_get_constants && used_add) {
        tracing::info!(target="live_test", get_constants=used_get_constants, add=used_add, steps=?answer.steps, "model did not execute both tools");
    }

    // Assert that at least one of them ran
    assert!(used_get_constants || used_add, "expected at least one of get_constants or add to be executed");

    // Heuristic: final answer should mention the computed sum
    let sum = (X + Y) as i64; // sums in JSON are i64 in our tool
    let ans = answer.final_answer.replace(',', ""); // tolerate commas
    let sum_str = sum.to_string();
    assert!(ans.contains(&sum_str) || ans.contains("合計") || ans.contains("足し算") || ans.contains("和"), "final answer should likely include the sum or mention an addition");

    Ok(())
}
