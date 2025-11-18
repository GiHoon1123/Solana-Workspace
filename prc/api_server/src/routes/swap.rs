use axum::{routing::get, Router};
use crate::handlers::swap_handler;

// 스왑 라우터 생성
// 역할: NestJS의 @Controller('swap') 같은 것
// Create swap router
pub fn create_swap_router() -> Router {
    Router::new()
        .route("/quote", get(swap_handler::get_quote))
}

