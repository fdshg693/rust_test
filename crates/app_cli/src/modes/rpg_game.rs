//! RPGゲームモード

use super::{AppMode, Mode};
use crate::core::rpg::{Game, Command};
use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::Stylize,
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use tracing::info;

/// RPGゲームモード状態
pub struct RpgGameMode {
    /// RPGゲームインスタンス
    pub game: Game,
    /// 入力中のコマンド（1文字）
    pub input_buffer: String,
    /// メッセージログ（最新5件を保持）
    pub messages: Vec<String>,
    /// ゲーム開始時のメッセージ
    pub is_initialized: bool,
}

impl RpgGameMode {
    /// 新しいRPGゲームモードインスタンスを作成
    pub fn new() -> Self {
        let game = Game::new();
        let mut mode = Self {
            game,
            input_buffer: String::new(),
            messages: Vec::new(),
            is_initialized: false,
        };
        mode.add_message("RPG Game started! Choose your action: [A]ttack, [H]eal, [R]un, [Q]uit or press ESC to return to menu".to_string());
        mode.is_initialized = true;
        mode
    }

    /// メッセージログに追加（最新5件を保持）
    fn add_message(&mut self, msg: String) {
        self.messages.push(msg);
        if self.messages.len() > 5 {
            self.messages.remove(0);
        }
    }

    /// コマンド文字を処理
    fn parse_command(&self, input: &str) -> Option<Command> {
        let c = input.to_ascii_lowercase();
        match c.as_str() {
            "a" => Some(Command::Attack),
            "h" => Some(Command::Heal),
            "r" => Some(Command::Run),
            "q" => Some(Command::Quit),
            _ => None,
        }
    }
}

impl Mode for RpgGameMode {
    fn update(&mut self) {
        // フレーム毎の更新処理（現在は特に必要なし）
    }

    fn render(&self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(8),  // ゲーム状態
                Constraint::Min(6),     // メッセージログ
                Constraint::Length(3),  // ユーザーメッセージ
            ])
            .split(f.area());

        // ゲーム状態パネル
        render_status(f, chunks[0], &self.game);

        // メッセージログパネル
        render_messages(f, chunks[1], &self.messages);

        // 入力パネル
        render_input(f, chunks[2], &self.input_buffer);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<Option<AppMode>> {
        match key.code {
            KeyCode::Esc => {
                info!(target: "app", "RPG mode: returning to menu");
                return Ok(Some(AppMode::Menu(super::menu::MenuMode::new())));
            }
            KeyCode::Char(c) if key.modifiers.is_empty() => {
                // コマンド入力
                self.input_buffer.clear();
                self.input_buffer.push(c);

                if let Some(cmd) = self.parse_command(&self.input_buffer) {
                    // コマンドを実行
                    match cmd {
                        Command::Quit => {
                            self.add_message("You quit the game.".to_string());
                            info!(target: "app", "RPG mode: quit game");
                            // 少し遅延させてメニューに戻る（メッセージを表示するため）
                            return Ok(Some(AppMode::Menu(super::menu::MenuMode::new())));
                        }
                        Command::Attack => {
                            let p = self.game.player();
                            let e = self.game.enemy();
                            self.add_message(format!("{} attacks {}!", p.name, e.name));
                        }
                        Command::Heal => {
                            let p = self.game.player();
                            self.add_message(format!("{} uses a potion!", p.name));
                        }
                        Command::Run => {
                            let p = self.game.player();
                            self.add_message(format!("{} tries to run away!", p.name));
                        }
                    }

                    // ゲーム処理を実行
                    match self.game.handle_command(cmd) {
                        Ok(true) => {
                            // ゲーム続行
                            let snapshot = self.game.snapshot();
                            if snapshot.is_over {
                                self.add_message("Game Over!".to_string());
                                info!(target: "app", "RPG mode: game over");
                                return Ok(Some(AppMode::Menu(super::menu::MenuMode::new())));
                            }
                        }
                        Ok(false) => {
                            // ゲーム終了
                            self.add_message("Game ended.".to_string());
                            info!(target: "app", "RPG mode: game ended");
                            return Ok(Some(AppMode::Menu(super::menu::MenuMode::new())));
                        }
                        Err(e) => {
                            self.add_message(format!("Error: {}", e));
                            info!(target: "app", error = %e, "RPG mode: command error");
                        }
                    }
                }
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
            }
            _ => {}
        }

        Ok(None)
    }
}

/// ゲーム状態を描画
fn render_status(f: &mut Frame, area: Rect, game: &Game) {
    let p = game.player();
    let e = game.enemy();
    let turn_text = format!("{:?}", game.turn());

    let block = Block::default()
        .title(" Battle Status ")
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded);

    let status_lines = vec![
        Line::from(format!("Battle #{}", game.battle_count())),
        Line::from(format!(
            "Player: {} | HP: {}/{} | ATK: {} | Potions: {} | Gold: {}",
            p.name, p.hp, p.max_hp, p.atk, p.potions, p.gold
        )),
        Line::from(format!("Enemy:  {} | HP: {} | ATK: {}", e.name, e.hp, e.atk)),
        Line::from(format!("Turn: {}", turn_text).cyan()),
    ];

    let paragraph = Paragraph::new(status_lines)
        .block(block)
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}

/// メッセージログを描画
fn render_messages(f: &mut Frame, area: Rect, messages: &[String]) {
    let block = Block::default()
        .title(" Battle Log ")
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded);

    let message_lines: Vec<Line> = messages
        .iter()
        .map(|msg| Line::from(msg.clone()).yellow())
        .collect();

    let paragraph = Paragraph::new(message_lines)
        .block(block)
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}

/// 入力パネルを描画
fn render_input(f: &mut Frame, area: Rect, input_buffer: &str) {
    let block = Block::default()
        .title(" Command ")
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded);

    let input_text = if input_buffer.is_empty() {
        "[A]ttack, [H]eal, [R]un, [Q]uit - Press ESC to return to menu".to_string()
    } else {
        format!("Input: {} (press Enter or wait)", input_buffer.green())
    };

    let paragraph = Paragraph::new(input_text)
        .block(block)
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}
