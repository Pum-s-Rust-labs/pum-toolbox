//! Cấu hình hệ thống đọc từ biến môi trường với giá trị mặc định an toàn.

use std::env;

use crate::error::{AppError, AppResult};

/// Cấu hình chạy của server `pum-toolbox`.
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub app_name: String,
    pub host: String,
    pub port: u16,
    pub public_base_url: String,
    pub cors_allowed_origins: Vec<String>,
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_access_ttl_secs: i64,
    pub refresh_token_ttl_days: i64,
    pub cookie_secure: bool,
    pub bootstrap_admin_email: Option<String>,
    pub bootstrap_admin_username: Option<String>,
    pub bootstrap_admin_password: Option<String>,
    pub allow_private_monitor_targets: bool,
    pub max_upload_bytes: usize,
}

impl AppConfig {
    /// Đọc cấu hình từ biến môi trường tiến trình hiện tại.
    pub fn from_env() -> AppResult<Self> {
        Self::from_lookup(|k| env::var(k).ok())
    }

    /// Đọc cấu hình qua closure tra cứu (hỗ trợ unit test an toàn).
    pub fn from_lookup<F>(lookup: F) -> AppResult<Self>
    where
        F: Fn(&str) -> Option<String>,
    {
        let database_url = lookup("DATABASE_URL")
            .filter(|v| !v.trim().is_empty())
            .ok_or_else(|| {
                AppError::Validation("thiếu biến môi trường bắt buộc `DATABASE_URL`".to_owned())
            })?;

        let jwt_secret = lookup("JWT_SECRET")
            .filter(|v| v.len() >= 32)
            .ok_or_else(|| {
                AppError::Validation(
                    "biến `JWT_SECRET` bắt buộc phải có độ dài tối thiểu 32 ký tự".to_owned(),
                )
            })?;

        let port = lookup("PORT")
            .or_else(|| lookup("APP_PORT"))
            .map(|v| {
                v.parse::<u16>()
                    .map_err(|_| AppError::Validation("`PORT`/`APP_PORT` không hợp lệ".to_owned()))
            })
            .transpose()?
            .unwrap_or(8080);

        let jwt_access_ttl_secs = lookup("JWT_ACCESS_TTL_SECS")
            .and_then(|v| v.parse::<i64>().ok())
            .filter(|&v| v >= 60)
            .unwrap_or(900);

        let refresh_token_ttl_days = lookup("REFRESH_TOKEN_TTL_DAYS")
            .and_then(|v| v.parse::<i64>().ok())
            .filter(|&v| v >= 1)
            .unwrap_or(7);

        let max_upload_mb = lookup("MAX_UPLOAD_MB")
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|&v| (1..=100).contains(&v))
            .unwrap_or(15);

        let cors_allowed_origins = lookup("CORS_ALLOWED_ORIGINS")
            .unwrap_or_else(|| "http://localhost:5173,http://127.0.0.1:8080".to_owned())
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect();

        Ok(Self {
            app_name: lookup("APP_NAME").unwrap_or_else(|| "pum-toolbox".to_owned()),
            host: lookup("APP_HOST").unwrap_or_else(|| "127.0.0.1".to_owned()),
            port,
            public_base_url: lookup("PUBLIC_BASE_URL")
                .unwrap_or_else(|| format!("http://127.0.0.1:{port}"))
                .trim_end_matches('/')
                .to_owned(),
            cors_allowed_origins,
            database_url,
            jwt_secret,
            jwt_access_ttl_secs,
            refresh_token_ttl_days,
            cookie_secure: lookup("COOKIE_SECURE").is_some_and(|v| v.eq_ignore_ascii_case("true")),
            bootstrap_admin_email: lookup("BOOTSTRAP_ADMIN_EMAIL").filter(|v| !v.trim().is_empty()),
            bootstrap_admin_username: lookup("BOOTSTRAP_ADMIN_USERNAME")
                .filter(|v| !v.trim().is_empty()),
            bootstrap_admin_password: lookup("BOOTSTRAP_ADMIN_PASSWORD")
                .filter(|v| !v.trim().is_empty()),
            allow_private_monitor_targets: lookup("ALLOW_PRIVATE_MONITOR_TARGETS")
                .is_some_and(|v| v.eq_ignore_ascii_case("true")),
            max_upload_bytes: max_upload_mb * 1024 * 1024,
        })
    }
}
