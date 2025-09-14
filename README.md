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
├── sqlite/          # SQLite 仮想ファイルストレージユーティリティ
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

### `sqlite/`
- `rusqlite` を用いたシンプルな仮想ファイルストレージ
- API: `Db::open_or_create`, `upsert_text`, `upsert_bytes`, `read_text`, `read_bytes`, `list_files`, `delete`, `import_file_from_fs`, `export_file_to_fs`
- テーブル `files(path PRIMARY KEY, data BLOB, size_bytes INTEGER, modified_at_epoch_ms INTEGER)`

#### 使用例
```rust
use rust_test::sqlite::Db;

fn main() -> color_eyre::Result<()> {
	color_eyre::install()?;
	let mut db = Db::open_or_create("app_data.sqlite")?;
	db.upsert_text("notes/hello.txt", "Hello SQLite")?;
	let txt = db.read_text("notes/hello.txt")?;
	println!("{}", txt);
	for entry in db.list_files("notes/%")? {
		println!("{} ({} bytes)", entry.path, entry.size_bytes);
	}
	Ok(())
}
```

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
