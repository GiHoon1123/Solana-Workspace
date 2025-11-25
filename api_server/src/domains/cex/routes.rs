// CEX domain routes
// CEX 도메인 라우터
use axum::{routing::get, Router};
use crate::domains::cex::handlers::balance_handler;
use crate::shared::services::AppState;

/// Create CEX router
/// CEX 라우터 생성
/// 
/// # Routes
/// * `GET /balances` - 모든 잔고 조회 (인증 필요)
/// * `GET /balances/{mint}` - 특정 자산 잔고 조회 (인증 필요)
pub fn create_cex_router() -> Router<AppState> {
    Router::new()
        // 잔고 관련 라우트
        // Balance routes
        .route("/balances", get(balance_handler::get_all_balances))
        .route("/balances/:mint", get(balance_handler::get_balance))
}

