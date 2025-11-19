use crate::clients::JupiterClient;
use crate::models::{TokenSearchRequest, TokenSearchResponse};
use axum::{extract::Query, http::StatusCode, Json};
use serde_json::json;

// 토큰 검색 핸들러
// 역할: NestJS의 @Get() 핸들러 같은 것
#[utoipa::path(
    get,
    path = "/api/tokens/search",
    params(TokenSearchRequest),
    responses(
        (status = 200, description = "Tokens retrieved successfully", body = TokenSearchResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error"),
        (status = 502, description = "Jupiter API error")
    ),
    tag = "Tokens"
)]
pub async fn search_tokens(
    Query(params): Query<TokenSearchRequest>,
) -> Result<Json<TokenSearchResponse>, (StatusCode, Json<serde_json::Value>)> {
    
    // Jupiter 클라이언트 생성
    let jupiter_client = JupiterClient::new().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to create Jupiter client: {}", e)})),
        )
    })?;


    // 토큰 검색 API 호출
    let search_result = jupiter_client
    .search_tokens(&params.query)
    .await
    .map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            Json(json!({
                "error": format!("Failed to search tokens from Jupiter: {}", e)
            })),
        )
    })?;

    Ok(Json(search_result))
}