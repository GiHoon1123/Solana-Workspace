// Routes module: 라우팅 설정
// 역할: NestJS의 controller 같은 것
pub mod swap;
pub mod tokens;  

use axum::Router;
use crate::database::Database;

// Router 생성 (State 타입 지정 필요 - swap router가 Database State 사용)
// Create router (needs State type - swap router uses Database State)
// tokens router도 State 타입 통일을 위해 Database State를 받음
pub fn create_router() -> Router<Database> {
    Router::new()
        .nest("/api/swap", swap::create_swap_router())
        .nest("/api/tokens", tokens::create_tokens_router())
}

  
