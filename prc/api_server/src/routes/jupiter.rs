use axum::{routing::get, Router};
use crate::handlers::jupiter_handler;

// Jupiter 라우터 생성
// 역할: NestJS의 @Controller('jupiter') 같은 것
// Create Jupiter router
pub fn create_jupiter_router() -> Router {
    Router::new()
        .route("/quote", get(jupiter_handler::get_quote))
}