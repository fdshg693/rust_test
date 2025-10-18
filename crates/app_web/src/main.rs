mod handlers;
mod models;

use axum::{
    routing::{get, post, delete},
    Router,
};
use tower_sessions::{MemoryStore, SessionManagerLayer, cookie::time::Duration};
use color_eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // エラーハンドリングの初期化
    color_eyre::install()?;

    // 環境変数のロード
    dotenvy::dotenv().ok();

    // ロギングの初期化
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,app_web=debug"))
        )
        .init();

    tracing::info!(target: "app_web", "Starting web server...");

    // セッションストアの設定
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_expiry(tower_sessions::Expiry::OnInactivity(Duration::seconds(3600)));

    // ルーティング設定
    let app = Router::new()
        // ページルート
        .route("/", get(handlers::home))
        .route("/chat", get(handlers::chat_page))
        .route("/rpg", get(handlers::rpg_page))
        
        // Chat APIルート
        .route("/api/chat", post(handlers::chat_api))
        .route("/api/chat/history", get(handlers::chat_history))
        .route("/api/chat/history", delete(handlers::clear_chat_history))
        
        // RPG APIルート
        .route("/api/rpg/action", post(handlers::rpg_action))
        .route("/api/rpg/state", get(handlers::rpg_state))
        .route("/api/rpg/reset", post(handlers::rpg_reset))
        
        // セッションレイヤー適用
        .layer(session_layer);

    // サーバー起動
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await?;
    
    let addr = listener.local_addr()?;
    tracing::info!(target: "app_web", "🚀 Server running on http://{}", addr);
    println!("🚀 Server running on http://{}", addr);
    println!("   📱 Home: http://localhost:3000");
    println!("   💬 Chat: http://localhost:3000/chat");
    println!("   🎮 RPG:  http://localhost:3000/rpg");

    axum::serve(listener, app).await?;
    
    Ok(())
}
