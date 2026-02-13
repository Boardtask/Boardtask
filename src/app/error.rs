use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use sqlx::Error as SqlxError;

/// Application error type for unified error handling across the app.
#[derive(Debug)]
pub enum AppError {
    /// Validation errors (400 Bad Request) - invalid input data
    Validation(String),

    /// Authentication errors (400 Bad Request) - wrong credentials, etc.
    Auth(String),

    /// Database errors (500 Internal Server Error)
    Database(SqlxError),

    /// Generic internal errors (500 Internal Server Error)
    Internal,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::Validation(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Auth(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Database(err) => {
                tracing::error!(%err, "database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
            AppError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
        };

        let body = Json(json!({
            "error": message
        }));

        (status, body).into_response()
    }
}