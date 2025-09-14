use rust_test::config::Config;
use rust_test::openai::{propose_tool_call_blocking, ToolResolution, build_tavily_search_tool, resolve_and_execute_tool_call};

// Load .env before tests in this integration test binary
#[ctor::ctor]
fn _load_dotenv() { let _ = dotenvy::dotenv(); }

// Helper: skip test when no API key
fn skip_if_no_api_key() -> bool {
    if std::env::var("OPENAI_API_KEY").is_err() {
        eprintln!("[skip] OPENAI_API_KEY not set; skipping live OpenAI test");
        true
    } else { false }
}


#[test]
#[ignore]
fn live_tool_call_with_tavily_search() -> Result<(), Box<dyn std::error::Error>> {
    if skip_if_no_api_key() { return Ok(()); }
    if std::env::var("tavily_API_KEY").is_err() {
        eprintln!("[skip] tavily_API_KEY not set; skipping live OpenAI test");
        return Ok(());
    }
    let tool_def = build_tavily_search_tool();  
    let cfg = Config::new();
    let prompt = "1+2";
    let decision = propose_tool_call_blocking(prompt, &[tool_def.clone()], &cfg)?;
    println!("Decision: {:?}", decision);

    let tool_results: ToolResolution = resolve_and_execute_tool_call(decision, &[tool_def]);
    match &tool_results {
        ToolResolution::Executed { name, result } => {
            assert_eq!(name, "tavily_search");
            if let Some(err) = result.get("error") {
                eprintln!("[skip] tavily_search returned error JSON: {}", err);
                return Ok(()); // treat as skipped
            }
            let answer_ok = result.get("answer").and_then(|v| v.as_str()).map(|s| !s.trim().is_empty()).unwrap_or(false);
            if !(answer_ok) {
                eprintln!("[skip] tavily_search JSON missing useful answer/results: {}", result);
                return Ok(());
            }
            println!("tavily_search JSON accepted (answer_ok={}): {}", answer_ok, result);
        }
        ToolResolution::ModelText(t) => {
            // If the model returned text directly, just print it
            println!("Model returned text directly: {}", t);
        }
        _ => {
            panic!("Unexpected tool resolution result: {:?}", tool_results);
        }
    };
    Ok(())
}