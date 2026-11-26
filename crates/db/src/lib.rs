//! Crate `pum-db`: kết nối PostgreSQL, chạy migrations tự động và cung cấp repositories.

pub mod models;
pub mod resources;
pub mod users;

use std::time::Duration;

use pum_core::{AppError, AppResult};
use sqlx::{PgPool, postgres::PgPoolOptions};
use tracing::info;

/// Tạo connection pool PostgreSQL với timeout an toàn.
pub async fn connect_pool(database_url: &str) -> AppResult<PgPool> {
    PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(15))
        .idle_timeout(Duration::from_secs(300))
        .connect(database_url)
        .await
        .map_err(|err| AppError::Internal(format!("không thể kết nối PostgreSQL: {err}")))
}

/// Chạy toàn bộ SQL migrations trong thư mục `migrations/`.
pub async fn run_migrations(pool: &PgPool) -> AppResult<()> {
    sqlx::migrate!("../../migrations")
        .run(pool)
        .await
        .map_err(|err| AppError::Internal(format!("chạy migration thất bại: {err}")))?;
    info!("PostgreSQL migrations completed successfully");
    Ok(())
}

/// Chuyển lỗi `sqlx::Error` sang `AppError` có ngữ nghĩa rõ ràng (`Conflict` / `NotFound` / `Internal`).
#[must_use]
pub fn map_db_err(err: sqlx::Error) -> AppError {
    match err {
        sqlx::Error::RowNotFound => AppError::NotFound("không tìm thấy bản ghi yêu cầu".to_owned()),
        sqlx::Error::Database(db_err) => {
            if let Some(code) = db_err.code() {
                if code == "23505" {
                    return AppError::Conflict(
                        "dữ liệu đã tồn tại (trùng khóa duy nhất như email, username hoặc slug)"
                            .to_owned(),
                    );
                }
                if code == "23503" || code == "23514" {
                    return AppError::Validation(format!(
                        "dữ liệu vi phạm ràng buộc cơ sở dữ liệu: {}",
                        db_err.message()
                    ));
                }
            }
            AppError::Internal(db_err.message().to_owned())
        }
        other => AppError::Internal(other.to_string()),
    }
}
