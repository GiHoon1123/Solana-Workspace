use axum::Router;
use axum::http::Method;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

// New module structure
mod domains;
mod shared;
mod routes;

use routes::create_router;
use crate::shared::database::Database;
use crate::shared::services::AppState;

// Import models for OpenAPI schema
use crate::domains::swap::models::*;
use crate::domains::auth::models::*;
use crate::domains::wallet::models::*;

// OpenAPI 스키마 정의: Swagger 문서 자동 생성
#[derive(OpenApi)]
#[openapi(
    paths(
        crate::domains::swap::handlers::swap_handler::get_quote,
        crate::domains::swap::handlers::swap_handler::create_swap_transaction,
        crate::domains::swap::handlers::token_handler::search_tokens,
        crate::domains::auth::handlers::auth_handler::signup,
        crate::domains::auth::handlers::auth_handler::signin,
        crate::domains::auth::handlers::auth_handler::refresh,
        crate::domains::auth::handlers::auth_handler::logout,
        crate::domains::auth::handlers::auth_handler::get_me,
        crate::domains::wallet::handlers::wallet_handler::create_wallet,
        crate::domains::wallet::handlers::wallet_handler::get_wallet,
        crate::domains::wallet::handlers::wallet_handler::get_user_wallets,
        crate::domains::wallet::handlers::wallet_handler::get_balance,
        crate::domains::wallet::handlers::wallet_handler::transfer_sol,
        crate::domains::wallet::handlers::wallet_handler::get_transaction_status
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
        RefreshTokenRequest,
        RefreshTokenResponse,
        LogoutRequest,
        UserResponse,
        CreateWalletResponse,
        WalletResponse,
        WalletsResponse,
        WalletBalanceResponse,
        TransferSolRequest,
        TransferSolResponse,
        TransactionStatusResponse,
        SolanaWallet
    )),
    modifiers(
        &SecurityAddon
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
    let app_state = AppState::new(db)
        .expect("Failed to initialize AppState");

    // CORS 설정
    use axum::http::HeaderValue;
    let cors = CorsLayer::new()
        .allow_origin("http://localhost:3003".parse::<HeaderValue>().unwrap())
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
        .allow_credentials(true);

    // Router 생성
    let app = Router::new()
        .merge(create_router())
        .merge(
            SwaggerUi::new("/api")
                .url("/api-docs/openapi.json", ApiDoc::openapi())
        )
        .layer(cors)
        .with_state(app_state);

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
