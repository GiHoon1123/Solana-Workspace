use axum::{routing::get, Router};
use crate::handlers::token_handler;
use crate::services::AppState;

// 토큰 라우터 생성
// 역할: NestJS의 @Controller('tokens') 같은 것
// Create tokens router (uses AppState as State)
pub fn create_tokens_router() -> Router<AppState> {
    Router::new()
        .route("/search", get(token_handler::search_tokens))
}