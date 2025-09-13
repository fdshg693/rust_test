//! アプリケーション設定と定数

/// 取得対象の定数 X
pub const X: i32 = 42;

/// 取得対象の定数 Y
pub const Y: i32 = 7;

/// アプリケーション設定
pub struct Config {
    /// OpenAI APIモデル名
    pub model: String,
    /// 最大トークン数
    pub max_tokens: u32,
    /// イベントポーリング間隔（ミリ秒）
    pub poll_interval_ms: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model: "gpt-4o-mini".to_string(),
            // NOTE: Keep in sync with tests (tests/config_tests.rs) and design doc.
            max_tokens: 2000,
            poll_interval_ms: 100,
        }
    }
}

impl Config {
    /// 新しい設定インスタンスを作成
    pub fn new() -> Self {
        Self::default()
    }
}
