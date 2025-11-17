use crate::clients::JupiterClient;
use crate::models::{QuoteRequest, QuoteResponse};
use axum::{extract::Query, http::StatusCode, Json};
use serde_json::json;

// Jupiter 가격 조회 핸들러
// 역할: NestJS의 @Get() 핸들러 같은 것
// Handler: queries Jupiter for swap quotes
// 비즈니스 로직: 클라이언트를 호출해서 데이터 가져오기만 함
// utoipa::path: Swagger 문서 자동 생성용 어노테이션
// params의 example: Swagger UI에서 "Try it out" 시 미리 채워지는 값
#[utoipa::path(
    get,
    path = "/api/jupiter/quote",
    params(
        (
            "input_mint" = String, 
            Query, 
            description = "Input token mint address (e.g., USDC)",
            example = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
        ),
        (
            "output_mint" = String, 
            Query, 
            description = "Output token mint address (e.g., SOL)",
            example = "So11111111111111111111111111111111111111112"
        ),
        (
            "amount" = u64, 
            Query, 
            description = "Amount to swap (in lamports/minimal units)",
            example = 1000000
        )
    ),
    responses(
        (status = 200, description = "Quote retrieved successfully", body = QuoteResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error"),
        (status = 502, description = "Jupiter API error")
    ),
    tag = "Jupiter"
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