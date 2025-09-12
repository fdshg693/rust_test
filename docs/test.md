# テストの実行方法

このプロジェクトのテストは、ネットワーク非依存の通常テストと、OpenAI を実際に叩くライブテスト（デフォルト無効）の2種類があります。以下は Windows PowerShell 向けの手順です。

## 前提
- Rust (cargo) がインストール済み
- リポジトリのルート（`C:\\CodeStudy\\rust_test`）で実行

## 1) 通常テスト（ネットワーク不使用）
プロジェクト内のユニット/統合テストをすべて実行します。

```powershell
cargo test
```

## 2) ライブテスト（OpenAI API を実呼び出し）
`tests/openai_simple_live_tests.rs` にある `live_get_ai_answer_once_blocking` は実際に OpenAI API を呼び出します。デフォルトでは `#[ignore]` でスキップされます。実行するには API キーを設定し、ignored テストを指定してください。

```powershell
# OpenAI API キーを設定
$env:OPENAI_API_KEY = "sk-..."

# ignored テストも含めて実行
cargo test -- --ignored

# レスポンスの標準出力（println!）も見たい場合
cargo test -- --ignored --nocapture
```

特定のライブテストだけを実行するにはテスト名でフィルタします。

```powershell
cargo test live_get_ai_answer_once_blocking -- --ignored --nocapture
```

- 使われる関数: `openai::get_ai_answer_once_blocking`
- 設定: `Config::new()` のデフォルト（`model = "gpt-4o-mini"`, `max_tokens = 512`）

## 3) テストの絞り込み実行（通常テスト）
テスト名やファイル単位でフィルタできます。

```powershell
# テスト名で実行
cargo test config_defaults

# ファイル名の一部で実行（例: app の統合テスト）
cargo test app_tests
```

## 4) トラブルシュート
- `OPENAI_API_KEY` 未設定: ライブテストは自動的にスキップされます（通常テストは影響なし）。
- 401/403: API キーが無効・権限不足の可能性。キーを確認してください。
- 429: レート制限。時間をおいて再実行してください。
- ネットワーク/プロキシ: 企業ネットワークやFWの影響で失敗することがあります。接続環境をご確認ください。
- モデル/トークン調整: デフォルトは `src/config.rs` を参照。必要に応じて編集してください。

## 補足
- ライブテストのソース: `tests/openai_simple_live_tests.rs`
- 通常テストのソース例:
  - `tests/config_tests.rs`: Config のデフォルト・定数の検証
  - `tests/app_tests.rs`: 送受信フローの簡易統合テスト（エコー用スレッドで外部依存なし）
