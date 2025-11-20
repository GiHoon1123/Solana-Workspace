// Routes module: 라우팅 설정
// 역할: NestJS의 controller 같은 것
pub mod swap;
pub mod tokens;

use axum::Router;
use crate::services::AppState;

// Router 생성 (AppState를 State로 사용)
// Create router (uses AppState as State)
pub fn create_router() -> Router<AppState> {
    Router::new()
        .nest("/api/swap", swap::create_swap_router())
        .nest("/api/tokens", tokens::create_tokens_router())
}

  
