//! Định nghĩa các kiểu lỗi dùng chung cho toàn bộ logic nghiệp vụ.

use thiserror::Error;

/// Kiểu `Result` chuẩn cho crate `app-core`.
pub type CoreResult<T> = Result<T, CoreError>;

/// Các lỗi có cấu trúc trong tầng core.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CoreError {
    /// Cấu hình không hợp lệ hoặc bị thiếu giá trị bắt buộc.
    #[error("cấu hình không hợp lệ cho `{key}`: {reason}")]
    InvalidConfig { key: String, reason: String },

    /// Dữ liệu đầu vào không hợp lệ.
    #[error("dữ liệu đầu vào không hợp lệ: {0}")]
    InvalidInput(String),
}
