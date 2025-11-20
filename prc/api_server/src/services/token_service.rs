use crate::clients::JupiterClient;
use crate::models::TokenSearchResponse;
use anyhow::{Context, Result};

// 토큰 검색 서비스
// 역할: NestJS의 Service 같은 것
// TokenService: handles token search business logic
pub struct TokenService;

impl TokenService {
    // 토큰 검색 (비즈니스 로직)
    // Search tokens (business logic)
    pub async fn search_tokens(query: &str) -> Result<TokenSearchResponse> {
        let jupiter_client = JupiterClient::new()
            .context("Failed to create Jupiter client")?;

        let search_result = jupiter_client
            .search_tokens(query)
            .await
            .context("Failed to search tokens from Jupiter")?;

        Ok(search_result)
    }
}

