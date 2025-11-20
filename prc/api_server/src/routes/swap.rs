use axum::{routing::{get, post}, Router};
use crate::handlers::swap_handler;
use crate::services::AppState;

// 스왑 라우터 생성
// 역할: NestJS의 @Controller('swap') 같은 것
// Create swap router (uses AppState as State)
pub fn create_swap_router() -> Router<AppState> {
    Router::new()
        .route("/quote", get(swap_handler::get_quote))
        .route("/transaction", post(swap_handler::create_swap_transaction))
}
