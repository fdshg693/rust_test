use color_eyre::Result;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use tracing_appender::rolling;

fn main() -> Result<()> {
    color_eyre::install()?;

    // Load .env (optional). This allows reading OPENAI_API_KEY from a local .env file.
    // If the file doesn't exist, ignore the error.
    let _ = dotenvy::dotenv();

    // ログ: 標準出力は使わず、ファイルへのみ出力してratatuiと衝突しないようにする
    let file_appender = rolling::daily("logs", "app.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    // _guardはdropするとログが失われるため、スコープ終了まで保持
    // メイン関数のライフタイム中保持するため、わざと未使用変数にする
    let guard = _guard; // keep alive

    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    let file_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false) // ファイルにANSIカラー不要
        .with_target(false);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .init();

    // guard を使うことでdropされないようにする
    let _keep_guard = guard;
    let terminal = ratatui::init();
    let res = rust_test::run(terminal);
    ratatui::restore();
    res
}
