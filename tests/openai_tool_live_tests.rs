use rust_test::config::Config;
use rust_test::openai::{propose_tool_call_blocking, ToolCallDecision, build_get_constants_tool};

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

/// Live test: with no tools provided, model must return a text answer
#[test]
#[ignore]
fn live_tool_call_none_tools_returns_text() -> Result<(), Box<dyn std::error::Error>> {
    if skip_if_no_api_key() { return Ok(()); }

    let cfg = Config::new();
    let prompt = "1+1は？短く答えて。";
    let empty: [rust_test::openai::ToolDefinition; 0] = [];
    let decision = propose_tool_call_blocking(prompt, &empty, &cfg)?;
    println!("Decision: {:?}", decision);

    match decision {
        ToolCallDecision::Text(t) => assert!(!t.trim().is_empty(), "expected non-empty text"),
        ToolCallDecision::ToolCall { .. } => panic!("unexpected ToolCall when no tools were provided"),
    }
    Ok(())
}

/// Live test: provide a simple function tool; model may propose a tool call or reply with text
#[test]
#[ignore]
fn live_tool_call_with_function() -> Result<(), Box<dyn std::error::Error>> {
    if skip_if_no_api_key() { return Ok(()); }

    // Use existing helper to build the tool definition
    let tool_def = build_get_constants_tool(42, 7);

    let cfg = Config::new();
    let prompt = "定数XとYの現在値を取得したいです。必要なら get_constants を使ってください。";
    let decision = propose_tool_call_blocking(prompt, &[tool_def], &cfg)?;
    println!("Decision: {:?}", decision);

    match decision {
        ToolCallDecision::ToolCall { name, arguments } => {
            assert_eq!(name, "get_constants");
            // Arguments may be an empty object or something minimal; just ensure it's valid JSON-ish
            assert!(
                !arguments.trim().is_empty(),
                "expected some arguments JSON (possibly {{}})"
            );
        }
        ToolCallDecision::Text(t) => {
            // Some models may answer directly without proposing a tool; allow it but require non-empty text
            assert!(!t.trim().is_empty(), "expected non-empty text response");
        }
    }
    Ok(())
}
