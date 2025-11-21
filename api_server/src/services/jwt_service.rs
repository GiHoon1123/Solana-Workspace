// src/services/jwt_service.rs
use crate::errors::AuthError;
use crate::models::jwt::Claims;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};

/// JWT 서비스
/// JWT Service for token generation and verification
#[derive(Clone)]
pub struct JwtService {
    secret: String,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JwtService {
    /// JWT Service 생성
    /// Create JWT Service
    pub fn new(secret: String) -> Self {
        let encoding_key = EncodingKey::from_secret(secret.as_ref());
        let decoding_key = DecodingKey::from_secret(secret.as_ref());

        Self {
            secret,
            encoding_key,
            decoding_key,
        }
    }

    /// JWT 토큰 발급
    /// Generate JWT token
    pub fn generate_token(&self, user_id: u64, email: String) -> Result<String, AuthError> {
        let claims = Claims::new(user_id, email, 24); // 24시간 만료

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| AuthError::Internal(format!("Failed to generate token: {}", e)))
    }

    /// JWT 토큰 검증
    /// Verify JWT token
    pub fn verify_token(&self, token: &str) -> Result<Claims, AuthError> {
        let validation = Validation::default();

        let token_data = decode::<Claims>(token, &self.decoding_key, &validation)
            .map_err(|e| {
                match e.kind() {
                    jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                        AuthError::InvalidToken // 만료된 토큰
                    }
                    _ => AuthError::InvalidToken,
                }
            })?;

        Ok(token_data.claims)
    }
}