/// アプリケーション設定
pub struct OpenAIConfig {
    /// OpenAI APIモデル名
    pub model: String,
    /// 最大トークン数
    /// 4o系モデルに利用
    pub max_tokens: u32,
    /// 5系モデルに利用
    pub max_completion_tokens: u32,
    /// イベントポーリング間隔（ミリ秒）
    pub poll_interval_ms: u64,
}

impl Default for OpenAIConfig {
    fn default() -> Self {
        Self {
            model: "gpt-5".to_string(),
            // NOTE: Keep in sync with tests (tests/config_tests.rs) and design doc.
            max_tokens: 10000,
            max_completion_tokens: 16000,
            poll_interval_ms: 100,
        }
    }
}

impl OpenAIConfig {
    /// 新しい設定インスタンスを作成
    pub fn new() -> Self {
        Self::default()
    }
}
