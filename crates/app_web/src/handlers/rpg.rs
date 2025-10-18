use axum::{
    extract::Json,
    http::StatusCode,
    response::IntoResponse,
};
use tower_sessions::Session;

use app_core::{rpg::Command, services::RpgService};
use crate::models::{RpgActionRequest, RpgStateResponse, ErrorResponse};

const RPG_SESSION_KEY: &str = "rpg_service";

/// POST /api/rpg/action - RPGアクションを実行
pub async fn rpg_action(
    session: Session,
    Json(req): Json<RpgActionRequest>,
) -> impl IntoResponse {
    tracing::info!(target: "web::rpg", action = %req.action, "Received RPG action");

    // セッションからゲーム状態を取得（なければ新規作成）
    let mut service: RpgService = session
        .get(RPG_SESSION_KEY)
        .await
        .unwrap_or_default()
        .unwrap_or_else(|| {
            tracing::info!(target: "web::rpg", "Creating new RPG game");
            RpgService::new()
        });

    // コマンドをパース
    let cmd = match req.action.as_str() {
        "attack" => Command::Attack,
        "heal" => Command::Heal,
        "run" => Command::Run,
        "quit" => Command::Quit,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!("Invalid action: {}", req.action),
                }),
            ).into_response();
        }
    };

    // コマンド実行
    let result = service.execute_command(cmd);
    
    match result {
        Ok(snapshot) => {
            // セッションに状態を保存
            session.insert(RPG_SESSION_KEY, &service).await.ok();

            tracing::info!(target: "web::rpg", "RPG action successful");

            (
                StatusCode::OK,
                Json(RpgStateResponse {
                    player_hp: snapshot.player.hp as u32,
                    player_hp_max: snapshot.player.max_hp as u32,
                    enemy_hp: snapshot.enemy.hp as u32,
                    enemy_hp_max: 50, // Enemy初期HPはルールから取得すべきだが、簡易的に50
                    turn: snapshot.battle_count as u32,
                    message: format!("Turn: {:?} | Battle #{}", snapshot.turn, snapshot.battle_count),
                    game_over: snapshot.is_over,
                })
            ).into_response()
        }
        Err(e) => {
            tracing::error!(target: "web::rpg", error = %e, "RPG action failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            ).into_response()
        }
    }
}

/// GET /api/rpg/state - 現在のRPGゲーム状態を取得
pub async fn rpg_state(
    session: Session,
) -> impl IntoResponse {
    let service: RpgService = session
        .get(RPG_SESSION_KEY)
        .await
        .unwrap_or_default()
        .unwrap_or_else(|| RpgService::new());

    let snapshot = service.snapshot();

    Json(RpgStateResponse {
        player_hp: snapshot.player.hp as u32,
        player_hp_max: snapshot.player.max_hp as u32,
        enemy_hp: snapshot.enemy.hp as u32,
        enemy_hp_max: 50,
        turn: snapshot.battle_count as u32,
        message: format!("Turn: {:?} | Battle #{}", snapshot.turn, snapshot.battle_count),
        game_over: snapshot.is_over,
    })
}

/// POST /api/rpg/reset - RPGゲームをリセット
pub async fn rpg_reset(
    session: Session,
) -> impl IntoResponse {
    let mut service = RpgService::new();
    service.reset();
    session.insert(RPG_SESSION_KEY, &service).await.ok();
    
    tracing::info!(target: "web::rpg", "RPG game reset");
    StatusCode::NO_CONTENT
}
