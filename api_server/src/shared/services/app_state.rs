use std::sync::Arc;
use crate::shared::database::Database;
use crate::domains::auth::services::state::AuthState;
use crate::domains::wallet::services::state::WalletState;
use crate::domains::swap::services::state::SwapState;
use crate::domains::cex::services::state::CexState;
use crate::domains::cex::engine::{Engine, runtime::HighPerformanceEngine};
use crate::domains::auth::services::JwtService;
use anyhow::Result;
use tokio::sync::Mutex;

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
    /// 엔진 인스턴스 (시작/정지용)
    /// Engine instance (for start/stop)
    pub engine: Arc<Mutex<HighPerformanceEngine>>,
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
        
        // CEX 엔진 생성 (HighPerformanceEngine 사용)
        let engine_instance = HighPerformanceEngine::new(db.clone());
        let engine = Arc::new(Mutex::new(engine_instance));
        
        // 서비스용 엔진 (trait 객체로 변환)
        // Arc<Mutex<HighPerformanceEngine>>를 Arc<dyn Engine>으로 변환하기 위해
        // 엔진을 다시 생성하거나, wrapper를 사용
        // 일단 간단하게: 엔진을 두 번 생성 (나중에 최적화 가능)
        let engine_for_service: Arc<dyn Engine> = Arc::new(HighPerformanceEngine::new(db.clone()));
        let cex_state = CexState::new(db.clone(), engine_for_service);
        
        // 3. AppState 조합
        Ok(Self {
            auth_state,
            wallet_state,
            swap_state,
            cex_state,
            engine,
        })
    }
}
