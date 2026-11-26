//! Crate `pum-core`: chứa domain models, lỗi chuẩn, ma trận phân quyền đa tầng RBAC
//! và bộ validator thuần Rust dùng chung cho toàn bộ hệ thống.

pub mod config;
pub mod error;
pub mod rbac;
pub mod validation;

pub use config::AppConfig;
pub use error::{AppError, AppResult};
pub use rbac::{Permission, PermissionOverride, Principal, Role, Visibility, resolve_permissions};
