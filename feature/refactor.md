# UI・ロジック分離とメニューシステム移行プラン

## 現状の問題点

### 1. UIとロジックの密結合
- `src/lib.rs`の`run()`関数が直接`App`を作成し、OpenAIワーカーを起動
- `src/ui.rs`がOpenAI専用のUI構造を持つ（Input/AI Answer/Last Submitted）
- `src/event.rs`のキーハンドラーが`App`構造体に直接依存
- RPGゲームには独自の`rpg::ui`があるが、ratatui統合されていない

### 2. 単一モード起動
- `main.rs` → `lib::run()` → `App::new()` で必ずOpenAIチャットモードが起動
- モード選択の仕組みが存在しない
- 異なる体験（RPGゲーム、OpenAI対話）を切り替える方法がない

### 3. 状態管理の課題
- `App`構造体がOpenAI特化型（`tx`/`rx`チャンネル、`ai_answer`、`pending`）
- 他のモード（RPGなど）を追加する余地がない
- モード間の遷移を管理する仕組みがない

---

## 目標アーキテクチャ

### トップレベル構造
```
main.rs
  ↓
lib.rs::run()
  ↓
AppMode enum {
  Menu,           // 起動時のメニュー画面
  OpenAIChat,     // 既存のOpenAI対話モード
  RpgGame,        // RPGゲームモード
}
  ↓
各モード専用の State + UI + EventHandler
```

### 分離の指針
1. **モード抽象化**: 各モードは独自の状態・UI・イベント処理を持つ
2. **共通インターフェース**: `Mode` traitで統一
3. **疎結合**: モード間は列挙型で切り替え、相互に依存しない

---

## 移行ステップ（段階的リファクタリング）

### Phase 0: 準備と設計確認
**目的**: 既存コードを壊さずに新しい構造を設計

- [x] 現状分析完了
- [ ] `Mode` trait設計案を作成
- [ ] ディレクトリ構造案を確定

**成果物**:
```
src/
  modes/          # 新設
    mod.rs        # Mode trait定義
    menu.rs       # メニューモード
    openai_chat.rs  # 既存OpenAI対話モード（移行先）
    rpg_game.rs   # RPGゲームモード
```

---

### Phase 1: Mode traitとメニュー実装
**目的**: モード切り替えの基盤を作る

#### 1.1 `src/modes/mod.rs` に `Mode` trait定義
```rust
pub trait Mode {
    /// 現在のモードを描画
    fn render(&self, f: &mut Frame);
    
    /// キーイベントを処理
    /// Returns: Some(次のモード) or None(同じモード継続)
    fn handle_key(&mut self, key: KeyEvent) -> Result<Option<AppMode>>;
    
    /// フレーム毎の更新処理（非ブロッキング）
    fn update(&mut self);
}

pub enum AppMode {
    Menu(MenuMode),
    OpenAIChat(OpenAIChatMode),
    RpgGame(RpgGameMode),
    Exit,
}
```

#### 1.2 `src/modes/menu.rs` でメニュー画面実装
```rust
pub struct MenuMode {
    selected_index: usize,
    options: Vec<&'static str>,
}

impl MenuMode {
    pub fn new() -> Self {
        Self {
            selected_index: 0,
            options: vec![
                "OpenAI Chat",
                "RPG Game",
                "Exit",
            ],
        }
    }
}

impl Mode for MenuMode {
    // Up/Down で選択
    // Enter で決定 → 対応するAppModeを返す
}
```

#### 1.3 `src/lib.rs` をモード駆動ループに書き換え
```rust
pub fn run(mut terminal: DefaultTerminal) -> Result<()> {
    let mut current_mode = AppMode::Menu(MenuMode::new());
    
    loop {
        match &mut current_mode {
            AppMode::Exit => break,
            AppMode::Menu(m) => {
                m.update();
                terminal.draw(|f| m.render(f))?;
                if poll_and_handle_event(m, &mut current_mode)? { break; }
            }
            AppMode::OpenAIChat(m) => {
                m.update();
                terminal.draw(|f| m.render(f))?;
                if poll_and_handle_event(m, &mut current_mode)? { break; }
            }
            AppMode::RPGGame(m) => {
                m.update();
                terminal.draw(|f| m.render(f))?;
                if poll_and_handle_event(m, &mut current_mode)? { break; }
            }
        }
    }
    Ok(())
}
```

**テスト**: メニューが表示され、Escで終了できることを確認

---

### Phase 2: OpenAI対話モードの分離
**目的**: 既存の`App`を`OpenAIChatMode`として独立させる

#### 2.1 `src/modes/openai_chat.rs` 作成
- 既存の`src/app.rs`の`App`構造体をコピーして`OpenAIChatMode`に改名
- `src/openai`の依存はそのまま維持
- `Mode` traitを実装

#### 2.2 専用UIを`openai_chat.rs`内に移動
- `src/ui.rs`から関連する描画関数を`OpenAIChatMode`のメソッドor内部モジュールに移動
- または`src/ui/openai_chat.rs`に分離

#### 2.3 専用イベントハンドラーを移動
- `src/event.rs`の内容を`OpenAIChatMode::handle_key`に統合

#### 2.4 `src/lib.rs`から呼び出し
```rust
AppMode::OpenAIChat(m) => {
    m.update(); // check_ai_response相当
    terminal.draw(|f| m.render(f))?;
    if let Some(next) = poll_and_handle_key(m)? {
        current_mode = next;
    }
}
```

**テスト**: メニューから"OpenAI Chat"を選択して既存機能が動作することを確認

---

### Phase 3: RPGゲームモードの統合
**目的**: `examples/rpg_manual.rs`をTUIモードとして統合

#### 3.1 `src/modes/rpg_game.rs` 作成
```rust
pub struct RpgGameMode {
    game: Game,  // src/rpg::Game
    message: String,
    input_buffer: String,
}

impl Mode for RpgGameMode {
    fn render(&self, f: &mut Frame) {
        // ゲーム状態（HP/敵情報）を表示
        // コマンド入力欄
        // メッセージログ
    }
    
    fn handle_key(&mut self, key: KeyEvent) -> Result<Option<AppMode>> {
        // A/H/R/Qキーでコマンド実行
        // Escでメニューに戻る
    }
}
```

#### 3.2 `src/rpg/ui.rs`の関数をratatuiウィジェット化
- `show_status()`を`Paragraph`ウィジェットに変換
- コマンドプロンプトを入力欄として実装

#### 3.3 `src/lib.rs`にRPGモード分岐追加

**テスト**: メニューから"RPG Game"を選択してゲームをプレイできることを確認

---

### Phase 4: 旧コードのクリーンアップ
**目的**: 不要になったファイルを整理

#### 4.1 削除候補
- `src/app.rs` → `modes/openai_chat.rs`に統合済みなら削除
- `src/ui.rs` → 各モードに分散済みなら削除
- `src/event.rs` → 各モードに統合済みなら削除

#### 4.2 `examples/`の整理
- `rpg_manual.rs` → TUI版に統合されたが、CLI版として残すか検討
- `rpg_ai.rs` → AI駆動モードとして残す（将来的にTUI統合も可能）

#### 4.3 ドキュメント更新
- `README.md`にメニュー機能を記載
- `.github/copilot-instructions.md`を新構造に合わせて更新

---

## ディレクトリ構造（移行後）

```
src/
  main.rs                # エントリーポイント（変更最小限）
  lib.rs                 # モード駆動ループ
  config.rs              # 設定（共通）
  openai/                # OpenAI API関連（共通ライブラリ）
  rpg/                   # RPGロジック（共通ライブラリ）
  sqlite/                # DB関連（共通ライブラリ）
  modes/
    mod.rs               # Mode trait + AppMode enum
    menu.rs              # メニューモード
    openai_chat.rs       # OpenAI対話モード（旧app.rs相当）
    rpg_game.rs          # RPGゲームモード
```

---

## 各フェーズの所要時間見積もり

| Phase | 作業内容 | 見積もり |
|-------|---------|---------|
| Phase 0 | 設計詳細化 | 30分 |
| Phase 1 | Mode trait + メニュー実装 | 1-2時間 |
| Phase 2 | OpenAI対話モード分離 | 1-2時間 |
| Phase 3 | RPGゲームモード統合 | 2-3時間 |
| Phase 4 | クリーンアップ | 1時間 |
| **合計** | | **5-8時間** |

---

## リスクと対策

### リスク1: 既存機能の破壊
**対策**: Phase 1完了時に既存の`run()`をバックアップとして残し、段階的に移行

### リスク2: チャンネル・スレッド管理の複雑化
**対策**: 各モードが独自にワーカースレッドを管理。メニュー遷移時に適切にクリーンアップ

### リスク3: UI描画のちらつき
**対策**: 既存の100ms pollロジックを維持。モード切り替え時のみ画面クリア

---

## 追加機能の提案（Phase 5以降）

### 1. OpenAI+RPG統合モード（AI RPGプレイヤー）
- `examples/rpg_ai.rs`の機能をTUI化
- リアルタイムでAIの思考過程とゲーム状態を表示

### 2. 履歴保存・ロード
- SQLiteを使って対話履歴やRPGセーブデータを保存
- メニューから"Resume"オプション

### 3. 設定画面モード
- モデル選択（gpt-4o-mini/gpt-4o）
- max_tokens調整
- ログレベル変更

---

## 参考: Mode trait設計案（詳細）

```rust
// src/modes/mod.rs
use color_eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::Frame;

pub trait Mode {
    /// フレーム毎の非ブロッキング更新（AIレスポンスチェックなど）
    fn update(&mut self);
    
    /// 画面描画
    fn render(&self, f: &mut Frame);
    
    /// キーイベント処理
    /// 戻り値: Some(次のモード) でモード遷移、None で継続
    fn handle_key(&mut self, key: KeyEvent) -> Result<Option<AppMode>>;
}

pub enum AppMode {
    Menu(MenuMode),
    OpenAIChat(OpenAIChatMode),
    RpgGame(RpgGameMode),
    Exit,
}

impl AppMode {
    pub fn update(&mut self) {
        match self {
            AppMode::Menu(m) => m.update(),
            AppMode::OpenAIChat(m) => m.update(),
            AppMode::RpgGame(m) => m.update(),
            AppMode::Exit => {}
        }
    }
    
    pub fn render(&self, f: &mut Frame) {
        match self {
            AppMode::Menu(m) => m.render(f),
            AppMode::OpenAIChat(m) => m.render(f),
            AppMode::RpgGame(m) => m.render(f),
            AppMode::Exit => {}
        }
    }
    
    pub fn handle_key(&mut self, key: KeyEvent) -> Result<Option<AppMode>> {
        match self {
            AppMode::Menu(m) => m.handle_key(key),
            AppMode::OpenAIChat(m) => m.handle_key(key),
            AppMode::RpgGame(m) => m.handle_key(key),
            AppMode::Exit => Ok(None),
        }
    }
}

pub mod menu;
pub mod openai_chat;
pub mod rpg_game;

pub use menu::MenuMode;
pub use openai_chat::OpenAIChatMode;
pub use rpg_game::RpgGameMode;
```

---

## まとめ

このプランに従うことで：
1. ✅ 起動時にメニューが表示される
2. ✅ OpenAI対話・RPGゲームを選択できる
3. ✅ UI・ロジックが分離され、各モードが独立
4. ✅ 既存機能を壊さず段階的に移行
5. ✅ 将来的な拡張（AI RPGプレイヤー、設定画面など）が容易

次のステップは **Phase 1: Mode traitとメニュー実装** です。
