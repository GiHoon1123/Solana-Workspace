use axum::Router;
use axum::http::Method;
use tokio::net::TcpListener;
use tower_http::cors::{CorsLayer, Any};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

// Routes import
mod routes;
mod handlers;
mod models;
mod clients;
mod database;
mod services;
mod errors;
mod middleware;  // Added middleware module

use routes::create_router;
use crate::models::{
    QuoteRequest, QuoteResponse, RoutePlan, SwapInfo, 
    TokenSearchRequest, TokenSearchResponse, Token,
    SwapTransactionRequest, SwapTransactionResponse, Transaction,
    SignupRequest, SignupResponse, SigninRequest, SigninResponse, UserResponse,
    CreateWalletResponse, WalletResponse, WalletsResponse,  // CreateWalletRequest 제거 (JWT 토큰에서 user_id 추출)
    BalanceResponse, TransferSolRequest, TransferSolResponse, TransactionStatusResponse,
    SolanaWallet,
};
use crate::handlers::{swap_handler, token_handler, auth_handler, wallet_handler};
use crate::database::Database;
use crate::services::AppState;

// OpenAPI 스키마 정의: Swagger 문서 자동 생성
#[derive(OpenApi)]
#[openapi(
    paths(
        swap_handler::get_quote,
        swap_handler::create_swap_transaction,
        token_handler::search_tokens,
        auth_handler::signup,
        auth_handler::signin,
        wallet_handler::create_wallet,
        wallet_handler::get_wallet,
        wallet_handler::get_user_wallets,
        wallet_handler::get_balance,
        wallet_handler::transfer_sol,
        wallet_handler::get_transaction_status
    ),
    components(schemas(
        QuoteRequest,
        QuoteResponse,
        RoutePlan,
        SwapInfo,
        TokenSearchRequest,
        TokenSearchResponse,
        Token,
        SwapTransactionRequest,
        SwapTransactionResponse,
        Transaction,
        SignupRequest,
        SignupResponse,
        SigninRequest,
        SigninResponse,
        UserResponse,
        CreateWalletResponse,  // CreateWalletRequest 제거 (JWT 토큰에서 user_id 추출)
        WalletResponse,
        WalletsResponse,
        BalanceResponse,
        TransferSolRequest,
        TransferSolResponse,
        TransactionStatusResponse,
        SolanaWallet
    )),
    modifiers(
        &SecurityAddon  // Security scheme 추가
    ),
    tags(
        (name = "Swap", description = "Swap API endpoints (Jupiter integration)"),
        (name = "Tokens", description = "Token search API endpoints"),
        (name = "Auth", description = "Authentication API endpoints"),
        (name = "Wallets", description = "Wallet API endpoints (Solana wallet management)")
    ),
    info(
        title = "Solana API Server",
        description = "API server for Solana blockchain interactions",
        version = "1.0.0"
    )
)]
struct ApiDoc;

// Security scheme 정의: Swagger UI에서 "Authorize" 버튼 추가
struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "BearerAuth",
                utoipa::openapi::security::SecurityScheme::Http(
                    utoipa::openapi::security::HttpBuilder::new()
                        .scheme(utoipa::openapi::security::HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            )
        }
    }
}

#[tokio::main]
async fn main() {
    // DB 연결
    let db_url = "postgresql://root:1234@localhost/solana_api";
    let db = Database::new(db_url)
        .await
        .expect("Failed to connect to database");

    db.initialize()
        .await
        .expect("Failed to initialize database");

    // AppState 생성 (모든 Service 초기화)
    // Create AppState (initialize all services)
    let app_state = AppState::new(db)
        .expect("Failed to initialize AppState");

    // CORS 설정
    // Allow requests from frontend (localhost:3003)
    // Note: allow_credentials(true)일 때 allow_origin을 *로 설정할 수 없음
    // 따라서 명시적인 origin을 지정해야 함
    use axum::http::HeaderValue;
    let cors = CorsLayer::new()
        .allow_origin("http://localhost:3003".parse::<HeaderValue>().unwrap())  // 프론트엔드 origin 명시
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
            axum::http::header::ACCEPT,
        ])
        .allow_credentials(true);  // JWT 토큰을 위한 credentials 허용

    // Router 생성 (AppState를 State로 사용)
    // Create router (uses AppState as State)
    // Axum 0.7에서는 with_state를 최상위에 하면 nested Router들에도 자동으로 전파됨
    let app = Router::new()
        .merge(create_router())
        .merge(
            SwaggerUi::new("/api")
                .url("/api-docs/openapi.json", ApiDoc::openapi())
        )
        .layer(cors)  // CORS 레이어 추가
        .with_state(app_state);  // AppState를 State로 - nested Router들에 자동 전파

    // 서버 시작: 3002 포트에서 리스닝
    let listener = TcpListener::bind("0.0.0.0:3002")
        .await
        .unwrap();
    
    println!("Server running on http://localhost:3002");
    println!("Swagger UI available at http://localhost:3002/api");
    println!("Database: PostgreSQL (solana_api)");
    
    // 서버 실행
    axum::serve(listener, app)
        .await
        .unwrap();
}