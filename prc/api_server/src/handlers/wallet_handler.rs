use crate::models::{
    CreateWalletRequest, CreateWalletResponse, WalletResponse, WalletsResponse,
    BalanceResponse, TransferSolRequest, TransferSolResponse, TransactionStatusResponse,
};
use crate::services::AppState;
use axum::{extract::{Path, State}, http::StatusCode, Json};
use serde_json::json;

/// 지갑 생성 핸들러
/// Create wallet handler
#[utoipa::path(
    post,
    path = "/api/wallets",
    request_body = CreateWalletRequest,
    responses(
        (status = 201, description = "Wallet created successfully", body = CreateWalletResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Wallets"
)]
pub async fn create_wallet(
    State(app_state): State<AppState>,
    Json(request): Json<CreateWalletRequest>,
) -> Result<Json<CreateWalletResponse>, (StatusCode, Json<serde_json::Value>)> {
    let wallet = app_state
        .wallet_service
        .create_wallet(request.user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": format!("Failed to create wallet: {}", e)
                })),
            )
        })?;

    Ok(Json(CreateWalletResponse {
        wallet,
        message: "Wallet created successfully".to_string(),
    }))
}

/// 지갑 조회 핸들러 (ID로)
/// Get wallet by ID handler
#[utoipa::path(
    get,
    path = "/api/wallets/{id}",
    params(
        ("id" = u64, Path, description = "Wallet ID")
    ),
    responses(
        (status = 200, description = "Wallet retrieved successfully", body = WalletResponse),
        (status = 404, description = "Wallet not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Wallets"
)]
pub async fn get_wallet(
    State(app_state): State<AppState>,
    Path(wallet_id): Path<u64>,
) -> Result<Json<WalletResponse>, (StatusCode, Json<serde_json::Value>)> {
    let wallet = app_state
        .wallet_service
        .get_wallet(wallet_id)
        .await
        .map_err(|e| {
            let status = if e.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (
                status,
                Json(json!({
                    "error": format!("Failed to get wallet: {}", e)
                })),
            )
        })?;

    Ok(Json(WalletResponse { wallet }))
}

/// 사용자의 모든 지갑 조회 핸들러
/// Get all wallets for user handler
#[utoipa::path(
    get,
    path = "/api/wallets/user/{user_id}",
    params(
        ("user_id" = u64, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "Wallets retrieved successfully", body = WalletsResponse),
        (status = 500, description = "Internal server error")
    ),
    tag = "Wallets"
)]
pub async fn get_user_wallets(
    State(app_state): State<AppState>,
    Path(user_id): Path<u64>,
) -> Result<Json<WalletsResponse>, (StatusCode, Json<serde_json::Value>)> {
    let wallets = app_state
        .wallet_service
        .get_user_wallets(user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": format!("Failed to get user wallets: {}", e)
                })),
            )
        })?;

    Ok(Json(WalletsResponse { wallets }))
}

/// 잔액 조회 핸들러
/// Get balance handler
#[utoipa::path(
    get,
    path = "/api/wallets/{id}/balance",
    params(
        ("id" = u64, Path, description = "Wallet ID")
    ),
    responses(
        (status = 200, description = "Balance retrieved successfully", body = BalanceResponse),
        (status = 404, description = "Wallet not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Wallets"
)]
pub async fn get_balance(
    State(app_state): State<AppState>,
    Path(wallet_id): Path<u64>,
) -> Result<Json<BalanceResponse>, (StatusCode, Json<serde_json::Value>)> {
    // 지갑 조회 (Public Key 가져오기)
    let wallet = app_state
        .wallet_service
        .get_wallet(wallet_id)
        .await
        .map_err(|e| {
            let status = if e.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (
                status,
                Json(json!({
                    "error": format!("Failed to get wallet: {}", e)
                })),
            )
        })?;

    // 잔액 조회
    let balance_lamports = app_state
        .wallet_service
        .get_balance(wallet_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": format!("Failed to get balance: {}", e)
                })),
            )
        })?;

    let balance_sol = app_state
        .wallet_service
        .get_balance_sol(wallet_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": format!("Failed to get balance: {}", e)
                })),
            )
        })?;

    Ok(Json(BalanceResponse {
        balance_lamports,
        balance_sol,
        public_key: wallet.public_key,
    }))
}

/// SOL 전송 핸들러
/// Transfer SOL handler
#[utoipa::path(
    post,
    path = "/api/wallets/{id}/transfer",
    params(
        ("id" = u64, Path, description = "Wallet ID (sender)")
    ),
    request_body = TransferSolRequest,
    responses(
        (status = 200, description = "Transfer successful", body = TransferSolResponse),
        (status = 400, description = "Bad request"),
        (status = 404, description = "Wallet not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Wallets"
)]
pub async fn transfer_sol(
    State(app_state): State<AppState>,
    Path(wallet_id): Path<u64>,
    Json(request): Json<TransferSolRequest>,
) -> Result<Json<TransferSolResponse>, (StatusCode, Json<serde_json::Value>)> {
    let signature = app_state
        .wallet_service
        .transfer_sol(wallet_id, &request.to_public_key, request.amount_lamports)
        .await
        .map_err(|e| {
            let status = if e.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else if e.to_string().contains("insufficient") || e.to_string().contains("balance") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (
                status,
                Json(json!({
                    "error": format!("Failed to transfer SOL: {}", e)
                })),
            )
        })?;

    Ok(Json(TransferSolResponse {
        signature,
        message: "Transfer successful".to_string(),
    }))
}

/// 트랜잭션 상태 조회 핸들러
/// Get transaction status handler
#[utoipa::path(
    get,
    path = "/api/wallets/transaction/{signature}",
    params(
        ("signature" = String, Path, description = "Transaction signature")
    ),
    responses(
        (status = 200, description = "Transaction status retrieved successfully", body = TransactionStatusResponse),
        (status = 500, description = "Internal server error")
    ),
    tag = "Wallets"
)]
pub async fn get_transaction_status(
    State(app_state): State<AppState>,
    Path(signature): Path<String>,
) -> Result<Json<TransactionStatusResponse>, (StatusCode, Json<serde_json::Value>)> {
    let status = app_state
        .wallet_service
        .get_transaction_status(&signature)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": format!("Failed to get transaction status: {}", e)
                })),
            )
        })?;

    Ok(Json(TransactionStatusResponse {
        signature,
        status,
    }))
}

