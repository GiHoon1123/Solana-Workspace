// 지갑 라우터
// Wallet router
use axum::{routing::{get, post}, Router};
use crate::handlers::wallet_handler;
use crate::services::AppState;

// 지갑 라우터 생성
// Create wallet router
pub fn create_wallet_router() -> Router<AppState> {
    Router::new()
        .route("/", post(wallet_handler::create_wallet))  // 인증 필요
        .route("/:id", get(wallet_handler::get_wallet))
        .route("/my", get(wallet_handler::get_user_wallets))  // 인증 필요, 경로 변경: /user/:user_id -> /my
        .route("/:id/balance", get(wallet_handler::get_balance))
        .route("/:id/transfer", post(wallet_handler::transfer_sol))  // 인증 필요
        .route("/transaction/:signature", get(wallet_handler::get_transaction_status))
}

