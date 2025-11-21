use crate::database::Database;
use crate::services::{AuthService, SwapService, TokenService, WalletService, JwtService};
use anyhow::Result;

// 애플리케이션 상태 (모든 Service 포함)
// 역할: NestJS의 Module에서 모든 Service를 주입하는 것과 유사
// AppState: contains all services for dependency injection
#[derive(Clone)]
pub struct AppState {
    pub auth_service: AuthService,
    pub swap_service: SwapService,
    pub token_service: TokenService,
    pub wallet_service: WalletService,
    pub jwt_service: JwtService,
}

impl AppState {
    // AppState 생성 (모든 Service 초기화)
    pub fn new(db: Database) -> Result<Self> {
        // JWT Secret 가져오기 (환경변수 또는 기본값)
        let jwt_secret = std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| "your-secret-key-change-in-production".to_string());

        Ok(Self {
            auth_service: AuthService::new(db.clone()),
            swap_service: SwapService::new(db.clone()),
            token_service: TokenService::new(db.clone()),
            wallet_service: WalletService::new(db.clone())?,
            jwt_service: JwtService::new(jwt_secret),
        })
    }
}

