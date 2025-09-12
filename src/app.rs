//! アプリケーション状態管理モジュール

use crate::config::Config;
use crate::openai;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Instant;
use tracing::info;

/// アプリケーションの状態を管理する構造体
pub struct App {
    /// 現在の入力テキスト
    pub input: String,
    /// 最後に送信されたテキスト
    pub last_submitted: String,
    /// AI回答（受信済みの場合）
    pub ai_answer: Option<String>,
    /// AI処理中フラグ
    pub pending: bool,
    /// アプリケーション開始時刻
    pub started: Instant,
    /// プロンプト送信用チャンネル
    pub tx: Sender<String>,
    /// AI回答受信用チャンネル
    pub rx: Receiver<String>,
}

impl App {
    /// 新しいアプリケーションインスタンスを作成
    pub fn new() -> Self {
        Self::with_config(Config::new())
    }

    /// 設定を指定してアプリケーションインスタンスを作成
    pub fn with_config(config: Config) -> Self {
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
    pub fn submit_prompt(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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
            if let Some(ans) = &self.ai_answer { info!(target: "app", "ai_answer_received: {}", ans); }
        }
    }

    /// アプリケーション開始からの経過時間を取得
    pub fn elapsed_time(&self) -> std::time::Duration {
        self.started.elapsed()
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
