
// 同階層のファイルをモジュールとしてインポート
pub mod config;
pub mod openai;
pub mod sqlite; // SQLite utilities
pub mod rpg; // Tiny RPG library (rules/models/game/ui) for AI tools
pub mod modes; // Mode system for different UI modes

pub use config::OpenAIConfig;
pub use sqlite::Db;

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
    let mut current_mode = modes::AppMode::Menu(modes::MenuMode::new());

    loop {
        // 現在のモードで更新処理を実行
        current_mode.update();

        // 画面を描画
        terminal.draw(|f| current_mode.render(f))?;

        // 100ms以内にイベントが来たら処理
        if crossterm_event::poll(Duration::from_millis(100))? {
            match crossterm_event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    match current_mode.handle_key(key) {
                        Ok(Some(next_mode)) => {
                            // モード遷移またはExit
                            if matches!(next_mode, modes::AppMode::Exit) {
                                break;
                            }
                            current_mode = next_mode;
                        }
                        Ok(None) => {
                            // 同じモード継続
                        }
                        Err(e) => {
                            // エラーが発生した場合はメニューに戻す
                            tracing::error!("Error in mode: {:?}", e);
                            current_mode = modes::AppMode::Menu(modes::MenuMode::new());
                        }
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
