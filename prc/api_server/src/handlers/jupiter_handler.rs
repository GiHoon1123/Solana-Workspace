use crate::clients::JupiterClient;
use crate::models::{QuoteRequest, QuoteResponse};
use axum::{extract::Query, http::StatusCode, Json};
use serde_json::json;

// Jupiter 가격 조회 핸들러
// 역할: NestJS의 @Get() 핸들러 같은 것
// Handler: queries Jupiter for swap quotes
// 비즈니스 로직: 클라이언트를 호출해서 데이터 가져오기만 함
// utoipa::path: Swagger 문서 자동 생성용 어노테이션
// Note: 예시값은 모델(QuoteRequest)에서 관리
// Note: example values are managed in model (QuoteRequest)
#[utoipa::path(
    get,
    path = "/api/jupiter/quote",
    params(QuoteRequest),
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