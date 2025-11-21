// 인증 라우터
// Authentication router
use axum::{routing::post, Router};
use crate::handlers::auth_handler;
use crate::services::AppState;

// 인증 라우터 생성
// Create authentication router
pub fn create_auth_router() -> Router<AppState> {
    Router::new()
        .route("/signup", post(auth_handler::signup))
        .route("/signin", post(auth_handler::signin))
}

