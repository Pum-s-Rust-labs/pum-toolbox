//! Cấu hình ứng dụng đọc từ biến môi trường kèm giá trị mặc định an toàn.

use std::env;

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};

/// Môi trường vận hành của ứng dụng.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    /// Môi trường phát triển cục bộ.
    #[default]
    Development,
    /// Môi trường kiểm thử / staging.
    Staging,
    /// Môi trường vận hành thực tế.
    Production,
}

impl Environment {
    /// Phân tích chuỗi tên môi trường thành `Environment`.
    pub fn parse(value: &str) -> CoreResult<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "dev" | "development" => Ok(Self::Development),
            "staging" | "stage" => Ok(Self::Staging),
            "prod" | "production" => Ok(Self::Production),
            other => Err(CoreError::InvalidConfig {
                key: "APP_ENV".to_owned(),
                reason: format!(
                    "giá trị `{other}` không được hỗ trợ (dùng development, staging, hoặc production)"
                ),
            }),
        }
    }
}

/// Cấu hình cơ bản dùng chung cho mọi nền tảng (CLI, Web, Desktop, Mobile).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    /// Tên hiển thị của ứng dụng.
    pub app_name: String,
    /// Môi trường chạy hiện tại.
    pub environment: Environment,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            app_name: "pum-rust-app".to_owned(),
            environment: Environment::Development,
        }
    }
}

impl AppConfig {
    /// Đọc cấu hình từ biến môi trường của tiến trình hiện tại.
    pub fn from_env() -> CoreResult<Self> {
        Self::from_lookup(|key| env::var(key).ok())
    }

    /// Đọc cấu hình từ một hàm tra cứu bất kỳ (thuận tiện cho unit test và nền tảng nhúng).
    pub fn from_lookup<F>(lookup: F) -> CoreResult<Self>
    where
        F: Fn(&str) -> Option<String>,
    {
        let defaults = Self::default();

        let app_name = match lookup("APP_NAME") {
            Some(name) if name.trim().is_empty() => {
                return Err(CoreError::InvalidConfig {
                    key: "APP_NAME".to_owned(),
                    reason: "không được để trống".to_owned(),
                });
            }
            Some(name) => name.trim().to_owned(),
            None => defaults.app_name,
        };

        let environment = match lookup("APP_ENV") {
            Some(env_value) => Environment::parse(&env_value)?,
            None => defaults.environment,
        };

        Ok(Self {
            app_name,
            environment,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_defaults_when_env_missing() {
        let config = AppConfig::from_lookup(|_| None).expect("default config should succeed");
        assert_eq!(config.app_name, "pum-rust-app");
        assert_eq!(config.environment, Environment::Development);
    }

    #[test]
    fn parses_custom_env_values() {
        let config = AppConfig::from_lookup(|key| match key {
            "APP_NAME" => Some("my-tool".to_owned()),
            "APP_ENV" => Some("production".to_owned()),
            _ => None,
        })
        .expect("custom config should succeed");

        assert_eq!(config.app_name, "my-tool");
        assert_eq!(config.environment, Environment::Production);
    }

    #[test]
    fn rejects_invalid_environment() {
        let err = AppConfig::from_lookup(|key| match key {
            "APP_ENV" => Some("unknown".to_owned()),
            _ => None,
        })
        .expect_err("invalid APP_ENV should return error");

        assert!(matches!(err, CoreError::InvalidConfig { key, .. } if key == "APP_ENV"));
    }

    #[test]
    fn rejects_blank_app_name() {
        let err = AppConfig::from_lookup(|key| match key {
            "APP_NAME" => Some("   ".to_owned()),
            _ => None,
        })
        .expect_err("blank APP_NAME should return error");

        assert!(matches!(err, CoreError::InvalidConfig { key, .. } if key == "APP_NAME"));
    }
}
