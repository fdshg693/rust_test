# ベンチマークの使い方

このプロジェクトには Criterion を使った簡単なベンチが含まれています。
以下はベンチの実行方法とトラブルシューティング、拡張方法のガイドです。

## 概要
- ベンチ本体: `benches/simple_bench.rs`
- dev-dependency: `criterion = "0.4"` を `Cargo.toml` の `[dev-dependencies]` に追加済み
- 重要: `Cargo.toml` に `[[bench]]` セクションを追加し `harness = false` を設定してあります。
  これは Criterion が独自の `main` を必要とするためで、テストハーネスと混ざると出力が抑止されることがあるためです。

## 前提
- Rust がインストールされていること（`cargo` が使えること）
- Windows PowerShell を想定したコマンド例を記載しています

## ベンチを実行する
PowerShell でリポジトリのルート（このプロジェクト）に移動して次を実行します:

```powershell
cargo bench
```

通常は `cargo bench` が Criterion ベンチをビルドして実行し、統計が標準出力に表示されます。
もし出力が見えない（またはテストのように表示される）場合は、`Cargo.toml` の `[[bench]]` に `harness = false` が設定されているか確認してください。

## ベンチ実行バイナリを直接実行する
ビルド済みのベンチ実行ファイルを直接起動して出力を確認できます（Cargo のラッパーをバイパス）。まず該当ファイルパスを取得して実行します:

```powershell
# 実行ファイルのパスを探して実行
$path = Get-ChildItem target\release\deps\simple_bench*.exe | Select-Object -First 1 -ExpandProperty FullName
& $path
```

> 注意: 実行ファイル名にはビルドハッシュが付きます。上のコマンドはそれを自動検出して実行します。

## 出力の見方
Criterion は実行ごとに統計（中央値、レンジ、ヒストグラム等）を表示します。例:

```
sort 1000 ints          time:   [10.099 µs 10.151 µs 10.220 µs]
Found 7 outliers among 100 measurements (7.00%)
  4 (4.00%) high mild
  3 (3.00%) high severe
```

- `time:` のレンジが測定された時間帯を表します。
- `outliers` は外れ値。測定環境（バックグラウンドプロセスなど）によって発生します。
- 詳細なレポート（HTML/CSV 等）は `target/criterion` フォルダに生成されます。

```powershell
# Criterion が生成する詳細結果を確認する
Get-ChildItem -Directory target\criterion
```

## ベンチの追加・カスタマイズ
- 新しいベンチを追加するには `benches/` に `xxx_bench.rs` を作成します。
- Criterion の標準 API（`criterion_group!` / `criterion_main!`）を使ってベンチを定義してください。
- 独立バイナリとして実行する必要がある場合は `Cargo.toml` に以下のようなエントリを追加します:

```toml
[[bench]]
name = "xxx_bench"
path = "benches/xxx_bench.rs"
harness = false
```

## CI での実行について（簡単メモ）
- ベンチは非決定的になりやすいので CI にそのまま入れるとノイズが多くなります。
- CI で使う場合は安定化用の設定（固定の CPU コア割当、最低実行回数の設定、結果差分の閾値）を検討してください。
- Criterion は結果を `target/criterion` に保存するため、履歴を CI アーティファクトとして保存できます。

## トラブルシューティング
- ベンチの出力が出ない → `Cargo.toml` の `harness = false` を確認。
- 非同期コードや外部リソースを測定する場合は、測定対象を短くして環境ノイズの影響を減らしてください。

---
このドキュメントはベンチの簡単な操作ガイドです。具体的な測定対象やレポート形式（CSV/HTML）を追加したい場合は教えてください。
