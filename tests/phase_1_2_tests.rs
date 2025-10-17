// Phase 1 & 2 テスト: メニューとOpenAIChatモード
#![allow(dead_code)]

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use rust_test::modes::{AppMode, MenuMode, OpenAIChatMode, Mode};

#[test]
fn test_menu_navigation() {
    let mut menu = MenuMode::new();
    
    // 初期状態は "OpenAI Chat" が選択されている
    // (内部状態なので直接テストできないが、以下の操作でエラーが出なければOK)
    
    // Down キーを押す
    let key_down = KeyEvent::new(KeyCode::Down, KeyModifiers::empty());
    let result = menu.handle_key(key_down);
    assert!(result.is_ok(), "Down key should be handled");
    
    // Up キーを押す
    let key_up = KeyEvent::new(KeyCode::Up, KeyModifiers::empty());
    let result = menu.handle_key(key_up);
    assert!(result.is_ok(), "Up key should be handled");
    
    // Esc キーを押す（終了）
    let key_esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
    let result = menu.handle_key(key_esc);
    assert!(result.is_ok(), "Esc key should be handled");
    if let Ok(Some(AppMode::Exit)) = result {
        // 期待通り
    } else {
        panic!("Esc should result in Exit mode");
    }
}

#[test]
fn test_openai_chat_mode_creation() {
    let chat = OpenAIChatMode::new();
    
    // 初期状態の確認
    assert_eq!(chat.input, "");
    assert_eq!(chat.last_submitted, "(まだありません)");
    assert!(chat.ai_answer.is_none());
    assert!(!chat.pending);
}

#[test]
fn test_openai_chat_mode_input() {
    let mut chat = OpenAIChatMode::new();
    
    // 文字入力
    let key_a = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty());
    let result = chat.handle_key(key_a);
    assert!(result.is_ok());
    assert_eq!(chat.input, "a");
    
    // 文字追加
    let key_b = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::empty());
    chat.handle_key(key_b).ok();
    assert_eq!(chat.input, "ab");
    
    // バックスペース
    let key_backspace = KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty());
    chat.handle_key(key_backspace).ok();
    assert_eq!(chat.input, "a");
}

#[test]
fn test_openai_chat_mode_esc_returns_to_menu() {
    let mut chat = OpenAIChatMode::new();
    
    let key_esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
    let result = chat.handle_key(key_esc);
    
    assert!(result.is_ok());
    if let Ok(Some(AppMode::Menu(_))) = result {
        // 期待通り: Esc でメニューに戻る
    } else {
        panic!("Esc should return to Menu mode");
    }
}

#[test]
fn test_app_mode_enum_dispatch() {
    let mut mode = AppMode::Menu(MenuMode::new());
    
    // update() が呼べることを確認
    mode.update();
    
    // handle_key() が呼べることを確認
    let key_esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
    let result = mode.handle_key(key_esc);
    assert!(result.is_ok());
}
