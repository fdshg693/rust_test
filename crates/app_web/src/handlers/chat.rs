use axum::{
    extract::Json,
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::json;
use tower_sessions::Session;

use app_core::{OpenAIConfig, services::ChatService};
use crate::models::{ChatRequest, ChatResponse, ErrorResponse};

const CHAT_HISTORY_KEY: &str = "chat_history";

/// POST /api/chat - チャットメッセージを送信してAI応答を取得
#[axum::debug_handler]
pub async fn chat_api(
    session: Session,
    Json(req): Json<ChatRequest>,
) -> impl IntoResponse {
    tracing::info!(target: "web::chat", prompt = %req.prompt, "Received chat request");

    // 設定とサービスの初期化
    let config = OpenAIConfig::default();
    let service = ChatService::new(config);

    // AI応答取得（非同期）
    match service.get_response(&req.prompt).await {
        Ok(response) => {
            // セッションに履歴を保存（オプション）
            let mut history: Vec<(String, String)> = session
                .get(CHAT_HISTORY_KEY)
                .await
                .unwrap_or_default()
                .unwrap_or_default();
            
            history.push((req.prompt.clone(), response.clone()));
            
            // 履歴を最新10件に制限
            if history.len() > 10 {
                let skip_count = history.len() - 10;
                history = history.into_iter().skip(skip_count).collect();
            }
            
            session.insert(CHAT_HISTORY_KEY, &history).await.ok();

            tracing::info!(target: "web::chat", "Chat response successful");
            Json(ChatResponse { response }).into_response()
        }
        Err(e) => {
            tracing::error!(target: "web::chat", error = %e, "Chat request failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to get AI response: {}", e),
                }),
            ).into_response()
        }
    }
}

/// GET /api/chat/history - チャット履歴を取得
pub async fn chat_history(
    session: Session,
) -> Json<serde_json::Value> {
    let history: Vec<(String, String)> = session
        .get(CHAT_HISTORY_KEY)
        .await
        .unwrap_or_default()
        .unwrap_or_default();

    Json(json!({ "history": history }))
}

/// DELETE /api/chat/history - チャット履歴をクリア
pub async fn clear_chat_history(
    session: Session,
) -> impl IntoResponse {
    session.remove::<Vec<(String, String)>>(CHAT_HISTORY_KEY).await.ok();
    tracing::info!(target: "web::chat", "Chat history cleared");
    StatusCode::NO_CONTENT
}
