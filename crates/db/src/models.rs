//! Các struct ánh xạ trực tiếp từ bảng PostgreSQL.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

/// Bản ghi người dùng đầy đủ (nội bộ DB, không trả `password_hash` ra API).
#[derive(Debug, Clone, FromRow)]
pub struct UserRow {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub password_hash: String,
    pub role: String,
    pub is_active: bool,
    pub failed_login_attempts: i32,
    pub locked_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Thông tin người dùng an toàn trả ra API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPublicView {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub role: String,
    pub is_active: bool,
    pub locked_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<UserRow> for UserPublicView {
    fn from(u: UserRow) -> Self {
        Self {
            id: u.id,
            email: u.email,
            username: u.username,
            role: u.role,
            is_active: u.is_active,
            locked_until: u.locked_until,
            created_at: u.created_at,
        }
    }
}

/// Bản ghi override quyền của từng user.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UserPermissionRow {
    pub user_id: Uuid,
    pub permission: String,
    pub granted: bool,
}

/// Bản ghi refresh token (chỉ lưu hash).
#[derive(Debug, Clone, FromRow)]
pub struct RefreshTokenRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub family_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

/// Bản ghi nhật ký kiểm toán bảo mật.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AuditLogRow {
    pub id: Uuid,
    pub actor_id: Option<Uuid>,
    pub actor_username: Option<String>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub status: String,
    pub metadata: Value,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Bản ghi Shortlink.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ShortLinkRow {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub slug: String,
    pub target_url: String,
    pub title: Option<String>,
    pub visibility: String,
    #[serde(skip_serializing)]
    pub password_hash: Option<String>,
    pub has_password: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub click_count: i64,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// Thống kê lượt click theo ngày.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DailyClickStat {
    pub day: String,
    pub clicks: i64,
}

/// Thống kê lượt click theo loại thiết bị.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DeviceClickStat {
    pub device_type: String,
    pub clicks: i64,
}

/// Bản ghi Uptime Monitor.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct MonitorRow {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub url: String,
    pub method: String,
    pub expected_status: i32,
    pub interval_secs: i32,
    pub timeout_ms: i32,
    pub visibility: String,
    pub is_paused: bool,
    pub last_status: String,
    pub last_status_code: Option<i32>,
    pub last_latency_ms: Option<i32>,
    pub last_error: Option<String>,
    pub last_checked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Lịch sử một lần ping monitor.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct MonitorCheckRow {
    pub id: Uuid,
    pub monitor_id: Uuid,
    pub is_up: bool,
    pub status_code: Option<i32>,
    pub latency_ms: i32,
    pub error_message: Option<String>,
    pub checked_at: DateTime<Utc>,
}

/// Bản ghi Paste đầy đủ.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct PasteRow {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub slug: String,
    pub title: String,
    pub content: String,
    pub rendered_html: Option<String>,
    pub language: String,
    pub tags: Vec<String>,
    pub visibility: String,
    pub max_views: Option<i32>,
    pub view_count: i32,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Bản ghi tóm tắt Paste cho danh sách.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct PasteSummaryRow {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub slug: String,
    pub title: String,
    pub language: String,
    pub tags: Vec<String>,
    pub visibility: String,
    pub max_views: Option<i32>,
    pub view_count: i32,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Bản ghi lịch sử xử lý ảnh.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ImageJobRow {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub file_count: i32,
    pub output_format: String,
    pub quality: i32,
    pub original_bytes: i64,
    pub processed_bytes: i64,
    pub duration_ms: i32,
    pub created_at: DateTime<Utc>,
}

/// Số liệu tổng quan cho Dashboard chính.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSummary {
    pub total_links: i64,
    pub total_clicks: i64,
    pub total_monitors: i64,
    pub monitors_up: i64,
    pub monitors_down: i64,
    pub total_pastes: i64,
    pub total_image_jobs: i64,
    pub total_bytes_saved: i64,
    pub total_users: i64,
}
