//! Crate `app-core`: chứa toàn bộ logic nghiệp vụ thuần Rust, độc lập với giao diện
//! và nền tảng (Web, CLI, Desktop Windows, Mobile).

pub mod config;
pub mod error;

use serde::{Deserialize, Serialize};
use tracing::info;

pub use crate::config::{AppConfig, Environment};
pub use crate::error::{CoreError, CoreResult};

/// Thông tin trạng thái cơ bản của ứng dụng.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppInfo {
    /// Tên ứng dụng.
    pub name: String,
    /// Phiên bản crate hiện tại.
    pub version: String,
    /// Môi trường vận hành.
    pub environment: Environment,
}

/// Tạo đối tượng mô tả thông tin ứng dụng từ `AppConfig`.
#[must_use]
pub fn app_info(config: &AppConfig) -> AppInfo {
    AppInfo {
        name: config.app_name.clone(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        environment: config.environment,
    }
}

/// Hàm nghiệp vụ mẫu: tạo lời chào đã kiểm tra dữ liệu đầu vào.
pub fn greet(target: &str) -> CoreResult<String> {
    let trimmed = target.trim();
    if trimmed.is_empty() {
        return Err(CoreError::InvalidInput(
            "tên người nhận không được để trống".to_owned(),
        ));
    }

    info!(target = trimmed, "generating greeting");
    Ok(format!(
        "Xin chào, {trimmed}! Chào mừng đến với Pum's Rust Labs."
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greet_returns_formatted_message() {
        let message = greet("Rustacean").expect("valid target should succeed");
        assert!(message.contains("Rustacean"));
    }

    #[test]
    fn greet_rejects_empty_target() {
        let err = greet("   ").expect_err("blank target should fail");
        assert!(matches!(err, CoreError::InvalidInput(_)));
    }

    #[test]
    fn app_info_serializes_to_json() {
        let info = app_info(&AppConfig::default());
        let json = serde_json::to_string(&info).expect("AppInfo should serialize");
        assert!(json.contains("pum-rust-app"));
    }
}
