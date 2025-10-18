use askama::Template;
use axum::{
    response::{Html, IntoResponse},
    http::StatusCode,
};

/// ホームページテンプレート
#[derive(Template)]
#[template(path = "home.html")]
pub struct HomeTemplate;

/// チャットページテンプレート
#[derive(Template)]
#[template(path = "chat.html")]
pub struct ChatTemplate;

/// RPGページテンプレート
#[derive(Template)]
#[template(path = "rpg.html")]
pub struct RpgTemplate;

/// GET / - ホームページ（メニュー）
pub async fn home() -> impl IntoResponse {
    let template = HomeTemplate;
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Template error: {}", e),
        )
            .into_response(),
    }
}

/// GET /chat - チャットページ
pub async fn chat_page() -> impl IntoResponse {
    let template = ChatTemplate;
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Template error: {}", e),
        )
            .into_response(),
    }
}

/// GET /rpg - RPGゲームページ
pub async fn rpg_page() -> impl IntoResponse {
    let template = RpgTemplate;
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Template error: {}", e),
        )
            .into_response(),
    }
}
