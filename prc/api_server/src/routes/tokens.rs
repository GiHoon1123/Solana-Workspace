use axum::{routing::get, Router};
use crate::handlers::token_handler;
use crate::database::Database;

// 토큰 라우터 생성
// 역할: NestJS의 @Controller('tokens') 같은 것
// State 타입 통일을 위해 Database State를 받음 (실제로는 사용하지 않음)
pub fn create_tokens_router() -> Router<Database> {
    Router::new()
        .route("/search", get(token_handler::search_tokens))
}