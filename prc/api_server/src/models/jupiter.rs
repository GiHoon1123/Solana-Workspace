use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;

// Jupiter Quote API 응답 모델
// 역할: NestJS의 interface 같은 것
// Note: Jupiter API는 camelCase로 응답하므로 #[serde(rename = "...")]로 매핑
// utoipa::ToSchema: Swagger 문서 자동 생성용
// schema(as = "QuoteResponse"): Swagger에서 사용할 스키마 이름
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(as = QuoteResponse)]
pub struct QuoteResponse {
    #[serde(rename = "inAmount")]
    pub in_amount: String,
    #[serde(rename = "outAmount")]
    pub out_amount: String,
    #[serde(rename = "otherAmountThreshold")]
    pub other_amount_threshold: String,
    #[serde(rename = "swapMode")]
    pub swap_mode: String,
    #[serde(rename = "slippageBps")]
    pub slippage_bps: i32,
    #[serde(rename = "priceImpactPct")]
    pub price_impact_pct: Option<String>,
    #[serde(rename = "routePlan")]
    pub route_plan: Vec<RoutePlan>,
    // 추가 필드들 (optional로 처리)
    #[serde(rename = "inputMint")]
    pub input_mint: Option<String>,
    #[serde(rename = "outputMint")]
    pub output_mint: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(as = RoutePlan)]
pub struct RoutePlan {
    #[serde(rename = "swapInfo")]
    pub swap_info: SwapInfo,
    pub percent: i32,
    // 추가 필드들
    pub bps: Option<i32>,
    #[serde(rename = "usdValue")]
    pub usd_value: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(as = SwapInfo)]
pub struct SwapInfo {
    #[serde(rename = "ammKey")]
    pub amm_key: String,
    pub label: String,
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    #[serde(rename = "inAmount")]
    pub in_amount: String,
    #[serde(rename = "outAmount")]
    pub out_amount: String,
    #[serde(rename = "feeAmount")]
    pub fee_amount: String,
    #[serde(rename = "feeMint")]
    pub fee_mint: String,
}

// Jupiter Quote API 요청 파라미터
// utoipa::ToSchema: Swagger 문서 자동 생성용
// example: Swagger UI에서 예시값으로 표시
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(as = QuoteRequest, example = json!({
    "input_mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
    "output_mint": "So11111111111111111111111111111111111111112",
    "amount": 1000000
}))]
pub struct QuoteRequest {
    /// Input token mint address (e.g., USDC)
    /// 입력 토큰 주소 (예: USDC)
    #[schema(example = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")]
    pub input_mint: String,
    
    /// Output token mint address (e.g., SOL)
    /// 출력 토큰 주소 (예: SOL)
    #[schema(example = "So11111111111111111111111111111111111111112")]
    pub output_mint: String,
    
    /// Amount to swap (in lamports/minimal units)
    /// 스왑할 수량 (lamports/최소 단위)
    #[schema(example = 1000000)]
    pub amount: u64,
    
    /// Slippage in basis points (optional)
    /// 슬리피지 (기본점 단위, 선택사항)
    pub slippage_bps: Option<i32>,
}