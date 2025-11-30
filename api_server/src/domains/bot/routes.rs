use axum::Router;
use crate::shared::services::AppState;
use crate::domains::bot::handlers::bot_handler;

/// Bot 라우터 생성
/// Create bot router
/// 
/// 봇 관련 API 엔드포인트를 제공합니다.
pub fn create_bot_router() -> Router<AppState> {
    Router::new()
        .route("/data", axum::routing::delete(bot_handler::delete_bot_data))
}

