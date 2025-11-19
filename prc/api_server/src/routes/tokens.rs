use axum::{routing::get, Router};
use crate::handlers::token_handler;

// 토큰 라우터 생성
// 역할: NestJS의 @Controller('tokens') 같은 것
pub fn create_tokens_router() -> Router {
    Router::new()
        .route("/search", get(token_handler::search_tokens))
}