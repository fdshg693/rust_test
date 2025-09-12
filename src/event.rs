//! イベント処理モジュール

use crate::app::App;
use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// キーイベントを処理
/// 
/// # Returns
/// - `Ok(true)` - アプリケーションを終了
/// - `Ok(false)` - 処理を継続
/// - `Err(_)` - エラーが発生
pub fn handle_key(app: &mut App, key: KeyEvent) -> Result<bool> {
    match key.code {
        KeyCode::Esc => return Ok(true),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return Ok(true),
        KeyCode::Enter => {
            if !app.input.is_empty() && !app.pending {
                app.last_submitted = app.input.clone();
                let to_send = app.input.clone();
                app.input.clear();
                app.ai_answer = None;
                app.pending = true;
                let _ = app.tx.send(to_send); // ワーカーが終了している場合は送信エラーを無視
            }
        }
        KeyCode::Backspace => {
            app.input.pop();
        }
        KeyCode::Char(ch) => {
            // 通常の文字入力
            app.input.push(ch);
        }
        _ => {}
    }
    Ok(false)
}
