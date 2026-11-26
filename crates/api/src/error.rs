//! Chuyển đổi `AppError` sang HTTP response JSON chuẩn hóa.

use axum::{
    Json,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use pum_core::AppError;
use serde::Serialize;

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug)]
pub struct ApiError(pub AppError);

impl From<AppError> for ApiError {
    fn from(err: AppError) -> Self {
        Self(err)
    }
}

#[derive(Serialize)]
struct ErrorEnvelope<'a> {
    error: ErrorDetail<'a>,
}

#[derive(Serialize)]
struct ErrorDetail<'a> {
    code: &'a str,
    message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, retry_after) = match &self.0 {
            AppError::Validation(_) => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR", None),
            AppError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", None),
            AppError::Forbidden(_) => (StatusCode::FORBIDDEN, "FORBIDDEN", None),
            AppError::NotFound(_) => (StatusCode::NOT_FOUND, "NOT_FOUND", None),
            AppError::Conflict(_) => (StatusCode::CONFLICT, "CONFLICT", None),
            AppError::Gone(_) => (StatusCode::GONE, "GONE", None),
            AppError::RateLimited { retry_after_secs } => (
                StatusCode::TOO_MANY_REQUESTS,
                "RATE_LIMITED",
                Some(*retry_after_secs),
            ),
            AppError::SecurityViolation(_) => {
                (StatusCode::UNPROCESSABLE_ENTITY, "SECURITY_VIOLATION", None)
            }
            AppError::Internal(msg) => {
                tracing::error!(error = %msg, "internal server error");
                (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", None)
            }
        };

        let message = match &self.0 {
            AppError::Internal(_) => {
                "đã xảy ra lỗi hệ thống nội bộ, vui lòng thử lại sau".to_owned()
            }
            other => other.to_string(),
        };

        let mut response = (
            status,
            Json(ErrorEnvelope {
                error: ErrorDetail { code, message },
            }),
        )
            .into_response();
        if let Some(secs) = retry_after
            && let Ok(val) = HeaderValue::from_str(&secs.to_string())
        {
            response.headers_mut().insert(header::RETRY_AFTER, val);
        }
        response
    }
}
