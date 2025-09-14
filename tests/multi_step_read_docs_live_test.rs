use rust_test::config::Config;
use rust_test::openai::{multi_step_tool_answer_blocking, build_read_doc_tool, ToolResolution};
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

/// Live multi-step function calling test:
/// Instruct the model to consult both benches.md and examples.md via the read_docs_file tool
/// and then provide a consolidated Japanese summary referencing benchmark usage and examples guidance.
/// Ignored by default. Run with: `cargo test --test multi_step_read_docs_live_test -- --ignored`
#[test]
#[ignore]
fn live_multi_step_reads_two_docs() -> Result<(), Box<dyn std::error::Error>> {
    if skip_if_no_api_key() { return Ok(()); }

    let cfg = Config::new();
    let tool = build_read_doc_tool();

    // Prompt asks explicitly for content from both files to push the model into 2 tool calls.
    let prompt = "`benches.md` と `examples.md` の両方を読み、それぞれの重要ポイントを日本語で1文に要約してください。";

    let answer = multi_step_tool_answer_blocking(prompt, &[tool], &cfg, Some(4))?; // allow a few loops
    tracing::info!(target="live_test", final_answer=%answer.final_answer, iterations=answer.iterations, truncated=answer.truncated, steps=?answer.steps, "multi-step doc read completed");

    // Basic sanity: final answer not empty
    assert!(!answer.final_answer.trim().is_empty(), "final answer should not be empty");

    // We expect at least one executed tool step; ideally two (one per file). The model may choose to fetch
    // them in any order or even merge into one, but we enforce at least one successful execution.
    let executed: Vec<&ToolResolution> = answer.steps.iter().filter(|s| matches!(s, ToolResolution::Executed { .. })).collect();
    assert!(!executed.is_empty(), "expected at least one executed tool step");

    // Collect filenames actually read
    let mut filenames = vec![];
    for s in &executed {
        if let ToolResolution::Executed { result, .. } = s {
            if let Some(fname) = result.get("filename").and_then(|v| v.as_str()) { filenames.push(fname.to_string()); }
        }
    }

    // If both were read, great; if not, just print a note (don't fail hard because model autonomy varies)
    let has_benches = filenames.iter().any(|f| f == "benches.md");
    let has_examples = filenames.iter().any(|f| f == "examples.md");
    if !(has_benches && has_examples) {
    tracing::info!(target="live_test", benches=has_benches, examples=has_examples, fetched=?filenames, "model did not fetch both target docs");
    }

    // Still, assert that at least one of the target docs was fetched
    assert!(has_benches || has_examples, "expected at least one of benches.md or examples.md to be fetched");

    // Light content heuristic: mention of 'ベンチ' or 'bench' and 'example' or 'examples'
    let ans_lower = answer.final_answer.to_lowercase();
    assert!(ans_lower.contains("bench") || answer.final_answer.contains("ベンチ"), "final answer should mention benches");
    assert!(ans_lower.contains("example") || ans_lower.contains("examples"), "final answer should mention examples");

    Ok(())
}
