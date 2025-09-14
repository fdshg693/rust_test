use once_cell::sync::Lazy;
use std::sync::Once;
use tracing_subscriber::{fmt, EnvFilter, prelude::*};
use tracing_appender::rolling;

static START: Once = Once::new();
static _GUARD: Lazy<std::sync::Mutex<Option<tracing_appender::non_blocking::WorkerGuard>>> = Lazy::new(|| std::sync::Mutex::new(None));

/// Initialize test environment: dotenv, color-eyre (optional), and tracing (stdout + file).
/// Idempotent: safe to call multiple times.
pub fn init() {
    START.call_once(|| {
        let _ = dotenvy::dotenv();
        let filter = EnvFilter::try_from_default_env()
            .or_else(|_| EnvFilter::try_new("info"))
            .expect("env filter");

        // Daily rotating log file separate from app runtime logs
        let file_appender = rolling::daily("logs", "tests.log");
        let (file_nb, guard) = tracing_appender::non_blocking(file_appender);
        *_GUARD.lock().unwrap() = Some(guard); // retain guard for lifetime

        // Layer for stderr (pretty)
        let stderr_layer = fmt::layer()
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_writer(std::io::stderr);

        // Layer for file (no ANSI)
        let file_layer = fmt::layer()
            .with_ansi(false)
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_writer(file_nb);

        tracing_subscriber::registry()
            .with(filter)
            .with(stderr_layer)
            .with(file_layer)
            .init();

        tracing::info!(target="test_init", "Test tracing initialized (stderr + rotating file)");
    });
}
