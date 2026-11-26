//! Axum Extractors cho xác thực JWT + nạp Principal RBAC tươi từ DB và thông tin request.

use axum::{
    extract::FromRequestParts,
    http::{HeaderMap, header, request::Parts},
};
use pum_core::{AppError, Principal};
use pum_db::users;

use crate::{error::ApiError, state::AppState};

/// Thông tin mạng của client để phục vụ Rate Limiting và Audit Log.
#[derive(Debug, Clone)]
pub struct RequestMeta {
    pub ip: Option<String>,
    pub user_agent: Option<String>,
}

impl RequestMeta {
    #[must_use]
    pub fn from_headers(headers: &HeaderMap) -> Self {
        let ip = headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.split(',').next())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .or_else(|| {
                headers
                    .get("x-real-ip")
                    .and_then(|v| v.to_str().ok())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
            });

        let user_agent = headers
            .get(header::USER_AGENT)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.chars().take(256).collect());

        Self { ip, user_agent }
    }

    #[must_use]
    pub fn rate_key(&self) -> String {
        self.ip.clone().unwrap_or_else(|| "local".to_owned())
    }
}

/// Extractor bắt buộc đăng nhập — giải mã JWT và nạp đầy đủ Role + Permission overrides mới nhất.
#[derive(Debug, Clone)]
pub struct AuthUser(pub Principal);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = extract_bearer_or_query_token(parts).ok_or_else(|| {
            ApiError(AppError::Unauthorized(
                "thiếu access token trong header Authorization".to_owned(),
            ))
        })?;

        let claims = state.tokens.verify_access_token(&token)?;
        let principal = users::load_principal(&state.pool, claims.sub).await?;
        Ok(Self(principal))
    }
}

/// Extractor xác thực tuỳ chọn (dùng cho các endpoint có thể xem công khai như paste public).
#[derive(Debug, Clone)]
pub struct OptionalAuthUser(pub Option<Principal>);

impl FromRequestParts<AppState> for OptionalAuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let Some(token) = extract_bearer_or_query_token(parts) else {
            return Ok(Self(None));
        };
        let Ok(claims) = state.tokens.verify_access_token(&token) else {
            return Ok(Self(None));
        };
        let Ok(principal) = users::load_principal(&state.pool, claims.sub).await else {
            return Ok(Self(None));
        };
        Ok(Self(Some(principal)))
    }
}

fn extract_bearer_or_query_token(parts: &Parts) -> Option<String> {
    if let Some(auth_val) = parts
        .headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        && let Some(token) = auth_val.strip_prefix("Bearer ")
    {
        let trimmed = token.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_owned());
        }
    }

    // Hỗ trợ `?token=...` cho kết nối SSE (`EventSource` không gửi được header Authorization)
    if let Some(query) = parts.uri.query() {
        for pair in query.split('&') {
            if let Some((k, v)) = pair.split_once('=')
                && k == "token"
                && !v.is_empty()
            {
                return Some(v.to_owned());
            }
        }
    }

    None
}
