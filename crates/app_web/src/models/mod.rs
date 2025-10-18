use serde::{Deserialize, Serialize};

/// OpenAI Chatリクエスト
#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub prompt: String,
}

/// OpenAI Chatレスポンス
#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub response: String,
}

/// RPGアクションリクエスト
#[derive(Debug, Deserialize)]
pub struct RpgActionRequest {
    pub action: String, // "attack", "heal", "run", "quit"
}

/// RPG状態レスポンス（GameSnapshotをそのままJSONで返す）
#[derive(Debug, Serialize)]
pub struct RpgStateResponse {
    pub player_hp: u32,
    pub player_hp_max: u32,
    pub enemy_hp: u32,
    pub enemy_hp_max: u32,
    pub turn: u32,
    pub message: String,
    pub game_over: bool,
}

/// エラーレスポンス
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}
