// Routes module: 라우팅 설정
// 역할: NestJS의 controller 같은 것
pub mod jupiter;

use axum::Router;

pub fn create_router() -> Router {
    Router::new()
        .nest("/api/jupiter", jupiter::create_jupiter_router())
}