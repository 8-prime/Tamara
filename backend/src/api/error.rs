use crate::error::{Error, ErrorKind};
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let status = match self.0 {
            ErrorKind::BadRequest => StatusCode::BAD_REQUEST,
            ErrorKind::NotFound => StatusCode::NOT_FOUND,
            ErrorKind::Internal => StatusCode::INTERNAL_SERVER_ERROR,
            ErrorKind::Conflict => StatusCode::CONFLICT,
            ErrorKind::Unauthorized => StatusCode::UNAUTHORIZED,
            ErrorKind::Forbidden => StatusCode::FORBIDDEN,
            ErrorKind::TooManyRequests => StatusCode::TOO_MANY_REQUESTS,
            ErrorKind::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            ErrorKind::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
        };
        (status, Json(json!({"error": self.1}))).into_response()
    }
}
