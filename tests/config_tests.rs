use rust_test::config::{Config, X, Y};

// Load .env before tests in this integration test binary
#[ctor::ctor]
fn _load_dotenv() { let _ = dotenvy::dotenv(); }

#[test]
fn config_defaults() {
    let c = Config::new();
    assert_eq!(c.model, "gpt-4o-mini");
    assert_eq!(c.max_tokens, 512);
    assert_eq!(c.poll_interval_ms, 100);
}

#[test]
fn constants_values() {
    assert_eq!(X, 42);
    assert_eq!(Y, 7);
}
