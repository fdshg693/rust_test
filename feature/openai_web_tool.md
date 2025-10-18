# OpenAI Tool呼び出し機能のWeb対応

## 現状の認識

### ✅ 現在の実装状況

**Web版（app_web）**: ツールなし版のOpenAI API呼び出しを使用
```rust
// crates/app_core/src/services/chat_service.rs
pub async fn get_response(&self, prompt: &str) -> Result<String> {
    // Web版では簡易版のAPIを使用（ツールなし）
    let answer = get_ai_answer_once(prompt, &self.config).await?;
    
    tracing::info!(target: "chat_service", "AI response received");
    Ok(answer)
}
```

**CLI版（app_cli）**: ツールあり版を使用（TUIモード内で動作）
```rust
// CLI版では multi_step_tool_answer_with_logger を使用
let tools = vec![build_number_guess_tool(8, 10)];
multi_step_tool_answer_with_logger(prompt, &tools, &config, Some(10), logger).await
```

---

## なぜWeb版でツールあり版が使えないのか

### 問題の根本原因：Send トレイト境界違反

#### エラー内容
```
error[E0277]: `dyn for<'a> FnMut(&'a MultiStepLogEvent)` cannot be sent between threads safely
```

#### 技術的詳細

1. **Axumの要求仕様**
   - Axumのハンドラは`Send + 'static`なFutureを返す必要がある
   - これは、リクエストハンドラが複数のスレッドで並行実行されるため

2. **問題のあるコード構造**
   ```rust
   // crates/app_core/src/openai/call/multi_step.rs
   pub async fn multi_step_tool_answer_with_logger<F>(
       original_user_prompt: &str,
       tools: &[ToolDefinition],
       config: &OpenAIConfig,
       max_loops: Option<usize>,
       mut logger: F,  // ← ここが問題
   ) -> Result<MultiStepAnswer>
   where
       F: FnMut(&MultiStepLogEvent)  // Send境界がない！
   {
       // ...
   }
   ```

3. **なぜSendが必要か**
   - `multi_step_tool_answer_with_logger`は内部で複数の`await`ポイントを持つ
   - `await`の前後でクロージャ`logger`がスタック上に保持される
   - Axumはこのハンドラを別スレッドに移動する可能性がある
   - → クロージャが`Send`でないと、スレッド間で安全に移動できない

4. **具体的な呼び出しチェーン**
   ```
   chat_api (Axumハンドラ - Sendが必要)
     ↓
   ChatService::get_response (async - Sendが必要)
     ↓
   multi_step_tool_answer_with_logger (loggerがSendでない)
     ↓ ❌ Send境界違反でコンパイルエラー
   ```

---

## 解決策：3つのアプローチ

### 🔧 解決策1：Send境界の追加（推奨）

#### 概要
`multi_step_tool_answer_with_logger`のクロージャに`Send`境界を追加し、Web/CLI両対応にする。

#### 実装手順

**Step 1: `multi_step.rs`の関数シグネチャを修正**

```rust
// crates/app_core/src/openai/call/multi_step.rs
pub async fn multi_step_tool_answer_with_logger<F>(
    original_user_prompt: &str,
    tools: &[ToolDefinition],
    config: &OpenAIConfig,
    max_loops: Option<usize>,
    mut logger: F,
) -> Result<MultiStepAnswer>
where
    F: FnMut(&MultiStepLogEvent) + Send  // ← Send を追加
{
    multi_step_tool_answer_with_logger_internal(
        original_user_prompt,
        tools,
        config,
        max_loops,
        Some(&mut logger),
    ).await
}
```

**Step 2: 内部実装関数も修正**

```rust
// crates/app_core/src/openai/call/multi_step.rs
async fn multi_step_tool_answer_with_logger_internal<F>(
    original_user_prompt: &str,
    tools: &[ToolDefinition],
    config: &OpenAIConfig,
    max_loops: Option<usize>,
    mut user_logger: Option<&mut F>,
) -> Result<MultiStepAnswer>
where
    F: FnMut(&MultiStepLogEvent) + Send  // ← Send を追加
{
    // 既存の実装
    // ...
}
```

**Step 3: `ChatService`を修正してツール版を使用**

```rust
// crates/app_core/src/services/chat_service.rs
pub async fn get_response(&self, prompt: &str) -> Result<String> {
    // ツールあり版を使用
    let tools = vec![build_number_guess_tool(8, 10)];
    
    let answer = multi_step_tool_answer_with_logger(
        prompt, 
        &tools, 
        &self.config, 
        Some(10),
        |ev| {
            // Sendなクロージャ（キャプチャなし or Send型のみキャプチャ）
            tracing::info!(target: "chat_service", event=%ev, "step");
        }
    ).await?;
    
    Ok(answer.final_answer)
}
```

**Step 4: ビルドとテスト**

```bash
# コアクレートのビルド
cargo build -p app_core

# Web版のビルド
cargo build -p app_web

# CLI版のビルド（既存機能が壊れていないか確認）
cargo build -p app_cli

# Web版の起動
cargo run -p app_web
```

#### メリット
- ✅ Web/CLI両方でツール呼び出し機能が使える
- ✅ コードの重複がない
- ✅ 既存のCLI機能に影響なし

#### デメリット
- ⚠️ CLI側のクロージャも`Send`である必要がある（通常は問題なし）

---

### 🔧 解決策2：Web専用の同期ラッパー

#### 概要
Web版では`Arc<Mutex<Vec<Event>>>`でイベントを収集し、後で取り出す方式。

#### 実装例

```rust
// crates/app_core/src/services/chat_service.rs
use std::sync::{Arc, Mutex};

pub async fn get_response(&self, prompt: &str) -> Result<String> {
    let tools = vec![build_number_guess_tool(8, 10)];
    
    // Sendなイベントコレクター
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_clone = Arc::clone(&events);
    
    let answer = multi_step_tool_answer_with_logger(
        prompt, 
        &tools, 
        &self.config, 
        Some(10),
        move |ev| {
            // Arc<Mutex<>>はSend
            if let Ok(mut e) = events_clone.lock() {
                e.push(ev.clone());
            }
        }
    ).await?;
    
    // 必要ならイベントログを取得
    let logged_events = events.lock().unwrap();
    tracing::debug!(target: "chat_service", events=?logged_events);
    
    Ok(answer.final_answer)
}
```

#### メリット
- ✅ `multi_step.rs`を変更せずに対応可能
- ✅ イベント履歴を保存できる

#### デメリット
- ⚠️ `MultiStepLogEvent`が`Clone`を実装している必要がある
- ⚠️ Mutexのオーバーヘッド

---

### 🔧 解決策3：Web/CLI分岐（現状維持）

#### 概要
現状のまま、Web版は簡易版、CLI版はツール版を使い分ける。

#### 実装方針

```rust
// crates/app_core/src/services/chat_service.rs
pub async fn get_response(&self, prompt: &str) -> Result<String> {
    // Web版：ツールなし
    get_ai_answer_once(prompt, &self.config).await
}

pub async fn get_response_with_tools(&self, prompt: &str) -> Result<String> {
    // CLI専用：ツールあり（CLI側で直接呼び出す）
    let tools = vec![build_number_guess_tool(8, 10)];
    
    let answer = multi_step_tool_answer_with_logger(
        prompt, 
        &tools, 
        &self.config, 
        Some(10),
        |ev| tracing::info!(target: "chat_service", event=%ev, "step")
    ).await?;
    
    Ok(answer.final_answer)
}
```

#### メリット
- ✅ 既存コードを変更しない
- ✅ 実装が簡単

#### デメリット
- ❌ Web版でツール呼び出しができない
- ❌ 機能に差が生まれる

---

## 推奨される移行手順

### Phase 1: Send境界の追加（1-2時間）

1. ✅ `multi_step.rs`の関数シグネチャに`Send`を追加
2. ✅ コンパイルエラーがないか確認
3. ✅ CLI版の既存機能テスト（`cargo run -p app_cli`）

### Phase 2: ChatServiceの修正（30分）

1. ✅ `get_response`メソッドでツール版APIを使用
2. ✅ ビルド確認（`cargo build -p app_core`）

### Phase 3: Web版の動作確認（30分）

1. ✅ `cargo run -p app_web`でサーバー起動
2. ✅ ブラウザでチャット機能をテスト
3. ✅ ツール呼び出しのログ確認

### Phase 4: ドキュメント更新（15分）

1. ✅ `README.md`にツール対応を記載
2. ✅ APIドキュメント更新

---

## コード差分例

### Before（現状）

```rust
// crates/app_core/src/openai/call/multi_step.rs
pub async fn multi_step_tool_answer_with_logger<F>(
    original_user_prompt: &str,
    tools: &[ToolDefinition],
    config: &OpenAIConfig,
    max_loops: Option<usize>,
    mut logger: F,
) -> Result<MultiStepAnswer>
where
    F: FnMut(&MultiStepLogEvent)  // Sendなし
{
    // ...
}
```

```rust
// crates/app_core/src/services/chat_service.rs
pub async fn get_response(&self, prompt: &str) -> Result<String> {
    // ツールなし版
    let answer = get_ai_answer_once(prompt, &self.config).await?;
    Ok(answer)
}
```

### After（推奨）

```rust
// crates/app_core/src/openai/call/multi_step.rs
pub async fn multi_step_tool_answer_with_logger<F>(
    original_user_prompt: &str,
    tools: &[ToolDefinition],
    config: &OpenAIConfig,
    max_loops: Option<usize>,
    mut logger: F,
) -> Result<MultiStepAnswer>
where
    F: FnMut(&MultiStepLogEvent) + Send  // ← Send追加
{
    // ...
}
```

```rust
// crates/app_core/src/services/chat_service.rs
pub async fn get_response(&self, prompt: &str) -> Result<String> {
    // ツールあり版に変更
    let tools = vec![build_number_guess_tool(8, 10)];
    
    let answer = multi_step_tool_answer_with_logger(
        prompt, 
        &tools, 
        &self.config, 
        Some(10),
        |ev| tracing::info!(target: "chat_service", event=%ev, "step")
    ).await?;
    
    Ok(answer.final_answer)
}
```

---

## よくある質問（FAQ）

### Q1: Sendを追加するとCLI側が壊れる？

**A**: いいえ、壊れません。CLI側のクロージャも自動的にSendになります（キャプチャする変数がSendなら）。

### Q2: パフォーマンスへの影響は？

**A**: Send境界の追加自体はコンパイル時のチェックなので、ランタイムのパフォーマンスには影響しません。

### Q3: 他のクロージャパラメータも修正が必要？

**A**: `get_response_with_tools`など、Webから呼ばれる可能性のあるメソッドは全て`Send`が必要です。

### Q4: なぜCLI版は動いている？

**A**: CLI版（TUI）は単一スレッド内で動作しているため、Sendが不要です。Webはマルチスレッドなので必須です。

---

## 参考リンク

- [Rustの並行性とSend/Sync](https://doc.rust-lang.org/book/ch16-04-extensible-concurrency-sync-and-send.html)
- [Axum Handlers and Extractors](https://docs.rs/axum/latest/axum/handler/index.html)
- [async-openai Documentation](https://docs.rs/async-openai/)

---

## まとめ

**現状**: Web版はツールなし版の`get_ai_answer_once`を使用

**理由**: `multi_step_tool_answer_with_logger`のクロージャが`Send`でないため、Axumのマルチスレッド環境で使用できない

**推奨解決策**: `multi_step.rs`の関数シグネチャに`+ Send`を追加（1行の変更で解決）

**作業時間**: 合計2-3時間（テスト含む）

**リスク**: 低（既存CLI機能への影響なし）
