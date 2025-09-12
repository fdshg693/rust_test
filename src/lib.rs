
// 同階層のファイルをモジュールとしてインポート
pub mod app;
pub mod config;
pub mod event;
pub mod openai;
pub mod ui;

pub use app::App;
pub use config::Config;

use color_eyre::Result;
use crossterm::event::{self as crossterm_event, Event, KeyEventKind};
use ratatui::DefaultTerminal;
use std::time::Duration;

// Ensure .env is loaded for tests before anything else runs in the test process.
#[cfg(test)]
#[ctor::ctor]
fn load_dotenv_for_tests() {
    let _ = dotenvy::dotenv();
}

/// アプリケーションのメインループを実行
pub fn run(mut terminal: DefaultTerminal) -> Result<()> {
    let mut app = App::new();

    loop {
        // AI回答の非ブロッキングチェック
        app.check_ai_response();
        
        terminal.draw(|f| ui::render(f, &app))?;

        // 100ms以内にイベントが来たら処理
        if crossterm_event::poll(Duration::from_millis(100))? {
            match crossterm_event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if event::handle_key(&mut app, key)? {
                        break; // trueの場合終了
                    }
                }
                Event::Resize(_, _) => {
                    // 次ループで再描画されるので特別な処理なし
                }
                _ => {}
            }
        }
    }
    Ok(())
}
