//! メニューモード: 起動時の選択画面

use super::{AppMode, Mode};
use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};
use std::fmt;

/// メニューの選択肢
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuItem {
    OpenAIChat,
    RpgGame,
    Exit,
}

impl fmt::Display for MenuItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MenuItem::OpenAIChat => write!(f, "OpenAI Chat"),
            MenuItem::RpgGame => write!(f, "RPG Game"),
            MenuItem::Exit => write!(f, "Exit"),
        }
    }
}

impl MenuItem {
    fn all() -> [MenuItem; 3] {
        [MenuItem::OpenAIChat, MenuItem::RpgGame, MenuItem::Exit]
    }

    fn next(self) -> MenuItem {
        match self {
            MenuItem::OpenAIChat => MenuItem::RpgGame,
            MenuItem::RpgGame => MenuItem::Exit,
            MenuItem::Exit => MenuItem::OpenAIChat,
        }
    }

    fn prev(self) -> MenuItem {
        match self {
            MenuItem::OpenAIChat => MenuItem::Exit,
            MenuItem::RpgGame => MenuItem::OpenAIChat,
            MenuItem::Exit => MenuItem::RpgGame,
        }
    }
}

/// メニューモード状態
pub struct MenuMode {
    selected: MenuItem,
}

impl MenuMode {
    pub fn new() -> Self {
        Self {
            selected: MenuItem::OpenAIChat,
        }
    }
}

impl Default for MenuMode {
    fn default() -> Self {
        Self::new()
    }
}

impl Mode for MenuMode {
    fn update(&mut self) {
        // メニューには定期更新は不要
    }

    fn render(&self, f: &mut Frame) {
        let area = f.area();

        // メインパネルの作成
        let block = Block::default()
            .title("Rust TUI App - Mode Selection")
            .borders(Borders::ALL);

        f.render_widget(block, area);

        // コンテンツエリア（パディング考慮）
        let content_area = Rect {
            x: area.x + 2,
            y: area.y + 2,
            width: area.width.saturating_sub(4),
            height: area.height.saturating_sub(4),
        };

        // タイトル
        let title = Paragraph::new("Select Mode")
            .style(Style::default().fg(Color::Cyan).bold());
        f.render_widget(title, Rect {
            x: content_area.x,
            y: content_area.y,
            width: content_area.width,
            height: 2,
        });

        // メニュー項目の表示
        let menu_start_y = content_area.y + 3;
        let items = MenuItem::all();

        for (index, item) in items.iter().enumerate() {
            let is_selected = self.selected == *item;
            let prefix = if is_selected { "▶ " } else { "  " };

            let text = format!("{}{}", prefix, item);
            let style = if is_selected {
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Blue)
                    .bold()
            } else {
                Style::default().fg(Color::White)
            };

            let paragraph = Paragraph::new(text).style(style);
            let item_area = Rect {
                x: content_area.x,
                y: menu_start_y + index as u16,
                width: content_area.width,
                height: 1,
            };
            f.render_widget(paragraph, item_area);
        }

        // フッター（操作説明）
        let footer_y = area.height.saturating_sub(2);
        let footer_text = "↑/↓: Navigate | Enter: Select | Esc/q: Exit";
        let footer = Paragraph::new(footer_text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(footer, Rect {
            x: area.x,
            y: footer_y,
            width: area.width,
            height: 1,
        });
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<Option<AppMode>> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = self.selected.prev();
                Ok(None)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.selected = self.selected.next();
                Ok(None)
            }
            KeyCode::Enter => {
                let next_mode = match self.selected {
                    MenuItem::OpenAIChat => {
                        AppMode::OpenAIChat(super::OpenAIChatMode::new())
                    }
                    MenuItem::RpgGame => {
                        AppMode::RpgGame(super::RpgGameMode::new())
                    }
                    MenuItem::Exit => AppMode::Exit,
                };
                Ok(Some(next_mode))
            }
            KeyCode::Esc | KeyCode::Char('q') => Ok(Some(AppMode::Exit)),
            _ => Ok(None),
        }
    }
}
