use crate::models::QuoteResponse;
use anyhow::{Context, Result};

// Jupiter API 클라이언트
// 역할: NestJS의 HttpClient나 axios 같은 것
// Jupiter API client for external calls
pub struct JupiterClient {
    http_client: reqwest::Client,
    base_url: String,
}

impl JupiterClient {
    // 클라이언트 생성
    // Create new Jupiter client instance
    pub fn new() -> Result<Self> {
        let http_client = reqwest::Client::builder()
            .danger_accept_invalid_certs(false)
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            http_client,
            base_url: "https://lite-api.jup.ag/ultra/v1".to_string(),
        })
    }

    // Quote 조회: Jupiter API 호출
    // Get quote: call Jupiter API
    // 역할: NestJS의 private async method 같은 것
    pub async fn get_quote(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
    ) -> Result<QuoteResponse> {
        // URL 생성
        // Build request URL
        let url = format!(
            "{}/quote?inputMint={}&outputMint={}&amount={}",
            self.base_url, input_mint, output_mint, amount
        );

        println!("Requesting Jupiter API: {}", url);  // 디버깅용 로그

        // HTTP GET 요청
        // HTTP GET request
        let response = self
            .http_client
            .get(&url)
            .header("User-Agent", "api-server/1.0")
            .send()
            .await
            .context("Failed to send request to Jupiter API")?;

        // HTTP 상태 코드 확인
        // Check HTTP status code
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Jupiter API returned error: {} - {}",
                status,
                body
            );
        }

        // JSON 파싱
        // Parse JSON response
        let quote: QuoteResponse = response
            .json()
            .await
            .context("Failed to parse Jupiter API response")?;

        Ok(quote)
    }
}

// 기본 구현: new() 메서드 제공
// Default implementation: provides new() method
impl Default for JupiterClient {
    fn default() -> Self {
        Self::new().expect("Failed to create JupiterClient")
    }
}

