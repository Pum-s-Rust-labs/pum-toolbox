//! Lỗi nghiệp vụ và bảo mật chuẩn dùng chung cho toàn bộ workspace.

use thiserror::Error;

/// Kiểu `Result` chuẩn của nền tảng Pum Toolbox.
pub type AppResult<T> = Result<T, AppError>;

/// Các nhóm lỗi có ngữ nghĩa rõ ràng để ánh xạ sang HTTP status code.
#[derive(Debug, Error)]
pub enum AppError {
    /// Dữ liệu đầu vào hoặc cấu hình không hợp lệ (HTTP 400).
    #[error("{0}")]
    Validation(String),

    /// Chưa đăng nhập hoặc token hết hạn/không hợp lệ (HTTP 401).
    #[error("{0}")]
    Unauthorized(String),

    /// Không đủ quyền theo chính sách RBAC / sở hữu tài nguyên (HTTP 403).
    #[error("{0}")]
    Forbidden(String),

    /// Không tìm thấy tài nguyên (HTTP 404).
    #[error("{0}")]
    NotFound(String),

    /// Xung đột dữ liệu (ví dụ trùng email, username, slug) (HTTP 409).
    #[error("{0}")]
    Conflict(String),

    /// Tài nguyên đã hết hạn hoặc bị tiêu hủy (HTTP 410).
    #[error("{0}")]
    Gone(String),

    /// Vượt giới hạn tần suất request (HTTP 429).
    #[error("quá nhiều yêu cầu, vui lòng thử lại sau {retry_after_secs} giây")]
    RateLimited { retry_after_secs: u64 },

    /// Vi phạm chính sách bảo mật (SSRF, file độc hại, decompression bomb) (HTTP 422).
    #[error("{0}")]
    SecurityViolation(String),

    /// Lỗi hệ thống / cơ sở dữ liệu nội bộ (HTTP 500).
    #[error("lỗi hệ thống nội bộ: {0}")]
    Internal(String),
}
