use axum::{routing::{get, post}, Router};
use crate::handlers::swap_handler;
use crate::database::Database;

// 스왑 라우터 생성
// 역할: NestJS의 @Controller('swap') 같은 것
// Create swap router (State가 필요한 핸들러 포함)
pub fn create_swap_router() -> Router<Database> {
    Router::new()
        .route("/quote", get(swap_handler::get_quote))
        .route("/transaction", post(swap_handler::create_swap_transaction))
}
