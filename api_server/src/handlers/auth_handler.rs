use crate::models::{SignupRequest, SignupResponse, SigninRequest, SigninResponse};
use crate::services::AppState;
use crate::errors::AuthError;
use axum::{extract::State, http::StatusCode, Json};

#[utoipa::path(
    post,
    path = "/api/auth/signup",
    request_body = SignupRequest,
    responses(
        (status = 201, description = "User created successfully", body = SignupResponse),
        (status = 400, description = "Bad request (email already exists)"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Auth"
)]
pub async fn signup(
    State(app_state): State<AppState>,
    Json(request): Json<SignupRequest>,
) -> Result<Json<SignupResponse>, (StatusCode, Json<serde_json::Value>)> {
    // Service 호출 (비즈니스 로직)
    let user = app_state
        .auth_service
        .signup(request)
        .await
        .map_err(|e: AuthError| -> (StatusCode, Json<serde_json::Value>) { e.into() })?;

    Ok(Json(SignupResponse {
        user: user.into(),
        message: "User created successfully".to_string(),
    }))
}

// 로그인 핸들러
#[utoipa::path(
    post,
    path = "/api/auth/signin",
    request_body = SigninRequest,
    responses(
        (status = 200, description = "Login successful", body = SigninResponse),
        (status = 401, description = "Invalid email or password"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Auth"
)]
pub async fn signin(
    State(app_state): State<AppState>,
    Json(request): Json<SigninRequest>,
) -> Result<Json<SigninResponse>, (StatusCode, Json<serde_json::Value>)> {
    // Service 호출 (비즈니스 로직)
    let user = app_state
        .auth_service
        .signin(request)
        .await
        .map_err(|e: AuthError| -> (StatusCode, Json<serde_json::Value>) { e.into() })?;

    // JWT 토큰 발급
    let access_token = app_state
        .jwt_service
        .generate_token(user.id, user.email.clone())
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Failed to generate token: {}", e) })),
            )
        })?;

    Ok(Json(SigninResponse {
        user: user.into(),
        access_token,
        message: "Login successful".to_string(),
    }))
}