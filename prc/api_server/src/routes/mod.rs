// Routes module: 라우팅 설정
// 역할: NestJS의 controller 같은 것
pub mod swap;

use axum::Router;

pub fn create_router() -> Router {
    Router::new()
        .nest("/api/swap", swap::create_swap_router())
}