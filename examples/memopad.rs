use color_eyre::Result;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

use ratatui::{DefaultTerminal, Frame, widgets::{Paragraph, Block, Borders}, layout::{Constraint, Direction, Layout}, style::{Modifier}, text::{Span, Line}};

fn main() -> Result<()> {
    color_eyre::install()?;

    let terminal = ratatui::init();
    let result = run(terminal);
    ratatui::restore();
    result
}

fn run(mut terminal: DefaultTerminal) -> Result<()> {
    // 編集バッファとカーソル位置（バイト単位ではなく文字インデックス）
    let mut buffer = String::new();
    let mut cursor_pos: usize = 0;

    loop {
        terminal.draw(|f| render(f, &buffer, cursor_pos))?;

        // 入力待ち（ブロッキング）
        if let Event::Key(key_event) = event::read()? {
            match key_event {
                // 終了: Esc または Ctrl-C
                KeyEvent {
                    code: KeyCode::Esc,
                    ..
                }
                | KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                    ..
                } => {
                    break Ok(());
                }

                // 左移動
                KeyEvent {
                    code: KeyCode::Left,
                    ..
                } => {
                    if cursor_pos > 0 {
                        cursor_pos -= 1;
                    }
                }

                // 右移動
                KeyEvent {
                    code: KeyCode::Right,
                    ..
                } => {
                    if cursor_pos < buffer.chars().count() {
                        cursor_pos += 1;
                    }
                }

                // バックスペース
                KeyEvent {
                    code: KeyCode::Backspace,
                    ..
                } => {
                    if cursor_pos > 0 {
                        // char 単位で前を削除
                        let mut chars: Vec<char> = buffer.chars().collect();
                        chars.remove(cursor_pos - 1);
                        buffer = chars.iter().collect();
                        cursor_pos -= 1;
                    }
                }

                // Delete（カーソル位置の文字削除）
                KeyEvent {
                    code: KeyCode::Delete,
                    ..
                } => {
                    if cursor_pos < buffer.chars().count() {
                        let mut chars: Vec<char> = buffer.chars().collect();
                        chars.remove(cursor_pos);
                        buffer = chars.iter().collect();
                    }
                }

                // 文字挿入（Printable）
                KeyEvent {
                    code: KeyCode::Char(c),
                    modifiers,
                    ..
                } => {
                    // 修飾キー付きの制御は除外（例: Ctrl+something は除く）
                    if modifiers.is_empty() || modifiers == KeyModifiers::SHIFT {
                        let mut chars: Vec<char> = buffer.chars().collect();
                        chars.insert(cursor_pos, c);
                        buffer = chars.iter().collect();
                        cursor_pos += 1;
                    }
                }

                // Home / End
                KeyEvent {
                    code: KeyCode::Home,
                    ..
                } => {
                    cursor_pos = 0;
                }
                KeyEvent {
                    code: KeyCode::End,
                    ..
                } => {
                    cursor_pos = buffer.chars().count();
                }

                _ => {}
            }
        }
    }
}

fn render(f: &mut Frame, buffer: &str, cursor_pos: usize) {
    let size = f.area();

    // 上下に余白を取る簡易レイアウト
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(
            [
                Constraint::Length(3), // 説明エリア
                Constraint::Length(3), // 編集エリア
                Constraint::Min(0),
            ]
            .as_ref(),
        )
        .split(size);

    // 説明
    let help = Paragraph::new(vec![
        Line::from(vec![
            Span::raw("ESC or Ctrl-C to quit. "),
            Span::raw("←/→ to move. "),
            Span::raw("Backspace/Delete to delete. "),
        ]),
    ])
    .block(Block::default().borders(Borders::ALL).title("Help"));
    f.render_widget(help, chunks[0]);

    // 編集対象（単一行）を表示。カーソル位置を擬似表示。
    // カーソルは文字の間にあるので、表示用に下線を入れる形で強調する。
    let mut line: Vec<Span> = Vec::new();
    for (i, ch) in buffer.chars().enumerate() {
        if i == cursor_pos {
            // カーソル位置の前に挿入ポイントとして逆色な空白
            line.push(Span::styled(
                "",
                Modifier::REVERSED, // ここでは空だが、実質カーソル位置を強調するための枠
            ));
        }
        line.push(Span::raw(ch.to_string()));
    }
    if cursor_pos == buffer.chars().count() {
        // 末尾にカーソルがある場合の表示（空の逆色表示）
        line.push(Span::styled(" ", Modifier::REVERSED));
    }

    let editor = Paragraph::new(Line::from(line))
        .block(Block::default().borders(Borders::ALL).title("Edit"));
    f.render_widget(editor, chunks[1]);
}
