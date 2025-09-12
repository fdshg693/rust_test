//! UI描画モジュール

use crate::app::App;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::Stylize;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

/// メインUI描画関数
pub fn render(f: &mut Frame, app: &App) {
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

    render_header(f, chunks[0]);
    render_input(f, app, chunks[1]);
    render_last_submitted(f, app, chunks[2]);
    render_ai_response(f, app, chunks[3]);
    render_footer(f, app, chunks[4]);
}

/// ヘッダー/ガイド部分を描画
fn render_header(f: &mut Frame, area: ratatui::layout::Rect) {
    let guide = vec![
        Line::from("Ratatui ECHO デモ".bold()),
        Line::from("文字をタイプ → Enter で確定 / Esc or Ctrl+C で終了"),
        Line::from("Backspace で削除"),
    ];
    let guide_widget = Paragraph::new(guide)
        .block(Block::default().borders(Borders::ALL).title("Guide"));
    f.render_widget(guide_widget, area);
}

/// 入力欄を描画
fn render_input(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let mut current = app.input.clone();
    current.push('_'); // 簡易カーソル表示
    let input_widget = Paragraph::new(current)
        .block(Block::default().borders(Borders::ALL).title("Input"));
    f.render_widget(input_widget, area);
}

/// 最後に送信されたテキストを描画
fn render_last_submitted(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let submitted_widget = Paragraph::new(app.last_submitted.clone())
        .block(Block::default().borders(Borders::ALL).title("Last Submitted"));
    f.render_widget(submitted_widget, area);
}

/// AI回答部分を描画
fn render_ai_response(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let status = if app.pending {
        "問い合わせ中...".to_string()
    } else if let Some(ans) = &app.ai_answer {
        ans.clone()
    } else {
        "(まだ回答はありません)".to_string()
    };
    let ai_widget = Paragraph::new(status)
        .block(Block::default().borders(Borders::ALL).title("AI Answer"));
    f.render_widget(ai_widget, area);
}

/// フッター部分を描画
fn render_footer(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let elapsed = app.started.elapsed().as_secs_f32();
    let footer = Paragraph::new(Line::from(vec![Span::raw(format!(
        "経過: {elapsed:.1}s"
    ))]));
    f.render_widget(footer, area);
}
