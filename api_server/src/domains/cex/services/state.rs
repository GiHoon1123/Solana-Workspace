// CEX domain state
// CEX 도메인 상태
use crate::shared::database::Database;
use crate::domains::cex::services::{BalanceService, FeeService};

/// CEX domain state
/// CEX 도메인에서 필요한 서비스들을 포함하는 상태
#[derive(Clone)]
pub struct CexState {
    pub balance_service: BalanceService,
    pub fee_service: FeeService,
}

impl CexState {
    /// Create CexState with database
    /// CexState 생성 (데이터베이스 필요)
    pub fn new(db: Database) -> Self {
        Self {
            balance_service: BalanceService::new(db.clone()),
            fee_service: FeeService::new(db),
        }
    }
}

     