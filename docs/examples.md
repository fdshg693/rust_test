cargo run --example foo
cargo run --example bar
examples/ 配下は独立したバイナリターゲットとしてビルドされ、src/lib.rs のAPIをそのまま use mycrate::... で利用できます。

## OpenAI Tools: `read_docs_file`

ローカル `docs/` ディレクトリ内の Markdown ファイル内容を返す Function Calling 用ツールを追加しました。

- ツール名: `read_docs_file`
- 引数: `filename` (必須, 以下のいずれか)
	- `benches.md`
	- `examples.md`
	- `ratatui.md`
	- `test.md`
- 戻り JSON 例:
```json
{
	"filename": "examples.md",
	"content": "<file text ...>"
}
```
サイズ上限 (16KB) を超える場合は `truncated: true, max_bytes: 16384` が付与されます。

セキュリティ: 許可リスト外やパストラバーサル (`../` など) は `{ "error": "..." }` を返します。