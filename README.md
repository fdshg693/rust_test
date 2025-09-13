# プロジェクト構造 - Rustベストプラクティス

このドキュメントでは、リファクタリング後のプロジェクト構造について説明します。

## 新しいモジュール構造

```
src/
├── main.rs          # エントリーポイント（最小限）
├── lib.rs           # ライブラリルートとメインループ
├── app.rs           # アプリケーション状態管理
├── config.rs        # 設定と定数
├── event.rs         # イベント処理
├── openai.rs        # OpenAI API統合
└── ui.rs            # UI描画コンポーネント
```

## 各モジュールの責務

### `main.rs`
- アプリケーションのエントリーポイント
- 最小限のコードで、ライブラリの`run`関数を呼び出すのみ

### `lib.rs`
- ライブラリのルートモジュール
- 各モジュールの公開とメインループの実装
- モジュール間の調整役

### `app.rs`
- アプリケーションの状態管理
- `App`構造体とその関連メソッド
- チャンネル通信の管理

### `config.rs`
- アプリケーション設定
- 定数の定義（X, Y）
- 設定可能なパラメータ

### `event.rs`
- キーボードイベントの処理
- ユーザー入力の解釈とアプリケーション状態の更新

### `openai.rs`
- OpenAI APIとの統合
- バックグラウンドワーカーの管理
- API呼び出しと応答処理

### `ui.rs`
- UI描画ロジック
- Ratutuiを使用したコンポーネント描画
- 画面レイアウトの管理

## ベストプラクティスの適用

1. **単一責任原則**: 各モジュールが明確な責務を持つ
2. **関心の分離**: UI、ビジネスロジック、API統合を分離
3. **モジュール化**: 再利用可能で保守しやすいコード構造
4. **型安全性**: Rustの型システムを活用した安全なコード
5. **エラーハンドリング**: `Result`型を使用した適切なエラー処理

## 利点

- **保守性**: 各機能が独立したモジュールに分離されている
- **テスト容易性**: 各モジュールを個別にテストできる
- **再利用性**: モジュールを他のプロジェクトで再利用可能
- **可読性**: コードの構造が明確で理解しやすい
- **拡張性**: 新機能の追加が容易

## 実行方法

```bash
# 開発モードで実行
cargo run

# リリースモードで実行
cargo run --release

# コンパイルチェック
cargo check

# テスト実行
cargo test

## TAVILY Search ツール統合

OpenAI の function calling から利用できる Web 検索ツール `TAVILY_search` を追加しました。モデルがツール呼び出しを提案すると、バックエンドワーカーが TAVILY API を呼び出し、その結果(JSON)を最終回答生成に渡します。

### 環境変数
`TAVILY_API_KEY` を設定してください。未設定の場合、ツールは `{ "error": "missing TAVILY_API_KEY env" }` を返します。

Windows PowerShell 例:
```powershell
$env:TAVILY_API_KEY = "tvly-..."
```

### ツール名と引数スキーマ
- ツール名: `TAVILY_search`
- 引数:
	- `query`: 文字列 (必須)
	- `max_results`: 数値 (任意, 1-10, 省略時 5)

### 返却JSON (例)
TAVILY API のレスポンス JSON をそのまま（あるいは `raw` / `error` フィールドを含む簡易オブジェクト）で返します。

### 実装概要
- ファイル: `src/openai/TAVILY.rs`
- HTTPクライアント: `reqwest` (blocking) をワーカースレッドで同期利用
- OpenAI との 2 ステップ: ツール提案 -> 実行 -> 関数結果を 2 度目の Chat Completion に投入

### 拡張アイデア
- 検索深度や日時フィルタなど追加パラメータをスキーマに露出
- 非同期化 / キャッシュ
- レスポンス要約を補助する追加ツール

### 直接呼び出し
アプリ内部やテストから直接 Web 検索を行いたい場合は関数を利用できます:

```rust
use rust_test::openai::TAVILY_search;

let json = TAVILY_search("Rust programming language", 3)?;
println!("{:?}", json);
```

### ツール提案と実行の簡略化ヘルパ

`call_tool.rs` に `resolve_and_execute_tool_call` と結果列挙体 `ToolResolution` を追加し、
ツール提案 (`ToolCallDecision`) から実行フェーズまでの典型処理を統一しました。

```rust
use rust_test::openai::{propose_tool_call, resolve_and_execute_tool_call, ToolResolution};

// 1. ツール提案
let decision = propose_tool_call("定数を教えて", &tools, &config).await?;
// 2. 提案結果の解決 & 実行
let resolution = resolve_and_execute_tool_call(decision, &tools);

match resolution {
	ToolResolution::ModelText(t) => println!("model text: {t}"),
	ToolResolution::Executed { name, result } => println!("executed {name}: {result}"),
	ToolResolution::ToolNotFound { requested } => eprintln!("tool not found: {requested}"),
	ToolResolution::ArgumentsParseError { name, error, .. } => eprintln!("args parse error for {name}: {error}"),
	ToolResolution::ExecutionError { name, error } => eprintln!("execution error for {name}: {error}"),
}
```

`ToolResolution::Executed` の場合のみ 2 回目の Chat Completion を行い会話へ組み込む実装は
`openai/worker.rs` を参照してください。

