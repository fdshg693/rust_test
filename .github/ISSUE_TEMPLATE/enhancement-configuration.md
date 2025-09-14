---
name: ⚙️ 設定管理とカスタマイゼーション機能の充実
about: 柔軟な設定システムとユーザーカスタマイゼーション機能の実装
title: '[Enhancement] 設定管理とカスタマイゼーション機能の充実'
labels: enhancement, configuration, customization
assignees: ''
---

## 概要

現在の設定管理は`src/config.rs`でハードコードされた値のみを提供しており、ユーザーが実行時に設定を変更する機能がありません。プロダクション使用やカスタマイゼーションのニーズに対応するため、柔軟な設定管理システムが必要です。

## 現在の問題

1. **設定の固定化**: すべての設定がコンパイル時に決定される
   ```rust
   impl Default for Config {
       fn default() -> Self {
           Self {
               model: "gpt-4o-mini".to_string(),
               max_tokens: 2000,
               poll_interval_ms: 100,
           }
       }
   }
   ```

2. **設定ファイルの不在**: 永続的な設定保存機能がない

3. **ランタイム設定変更の不可**: アプリケーション実行中の設定変更ができない

4. **UIテーマ・カスタマイゼーション不足**: 色やレイアウトの個人設定ができない

5. **キーバインドの固定**: ユーザーが独自のキーマッピングを設定できない

## 提案する改善

### 1. 階層的設定システム

```rust
// src/config.rs での改善例
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub openai: OpenAIConfig,
    pub ui: UIConfig,
    pub behavior: BehaviorConfig,
    pub keybindings: KeyBindings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIConfig {
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
    pub timeout_seconds: u64,
    pub retry_attempts: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIConfig {
    pub theme: Theme,
    pub poll_interval_ms: u64,
    pub auto_scroll: bool,
    pub show_timestamps: bool,
    pub compact_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub primary_color: String,
    pub secondary_color: String,
    pub error_color: String,
    pub success_color: String,
    pub background_color: String,
    pub highlight_color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBindings {
    pub quit: String,
    pub submit: String,
    pub clear: String,
    pub scroll_up: String,
    pub scroll_down: String,
    pub toggle_help: String,
    pub toggle_settings: String,
}
```

### 2. 設定ファイルの読み込み・保存

```rust
impl Config {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
    
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        // 設定ディレクトリが存在しない場合は作成
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
    
    pub fn load_with_fallback() -> Self {
        // 1. 環境変数から読み込み
        // 2. 設定ファイルから読み込み
        // 3. デフォルト値を使用
        let config_path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rust_test")
            .join("config.toml");
            
        Self::load_from_file(&config_path)
            .unwrap_or_else(|_| {
                let default_config = Self::default();
                // デフォルト設定を保存
                let _ = default_config.save_to_file(&config_path);
                default_config
            })
    }
}
```

### 3. ランタイム設定UI

```rust
// src/ui/settings.rs での新機能
use ratatui::{
    widgets::{Tabs, List, ListItem, Block, Borders, Paragraph},
    layout::{Layout, Direction, Constraint, Rect},
    style::{Style, Color, Modifier},
};

pub struct SettingsPanel {
    pub active: bool,
    pub current_tab: SettingsTab,
    pub selected_item: usize,
    pub temp_config: Config,
    pub modified: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum SettingsTab {
    OpenAI = 0,
    UI = 1,
    KeyBindings = 2,
    Advanced = 3,
}

impl SettingsPanel {
    pub fn new(config: &Config) -> Self {
        Self {
            active: false,
            current_tab: SettingsTab::OpenAI,
            selected_item: 0,
            temp_config: config.clone(),
            modified: false,
        }
    }
    
    pub fn render(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // タブ
                Constraint::Min(0),    // コンテンツ
                Constraint::Length(3), // ヘルプ
            ])
            .split(area);
        
        // タブの描画
        let tabs = vec!["OpenAI", "UI", "Keys", "Advanced"];
        let tab_widget = Tabs::new(tabs)
            .block(Block::default().borders(Borders::ALL).title("設定"))
            .select(self.current_tab as usize)
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD));
            
        f.render_widget(tab_widget, chunks[0]);
        
        // コンテンツの描画
        match self.current_tab {
            SettingsTab::OpenAI => self.render_openai_settings(f, chunks[1]),
            SettingsTab::UI => self.render_ui_settings(f, chunks[1]),
            SettingsTab::KeyBindings => self.render_keybinding_settings(f, chunks[1]),
            SettingsTab::Advanced => self.render_advanced_settings(f, chunks[1]),
        }
        
        // ヘルプの描画
        let help_text = if self.modified {
            "↑↓: 選択 | Enter: 編集 | S: 保存 | R: リセット | ESC: キャンセル"
        } else {
            "↑↓: 選択 | Enter: 編集 | ESC: 閉じる"
        };
        
        let help = Paragraph::new(help_text)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(help, chunks[2]);
    }
    
    fn render_openai_settings(&self, f: &mut Frame, area: Rect) {
        let items = vec![
            ListItem::new(format!("Model: {}", self.temp_config.openai.model)),
            ListItem::new(format!("Max Tokens: {}", self.temp_config.openai.max_tokens)),
            ListItem::new(format!("Temperature: {:.1}", self.temp_config.openai.temperature)),
            ListItem::new(format!("Timeout: {}s", self.temp_config.openai.timeout_seconds)),
        ];
        
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("OpenAI 設定"))
            .highlight_style(Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD));
                
        f.render_stateful_widget(list, area, &mut ListState::default());
    }
}
```

### 4. 設定ファイルの例

```toml
# ~/.config/rust_test/config.toml
[openai]
model = "gpt-4o-mini"
max_tokens = 2000
temperature = 0.7
timeout_seconds = 30
retry_attempts = 3

[ui]
poll_interval_ms = 100
auto_scroll = true
show_timestamps = false
compact_mode = false

[ui.theme]
primary_color = "cyan"
secondary_color = "gray"
error_color = "red"
success_color = "green"
background_color = "black"
highlight_color = "yellow"

[behavior]
auto_save_history = true
max_history_items = 100
confirm_quit = true
save_on_exit = true

[keybindings]
quit = "Ctrl+C"
submit = "Enter"
clear = "Ctrl+L"
scroll_up = "PageUp"
scroll_down = "PageDown"
toggle_help = "F1"
toggle_settings = "F2"
save_config = "Ctrl+S"
```

## 実装計画

### Phase 1: 基本設定システム (Week 1)
- [ ] 設定構造体の拡張
- [ ] TOML設定ファイルの読み込み・保存
- [ ] 環境変数オーバーライド
- [ ] デフォルト設定ファイルの生成

### Phase 2: ランタイム設定変更 (Week 2)
- [ ] 設定パネルUIの実装
- [ ] キーバインド処理の動的化
- [ ] 設定の即時反映機能
- [ ] 設定検証とエラーハンドリング

### Phase 3: 高度なカスタマイゼーション (Week 3)
- [ ] テーマシステムの実装
- [ ] 設定のインポート・エクスポート
- [ ] 設定リセット機能
- [ ] 設定プリセット機能

## 期待される効果

1. **ユーザビリティの向上**: 個人の好みに合わせたカスタマイゼーション
2. **運用性の改善**: 環境に応じた設定の調整が容易
3. **アクセシビリティの向上**: キーバインドやテーマの個人設定
4. **プロダクション対応**: 企業環境での設定管理

## 実装優先度

**Medium** - ユーザー体験の向上に寄与するが、現在の機能に支障はない

## 関連ファイル

- `src/config.rs` - 設定構造体とロジック
- `src/ui/settings.rs` - 設定パネルUI（新規）
- `src/event.rs` - 設定変更イベント処理
- `src/app.rs` - 設定状態管理
- `Cargo.toml` - 依存関係の追加

## 依存関係の追加

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"
dirs = "5.0"
```

## テスト計画

- [ ] 設定ファイルの読み込み・保存テスト
- [ ] 不正な設定値のハンドリングテスト
- [ ] デフォルト値フォールバックのテスト
- [ ] 設定UIの操作テスト
- [ ] 環境変数オーバーライドのテスト
- [ ] 設定の永続化テスト

## 受け入れ基準

- 設定ファイルが正常に読み込み・保存される
- 実行時に設定変更できる
- 不正な設定値が適切にハンドリングされる
- デフォルト設定が自動生成される
- 設定変更が即座に反映される
- キーバインドが動的に変更される

## ドキュメント更新

- [ ] 設定ファイルの形式をREADMEに追加
- [ ] 環境変数の一覧を文書化
- [ ] 設定UI操作方法のドキュメント作成