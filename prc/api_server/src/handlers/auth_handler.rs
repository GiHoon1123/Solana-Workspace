use crate::models::{SignupRequest, SignupResponse, SigninRequest, SigninResponse};
use crate::services::AppState;
use axum::{extract::State, http::StatusCode, Json};
use serde_json::json;

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
        .map_err(|e| {
            let status = if e.to_string().contains("already exists") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (
                status,
                Json(json!({
                    "error": e.to_string()
                })),
            )
        })?;

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
        .map_err(|e| {
            let status = if e.to_string().contains("Invalid") {
                StatusCode::UNAUTHORIZED
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (
                status,
                Json(json!({
                    "error": e.to_string()
                })),
            )
        })?;

    Ok(Json(SigninResponse {
        user: user.into(),
        message: "Login successful".to_string(),
    }))
}