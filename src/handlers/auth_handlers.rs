use axum::{
    extract::State,
    Json,
};
use crate::models::{LoginRequest, LoginResponse, AppResult};
use crate::handlers::comparison_handlers::AppState;

/// Authenticate with a URL and get session cookies
#[utoipa::path(
    post,
    path = "/xml-compare-api/api/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Authentication successful", body = LoginResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Authentication failed"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Authentication"
)]
pub async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> AppResult<Json<LoginResponse>> {
    let response = state.auth_service.login(&request).await?;
    Ok(Json(response))
}

/// Logout and invalidate session
#[utoipa::path(
    post,
    path = "/xml-compare-api/api/auth/logout/{session_id}",
    params(
        ("session_id" = String, Path, description = "Session ID to logout")
    ),
    responses(
        (status = 200, description = "Logout successful"),
        (status = 404, description = "Session not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Authentication"
)]
pub async fn logout(
    State(state): State<AppState>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> AppResult<Json<()>> {
    state.auth_service.logout(&session_id).await?;
    Ok(Json(()))
}