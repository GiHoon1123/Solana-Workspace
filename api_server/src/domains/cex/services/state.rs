// CEX domain state
// CEX 도메인 상태
use std::sync::Arc;
use crate::shared::database::Database;
use crate::domains::cex::services::{BalanceService, FeeService, OrderService, TradeService};
use crate::domains::cex::engine::Engine;

/// CEX domain state
/// CEX 도메인에서 필요한 서비스들을 포함하는 상태
#[derive(Clone)]
pub struct CexState {
    pub balance_service: BalanceService,
    pub fee_service: FeeService,
    pub order_service: OrderService,
    pub trade_service: TradeService,
}

impl CexState {
    /// Create CexState with database and engine
    /// CexState 생성 (데이터베이스 + 엔진 필요)
    /// 
    /// # Arguments
    /// * `db` - 데이터베이스 연결
    /// * `engine` - 체결 엔진 (trait 객체)
    pub fn new(db: Database, engine: Arc<dyn Engine>) -> Self {
        Self {
            balance_service: BalanceService::new(db.clone()),
            fee_service: FeeService::new(db.clone()),
            order_service: OrderService::new(db.clone(), engine),
            trade_service: TradeService::new(db),
        }
    }
}

     