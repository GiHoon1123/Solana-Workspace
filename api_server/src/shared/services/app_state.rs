use crate::shared::database::Database;
use crate::domains::auth::services::state::AuthState;
use crate::domains::wallet::services::state::WalletState;
use crate::domains::swap::services::state::SwapState;
use crate::domains::cex::services::state::CexState;
use crate::domains::auth::services::JwtService;
use anyhow::Result;

/// Application state (combines all domain states)
/// 애플리케이션 상태 (모든 도메인 상태를 조합)
/// 
/// 역할: NestJS의 Module에서 모든 Service를 주입하는 것과 유사
/// 각 도메인의 State를 조합하여 전체 애플리케이션 상태를 관리
#[derive(Clone)]
pub struct AppState {
    pub auth_state: AuthState,
    pub wallet_state: WalletState,
    pub swap_state: SwapState,
    pub cex_state: CexState,
}

impl AppState {
    /// Create AppState with database
    /// 모든 도메인 State를 초기화하고 조합
    pub fn new(db: Database) -> Result<Self> {
        // 1. 공유 서비스 생성 (JWT 등)
        let jwt_secret = std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| "your-secret-key-change-in-production".to_string());
        let jwt_service = JwtService::new(jwt_secret);

        // 2. 각 도메인 State 생성
        let auth_state = AuthState::new(db.clone(), jwt_service);
        let wallet_state = WalletState::new(db.clone())?;
        let swap_state = SwapState::new(db.clone());
        let cex_state = CexState::new(db.clone());

        // 3. AppState 조합
        Ok(Self {
            auth_state,
            wallet_state,
            swap_state,
            cex_state,
        })
    }
}
