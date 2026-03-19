use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug)]
pub enum Error {
    Internal(String),
    NotFound(String),
    BadRequest(String),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Error::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            Error::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            Error::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
        };

        let body = Json(json!({
            "error": message
        }));

        (status, body).into_response()
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Internal(msg) => write!(f, "Internal error: {}", msg),
            Error::NotFound(msg) => write!(f, "Not found: {}", msg),
            Error::BadRequest(msg) => write!(f, "Bad request: {}", msg),
        }
    }
}

impl std::error::Error for Error {}
