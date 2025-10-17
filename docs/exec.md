# AIでプレイ（ツールを使って多段推論）
cargo run --example rpg_ai
- デバッグ用にSTDOUTにログを出力する場合:
```$env:RUST_LOG = "example_rpg_ai=info,openai=debug"```

# 手動ツール実行（モデル不使用）
cargo run --example rpg_manual