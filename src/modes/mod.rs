//! モードシステム: 複数のUIモード（メニュー、OpenAI対話、RPGゲーム）を管理

use color_eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::Frame;

/// 各モードが実装すべきトレイト
pub trait Mode {
    /// フレーム毎の非ブロッキング更新処理（AIレスポンスチェックなど）
    fn update(&mut self);

    /// 画面描画
    fn render(&self, f: &mut Frame);

    /// キーイベント処理
    /// 戻り値: Some(次のモード) でモード遷移、None で同じモード継続
    fn handle_key(&mut self, key: KeyEvent) -> Result<Option<AppMode>>;
}

/// アプリケーションが取り得るモードの列挙型
pub enum AppMode {
    Menu(MenuMode),
    // OpenAIChat(OpenAIChatMode),  // Phase 2で追加
    // RpgGame(RpgGameMode),         // Phase 3で追加
    Exit,
}

impl AppMode {
    /// 現在のモードで update() を呼び出す
    pub fn update(&mut self) {
        match self {
            AppMode::Menu(m) => m.update(),
            AppMode::Exit => {}
        }
    }

    /// 現在のモードで render() を呼び出す
    pub fn render(&self, f: &mut Frame) {
        match self {
            AppMode::Menu(m) => m.render(f),
            AppMode::Exit => {}
        }
    }

    /// 現在のモードで handle_key() を呼び出す
    pub fn handle_key(&mut self, key: KeyEvent) -> Result<Option<AppMode>> {
        match self {
            AppMode::Menu(m) => m.handle_key(key),
            AppMode::Exit => Ok(None),
        }
    }
}

pub mod menu;

pub use menu::MenuMode;
