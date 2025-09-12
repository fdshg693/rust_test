cargo run --example foo
cargo run --example bar
examples/ 配下は独立したバイナリターゲットとしてビルドされ、src/lib.rs のAPIをそのまま use mycrate::... で利用できます。