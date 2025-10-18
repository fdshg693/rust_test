//! OpenAI対話モード

use super::{AppMode, Mode};
use crate::core::{OpenAIConfig, openai};
use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::Stylize,
    style::Color,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Instant;
use tracing::info;

/// OpenAI対話モード状態
pub struct OpenAIChatMode {
    /// 現在の入力テキスト
    pub input: String,
    /// 最後に送信されたテキスト
    pub last_submitted: String,
    /// AI回答（受信済みの場合）
    pub ai_answer: Option<String>,
    /// AI処理中フラグ
    pub pending: bool,
    /// モード開始時刻
    pub started: Instant,
    /// プロンプト送信用チャンネル
    pub tx: Sender<String>,
    /// AI回答受信用チャンネル
    pub rx: Receiver<String>,
}

impl OpenAIChatMode {
    /// 新しいOpenAI対話モードインスタンスを作成
    pub fn new() -> Self {
        Self::with_config(OpenAIConfig::new())
    }

    /// 設定を指定してOpenAI対話モードインスタンスを作成
    pub fn with_config(config: OpenAIConfig) -> Self {
        // プロンプト送信用チャンネル
        let (tx_prompt, rx_prompt) = mpsc::channel::<String>();
        // AI回答受信用チャンネル
        let (tx_answer, rx_answer) = mpsc::channel::<String>();

        // OpenAI APIワーカーをバックグラウンドで開始
        openai::start_openai_worker(rx_prompt, tx_answer, config);

        Self {
            input: String::new(),
            last_submitted: String::from("(まだありません)"),
            ai_answer: None,
            pending: false,
            started: Instant::now(),
            tx: tx_prompt,
            rx: rx_answer,
        }
    }

    /// 入力テキストをクリア
    pub fn clear_input(&mut self) {
        self.input.clear();
    }

    /// 入力テキストに文字を追加
    pub fn push_char(&mut self, ch: char) {
        self.input.push(ch);
    }

    /// 入力テキストから最後の文字を削除
    pub fn pop_char(&mut self) {
        self.input.pop();
    }

    /// プロンプトを送信
    pub fn submit_prompt(&mut self) -> Result<()> {
        if !self.input.is_empty() && !self.pending {
            self.last_submitted = self.input.clone();
            let to_send = self.input.clone();
            self.clear_input();
            self.ai_answer = None;
            self.pending = true;
            info!(target: "app", "submit_prompt: {}", self.last_submitted);
            self.tx.send(to_send)?;
        }
        Ok(())
    }

    /// AI回答をチェックして更新
    pub fn check_ai_response(&mut self) {
        if let Ok(answer) = self.rx.try_recv() {
            self.ai_answer = Some(answer);
            self.pending = false;
            if let Some(ans) = &self.ai_answer {
                info!(target: "app", "ai_answer_received: {}", ans);
            }
        }
    }

    /// モード開始からの経過時間を取得
    pub fn elapsed_time(&self) -> std::time::Duration {
        self.started.elapsed()
    }
}

impl Default for OpenAIChatMode {
    fn default() -> Self {
        Self::new()
    }
}

impl Mode for OpenAIChatMode {
    fn update(&mut self) {
        // AI回答の非ブロッキングチェック
        self.check_ai_response();
    }

    fn render(&self, f: &mut Frame) {
        let area = f.area();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),  // ヘッダ
                Constraint::Length(3),  // 入力欄
                Constraint::Length(3),  // 直近送信
                Constraint::Length(30), // AI回答
                Constraint::Min(0),     // 余白
            ])
            .split(area);

        self.render_header(f, chunks[0]);
        self.render_input(f, chunks[1]);
        self.render_last_submitted(f, chunks[2]);
        self.render_ai_response(f, chunks[3]);
        self.render_footer(f, chunks[4]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<Option<AppMode>> {
        match key.code {
            KeyCode::Esc => Ok(Some(AppMode::Menu(super::MenuMode::new()))),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Ok(Some(AppMode::Exit))
            }
            KeyCode::Enter => {
                self.submit_prompt()?;
                Ok(None)
            }
            KeyCode::Backspace => {
                self.pop_char();
                Ok(None)
            }
            KeyCode::Char(ch) => {
                self.push_char(ch);
                Ok(None)
            }
            _ => Ok(None),
        }
    }
}

// ===== Private render methods =====

impl OpenAIChatMode {
    /// ヘッダー/ガイド部分を描画
    fn render_header(&self, f: &mut Frame, area: Rect) {
        let guide = vec![
            Line::from("OpenAI Chat Mode".bold().fg(Color::Cyan)),
            Line::from("文字をタイプ → Enter で確定 / Esc でメニューに戻る"),
            Line::from("Backspace で削除 / Ctrl+C で終了"),
        ];
        let guide_widget = Paragraph::new(guide)
            .block(Block::default().borders(Borders::ALL).title("Guide"));
        f.render_widget(guide_widget, area);
    }

    /// 入力欄を描画
    fn render_input(&self, f: &mut Frame, area: Rect) {
        let mut current = self.input.clone();
        current.push('_'); // 簡易カーソル表示
        let input_widget = Paragraph::new(current)
            .block(Block::default().borders(Borders::ALL).title("Input"));
        f.render_widget(input_widget, area);
    }

    /// 最後に送信されたテキストを描画
    fn render_last_submitted(&self, f: &mut Frame, area: Rect) {
        let submitted_widget = Paragraph::new(self.last_submitted.clone())
            .block(Block::default().borders(Borders::ALL).title("Last Submitted"));
        f.render_widget(submitted_widget, area);
    }

    /// AI回答部分を描画
    fn render_ai_response(&self, f: &mut Frame, area: Rect) {
        let status = if self.pending {
            "問い合わせ中...".to_string()
        } else if let Some(ans) = &self.ai_answer {
            ans.clone()
        } else {
            "(まだ回答はありません)".to_string()
        };
        let ai_widget = Paragraph::new(status)
            .block(Block::default().borders(Borders::ALL).title("AI Answer"));
        f.render_widget(ai_widget, area);
    }

    /// フッター部分を描画
    fn render_footer(&self, f: &mut Frame, area: Rect) {
        let elapsed = self.started.elapsed().as_secs_f32();
        let footer = Paragraph::new(Line::from(vec![Span::raw(format!(
            "経過: {elapsed:.1}s"
        ))]))
        .alignment(Alignment::Right);
        f.render_widget(footer, area);
    }
}
