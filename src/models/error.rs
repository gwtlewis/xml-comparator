use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("XML parsing error: {0}")]
    XmlParseError(String),
    
    #[error("HTTP request error: {0}")]
    HttpError(#[from] reqwest::Error),
    
    #[error("Authentication failed: {0}")]
    AuthError(String),
    
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
    
    #[error("Internal server error: {0}")]
    InternalError(String),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::XmlParseError(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            AppError::HttpError(_) => (StatusCode::BAD_GATEWAY, "Failed to fetch XML from URL".to_string()),
            AppError::AuthError(_) => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::InvalidUrl(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            AppError::ValidationError(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            AppError::InternalError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string()),
        };

        let body = Json(json!({
            "error": error_message,
            "status": status.as_u16()
        }));

        (status, body).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;