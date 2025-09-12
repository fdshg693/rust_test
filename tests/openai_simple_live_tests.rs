use rust_test::config::Config;
use rust_test::openai::get_ai_answer_once_blocking;

// Load .env before tests in this integration test binary
#[ctor::ctor]
fn _load_dotenv() { let _ = dotenvy::dotenv(); }

/// Live test that actually calls OpenAI. Ignored by default.
/// Run with: set OPENAI_API_KEY first, then `cargo test -- --ignored`
#[test]
#[ignore]
fn live_get_ai_answer_once_blocking() -> Result<(), Box<dyn std::error::Error>> {
    // Only run when OPENAI_API_KEY is available
    if std::env::var("OPENAI_API_KEY").is_err() {
        eprintln!("[skip] OPENAI_API_KEY not set; skipping live OpenAI test");
        return Ok(());
    }

    let cfg = Config::new();
    let prompt = "1+1は？ 短く答えて。";
    let ans = get_ai_answer_once_blocking(prompt, &cfg)?;
    println!("Live response: {}", ans);
    assert!(!ans.trim().is_empty(), "expected non-empty response");
    Ok(())
}
