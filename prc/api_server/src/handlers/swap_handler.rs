use crate::clients::JupiterClient;
use crate::models::{QuoteRequest, QuoteResponse, SwapTransactionRequest, SwapTransactionResponse};
use crate::database::Database;
use crate::database::TransactionRepository;
use crate::models::TransactionStatus;
use axum::{extract::Query, extract::State, http::StatusCode, Json};
use serde_json::json;

// 스왑 가격 조회 핸들러
// 역할: NestJS의 @Get() 핸들러 같은 것
// Handler: queries Jupiter for swap quotes
// 비즈니스 로직: 클라이언트를 호출해서 데이터 가져오기만 함
// utoipa::path: Swagger 문서 자동 생성용 어노테이션
// Note: 예시값은 모델(QuoteRequest)에서 관리
// Note: example values are managed in model (QuoteRequest)
#[utoipa::path(
    get,
    path = "/api/swap/quote",
    params(QuoteRequest),
    responses(
        (status = 200, description = "Quote retrieved successfully", body = QuoteResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error"),
        (status = 502, description = "Jupiter API error")
    ),
    tag = "Swap"
)]
pub async fn get_quote(
    Query(params): Query<QuoteRequest>,
) -> Result<Json<QuoteResponse>, (StatusCode, Json<serde_json::Value>)> {
    // Jupiter 클라이언트 생성 (또는 싱글톤으로 관리 가능)
    // Create Jupiter client (can be managed as singleton)
    let jupiter_client = JupiterClient::new().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to create Jupiter client: {}", e)})),
        )
    })?;

    // 클라이언트를 통해서 외부 API 호출
    // Call external API through client
    let quote = jupiter_client
        .get_quote(&params.input_mint, &params.output_mint, params.amount)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                Json(json!({
                    "error": format!("Failed to fetch quote from Jupiter: {}", e)
                })),
            )
        })?;

    Ok(Json(quote))
}


// 스왑 트랜잭션 생성 핸들러
// 역할: NestJS의 @Post() 핸들러 같은 것
// Handler: creates swap transaction via Jupiter API and saves to DB
#[utoipa::path(
    post,
    path = "/api/swap/transaction",
    request_body = SwapTransactionRequest,
    responses(
        (status = 200, description = "Swap transaction created successfully", body = SwapTransactionResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error"),
        (status = 502, description = "Jupiter API error")
    ),
    tag = "Swap"
)]
pub async fn create_swap_transaction(
    State(db): State<Database>,
    Json(request): Json<SwapTransactionRequest>,
) -> Result<Json<SwapTransactionResponse>, (StatusCode, Json<serde_json::Value>)> {
    // 1. Jupiter 클라이언트 생성
    // Create Jupiter client
    let jupiter_client = JupiterClient::new().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to create Jupiter client: {}", e)})),
        )
    })?;

    // 2. Quote 조회
    // Get quote
    let quote = jupiter_client
        .get_quote(&request.input_mint, &request.output_mint, request.amount)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                Json(json!({
                    "error": format!("Failed to fetch quote from Jupiter: {}", e)
                })),
            )
        })?;

    // 3. Swap 트랜잭션 생성
    // Create swap transaction
    let mut swap_response = jupiter_client
        .create_swap_transaction(&request, &quote)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                Json(json!({
                    "error": format!("Failed to create swap transaction from Jupiter: {}", e)
                })),
            )
        })?;

    // 4. 예상 출력 금액 파싱 (outAmount를 u64로 변환)
    // Parse expected output amount (convert outAmount to u64)
    let expected_out_amount = quote.out_amount.parse::<u64>().ok();

    // 5. Quote 응답을 JSON으로 변환
    // Convert quote response to JSON
    let quote_json = serde_json::to_value(&quote).ok();

    // 6. 레포지토리 생성
    // Create repository
    let repo = TransactionRepository::new(db.pool().clone());

    // 7. DB에 트랜잭션 저장
    // Save transaction to database
    let saved_transaction = repo
        .save_transaction(
            &request.input_mint,
            &request.output_mint,
            request.amount,  // u64
            expected_out_amount,  // Option<u64>
            &request.user_public_key,
            &swap_response.swap_transaction,
            quote_json,
            TransactionStatus::Created,
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": format!("Failed to save transaction to database: {}", e)
                })),
            )
        })?;

    // 8. 응답에 생성된 ID 설정
    // Set generated ID in response
    swap_response.id = Some(saved_transaction.id);

    Ok(Json(swap_response))
}

