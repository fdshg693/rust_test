# OpenAI History Refactor Plan

**目的**: `ConversationHistory`を使ってAI呼び出しを統一し、システムプロンプトの管理責務を追加する

**作成日**: 2025-10-18

---

## 現状分析

### 問題点

1. **手動メッセージ構築が散在**
   - `ChatCompletionRequestSystemMessageArgs` など各種MessageArgsを直接使用している箇所:
     - `src/openai/simple.rs`: L20-26 (system + user)
     - `src/openai/call/proposer.rs`: L25-31 (system + user)
   - 各所で同じようなシステムプロンプト "あなたは簡潔な日本語で答えるアシスタントです。" をハードコード

2. **ConversationHistoryの限定的な使用**
   - 現在は `multi_step.rs` のみで使用
   - user/assistant/functionメッセージのみサポート
   - システムメッセージは管理対象外（意図的に除外）

3. **責務の不明確さ**
   - システムプロンプト: 各関数内でハードコード
   - 会話履歴: ConversationHistoryで管理
   - メッセージ順序: 各関数で手動組み立て

---

## リファクタリング方針

### フェーズ1: ConversationHistoryへのシステムプロンプト機能追加

#### 1.1 設計変更

**新規メソッド追加**:
```rust
impl ConversationHistory {
    /// システムメッセージを設定（最初に配置）
    pub fn set_system<S: AsRef<str>>(&mut self, content: S) -> &mut Self;
    
    /// システムメッセージを削除
    pub fn clear_system(&mut self) -> &mut Self;
    
    /// システムメッセージ込みで全メッセージを取得
    pub fn as_slice_with_system(&self) -> Vec<ChatCompletionRequestMessage>;
    
    /// デフォルトシステムプロンプト付きで新規作成
    pub fn with_default_system() -> Self;
}
```

**内部構造変更**:
```rust
pub struct ConversationHistory {
    system_message: Option<ChatCompletionRequestMessage>,
    messages: Vec<ChatCompletionRequestMessage>,
}
```

#### 1.2 実装詳細

- `system_message: Option<ChatCompletionRequestMessage>` フィールドを追加
- `as_slice()` は既存の動作を維持（システムメッセージ除外）
- `as_slice_with_system()` で system + messages を返す
- デフォルトシステムプロンプトは `config.rs` に定数として定義

**config.rsへの追加**:
```rust
pub const DEFAULT_SYSTEM_PROMPT: &str = "あなたは簡潔な日本語で答えるアシスタントです。";
```

#### 1.3 テスト追加

`src/openai/history.rs` のテストセクションに追加:
- `set_system_and_retrieve`: システムメッセージ設定と取得
- `system_at_beginning`: システムメッセージが常に先頭
- `clear_system_removes`: システムメッセージ削除確認
- `with_default_system_constructor`: デフォルトコンストラクタ

---

### フェーズ2: 各API関数の統一化

#### 2.1 simple.rs のリファクタリング

**変更前** (L15-26):
```rust
pub async fn get_ai_answer_once(prompt: &str, config: &OpenAIConfig) -> Result<String> {
    let client = Client::new();
    let system = ChatCompletionRequestSystemMessageArgs::default()
        .content("あなたは簡潔な日本語で答えるアシスタントです。")
        .build()?;
    let user = ChatCompletionRequestUserMessageArgs::default()
        .content(prompt)
        .build()?;
    let req = CreateChatCompletionRequestArgs::default()
        .model(&config.model)
        .messages([system.into(), user.into()])
        // ...
```

**変更後**:
```rust
pub async fn get_ai_answer_once(prompt: &str, config: &OpenAIConfig) -> Result<String> {
    let client = Client::new();
    let mut history = ConversationHistory::with_default_system();
    history.add_user(prompt);
    
    let req = CreateChatCompletionRequestArgs::default()
        .model(&config.model)
        .messages(history.as_slice_with_system())
        // ...
```

**影響範囲**:
- `get_ai_answer_once` 関数
- import文から `ChatCompletionRequestSystemMessageArgs`, `ChatCompletionRequestUserMessageArgs` を削除
- `use crate::openai::ConversationHistory;` を追加
- `use crate::config::DEFAULT_SYSTEM_PROMPT;` を追加（必要に応じて）

#### 2.2 proposer.rs のリファクタリング

**変更前** (L23-36):
```rust
let system = ChatCompletionRequestSystemMessageArgs::default()
    .content("あなたは簡潔な日本語で答えるアシスタントです。")
    .build()?;
let user = ChatCompletionRequestUserMessageArgs::default()
    .content(prompt)
    .build()?;

let mut messages: Vec<ChatCompletionRequestMessage> = Vec::with_capacity(1 + history.len() + 1);
messages.push(system.into());
messages.push(user.into());
messages.extend_from_slice(history);
```

**変更後**:
```rust
let mut full_history = ConversationHistory::with_default_system();
full_history.add_user(prompt);
// 既存の会話履歴をマージ
for msg in history {
    full_history.push(msg.clone());
}

let messages = full_history.as_slice_with_system();
```

**影響範囲**:
- `propose_tool_call` 関数の引数は変更なし（下位互換性維持）
- import文から手動MessageArgs削除
- `use crate::openai::ConversationHistory;` を追加

#### 2.3 multi_step.rs の調整

**現状**: 既に `ConversationHistory` を使用しているが、システムメッセージなし

**変更**:
```rust
// Before (L61-62):
let mut history = ConversationHistory::new();
history.add_user(original_user_prompt);

// After:
let mut history = ConversationHistory::with_default_system();
history.add_user(original_user_prompt);
```

**proposer呼び出し部分** (L67):
```rust
// Before:
let decision = propose_tool_call(history.as_slice(), "", tools, config).await?;

// After (proposer側が内部でsystem追加するため、ここは一旦そのまま):
// proposer側リファクタ完了後は history.as_slice()で十分
```

**注意**: proposer側のリファクタ完了後、multi_stepから渡すhistoryにシステムメッセージが含まれないよう調整が必要。proposer内で改めてシステムメッセージを追加する設計とする。

---

### フェーズ3: システムプロンプトのカスタマイズ対応

#### 3.1 設定ベース＋オーバーライド

**config.rsへの追加**:
```rust
#[derive(Debug, Clone)]
pub struct OpenAIConfig {
    pub model: String,
    pub max_tokens: u32,
    pub max_completion_tokens: u32,
    pub system_prompt: Option<String>, // 新規追加
}

impl Default for OpenAIConfig {
    fn default() -> Self {
        Self {
            model: DEFAULT_MODEL.to_string(),
            max_tokens: DEFAULT_MAX_TOKENS,
            max_completion_tokens: DEFAULT_MAX_COMPLETION_TOKENS,
            system_prompt: None, // Noneならデフォルト使用
        }
    }
}
```

#### 3.2 ConversationHistoryへの反映

```rust
impl ConversationHistory {
    /// 設定からシステムプロンプトを設定
    pub fn with_config_system(config: &OpenAIConfig) -> Self {
        let mut h = Self::new();
        let prompt = config.system_prompt.as_deref()
            .unwrap_or(DEFAULT_SYSTEM_PROMPT);
        h.set_system(prompt);
        h
    }
}
```

#### 3.3 各API関数の更新

**simple.rs**:
```rust
let mut history = ConversationHistory::with_config_system(config);
```

**proposer.rs**:
```rust
let mut full_history = ConversationHistory::with_config_system(config);
```

---

## 実装順序

### ステップ1: ConversationHistory拡張 (推定: 30分)
- [ ] `src/openai/history.rs` に system_message フィールド追加
- [ ] `set_system()`, `clear_system()`, `as_slice_with_system()` 実装
- [ ] `with_default_system()` コンストラクタ追加
- [ ] テスト追加・確認

### ステップ2: config.rs 更新 (推定: 10分)
- [ ] `DEFAULT_SYSTEM_PROMPT` 定数追加
- [ ] `OpenAIConfig` に `system_prompt: Option<String>` フィールド追加
- [ ] Default実装更新

### ステップ3: simple.rs リファクタ (推定: 15分)
- [ ] 手動メッセージ構築を `ConversationHistory` に置き換え
- [ ] import文整理
- [ ] 既存テスト実行確認 (`tests/openai_simple_live_tests.rs`)

### ステップ4: proposer.rs リファクタ (推定: 20分)
- [ ] 手動メッセージ構築を `ConversationHistory` に置き換え
- [ ] import文整理
- [ ] 既存テスト実行確認 (`tests/openai_tool_live_tests.rs`)

### ステップ5: multi_step.rs 調整 (推定: 15分)
- [ ] `with_default_system()` 使用に変更
- [ ] proposerとの連携確認
- [ ] 既存テスト実行確認 (`tests/multi_step_live_test.rs`)

### ステップ6: 統合テスト (推定: 20分)
- [ ] 全テストスイート実行 (`cargo test`)
- [ ] TUIアプリでOpenAIChatMode動作確認
- [ ] RPG AIモード動作確認

### ステップ7: ドキュメント更新 (推定: 10分)
- [ ] `src/openai/history.rs` のモジュールドキュメント更新
- [ ] `.github/copilot-instructions.md` のConversationHistory記述更新

---

## マイルストーン

### マイルストーン1: 基本統一化完了
- ConversationHistory拡張
- simple.rs, proposer.rs, multi_step.rs リファクタ完了
- 全既存テストパス

### マイルストーン2: カスタマイズ対応完了
- OpenAIConfigへのsystem_prompt追加
- with_config_system()実装
- 各モードでカスタムプロンプト使用可能

---

## リスクと対策

### リスク1: 既存動作の破壊
**対策**: 
- 各ステップ後に対応するlive testを実行
- as_slice()は既存動作維持（システムメッセージ含めない）

### リスク2: メッセージ順序の誤り
**対策**:
- as_slice_with_system()で必ずsystemが先頭に来ることをテストで保証
- proposer内でのメッセージ組み立て順序を慎重に確認

### リスク3: multi_stepでの重複システムメッセージ
**対策**:
- proposer側でシステムメッセージを管理
- multi_stepから渡すhistoryはas_slice()を使用（システムメッセージなし）
- proposer内部でfull_historyを構築時に改めてシステムメッセージ追加

---

## 成功基準

1. ✅ 全ての `ChatCompletionRequestSystemMessageArgs` 直接使用箇所が削除される
2. ✅ `ConversationHistory` が全てのOpenAI API呼び出しで使用される
3. ✅ システムプロンプトが一箇所（config.rs）で管理される
4. ✅ 全既存テストがパスする
5. ✅ TUIアプリの全モードが正常動作する

---

## 今後の拡張可能性

1. **モード別カスタムプロンプト**:
   - RPGモード専用システムプロンプト
   - OpenAIChatモード専用システムプロンプト
   
2. **動的プロンプト切り替え**:
   - ユーザー設定によるシステムプロンプト変更UI
   
3. **プロンプトテンプレート**:
   - 複数のプリセットプロンプトから選択可能に

4. **会話履歴の永続化**:
   - ConversationHistoryのシリアライズ/デシリアライズ
   - SQLiteへの保存/読み込み
