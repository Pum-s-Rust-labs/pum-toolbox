//! Repository quản lý người dùng, phân quyền override, refresh token rotation và audit log.

use chrono::{DateTime, Utc};
use pum_core::{
    AppError, AppResult, Permission, PermissionOverride, Principal, Role, resolve_permissions,
};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::map_db_err;
use crate::models::{AuditLogRow, RefreshTokenRow, UserPermissionRow, UserRow};

/// Đếm tổng số user hiện có trong hệ thống.
pub async fn count_users(pool: &PgPool) -> AppResult<i64> {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await
        .map_err(map_db_err)
}

/// Tạo mới người dùng.
pub async fn create_user(
    pool: &PgPool,
    email: &str,
    username: &str,
    password_hash: &str,
    role: Role,
) -> AppResult<UserRow> {
    let id = Uuid::new_v4();
    sqlx::query_as::<_, UserRow>(
        r"
        INSERT INTO users (id, email, username, password_hash, role)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, email, username, password_hash, role, is_active,
                  failed_login_attempts, locked_until, created_at, updated_at
        ",
    )
    .bind(id)
    .bind(email)
    .bind(username)
    .bind(password_hash)
    .bind(role.as_str())
    .fetch_one(pool)
    .await
    .map_err(map_db_err)
}

/// Tìm user theo email hoặc username (dùng cho đăng nhập).
pub async fn find_user_by_identifier(
    pool: &PgPool,
    identifier: &str,
) -> AppResult<Option<UserRow>> {
    let lowered = identifier.trim().to_ascii_lowercase();
    sqlx::query_as::<_, UserRow>(
        r"
        SELECT id, email, username, password_hash, role, is_active,
               failed_login_attempts, locked_until, created_at, updated_at
        FROM users
        WHERE email = $1 OR username = $1
        ",
    )
    .bind(lowered)
    .fetch_optional(pool)
    .await
    .map_err(map_db_err)
}

/// Tìm user theo `id`.
pub async fn find_user_by_id(pool: &PgPool, id: Uuid) -> AppResult<Option<UserRow>> {
    sqlx::query_as::<_, UserRow>(
        r"
        SELECT id, email, username, password_hash, role, is_active,
               failed_login_attempts, locked_until, created_at, updated_at
        FROM users
        WHERE id = $1
        ",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(map_db_err)
}

/// Liệt kê toàn bộ người dùng (cho Admin / `SuperAdmin`).
pub async fn list_users(pool: &PgPool) -> AppResult<Vec<UserRow>> {
    sqlx::query_as::<_, UserRow>(
        r"
        SELECT id, email, username, password_hash, role, is_active,
               failed_login_attempts, locked_until, created_at, updated_at
        FROM users
        ORDER BY created_at ASC
        ",
    )
    .fetch_all(pool)
    .await
    .map_err(map_db_err)
}

/// Ghi nhận một lần đăng nhập thất bại và khóa tạm thời 15 phút sau 5 lần sai liên tiếp.
pub async fn record_failed_login(pool: &PgPool, user_id: Uuid) -> AppResult<()> {
    sqlx::query(
        r"
        UPDATE users
        SET failed_login_attempts = failed_login_attempts + 1,
            locked_until = CASE
                WHEN failed_login_attempts + 1 >= 5 THEN NOW() + INTERVAL '15 minutes'
                ELSE locked_until
            END,
            updated_at = NOW()
        WHERE id = $1
        ",
    )
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(map_db_err)?;
    Ok(())
}

/// Xóa bộ đếm đăng nhập sai sau khi đăng nhập thành công.
pub async fn reset_failed_logins(pool: &PgPool, user_id: Uuid) -> AppResult<()> {
    sqlx::query(
        r"
        UPDATE users
        SET failed_login_attempts = 0,
            locked_until = NULL,
            updated_at = NOW()
        WHERE id = $1
        ",
    )
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(map_db_err)?;
    Ok(())
}

/// Cập nhật vai trò của người dùng.
pub async fn update_user_role(pool: &PgPool, user_id: Uuid, role: Role) -> AppResult<UserRow> {
    sqlx::query_as::<_, UserRow>(
        r"
        UPDATE users
        SET role = $2, updated_at = NOW()
        WHERE id = $1
        RETURNING id, email, username, password_hash, role, is_active,
                  failed_login_attempts, locked_until, created_at, updated_at
        ",
    )
    .bind(user_id)
    .bind(role.as_str())
    .fetch_one(pool)
    .await
    .map_err(map_db_err)
}

/// Khóa hoặc mở khóa tài khoản người dùng.
pub async fn update_user_status(
    pool: &PgPool,
    user_id: Uuid,
    is_active: bool,
) -> AppResult<UserRow> {
    sqlx::query_as::<_, UserRow>(
        r"
        UPDATE users
        SET is_active = $2,
            locked_until = CASE WHEN $2 THEN NULL ELSE locked_until END,
            failed_login_attempts = CASE WHEN $2 THEN 0 ELSE failed_login_attempts END,
            updated_at = NOW()
        WHERE id = $1
        RETURNING id, email, username, password_hash, role, is_active,
                  failed_login_attempts, locked_until, created_at, updated_at
        ",
    )
    .bind(user_id)
    .bind(is_active)
    .fetch_one(pool)
    .await
    .map_err(map_db_err)
}

/// Đổi mật khẩu người dùng.
pub async fn update_user_password(
    pool: &PgPool,
    user_id: Uuid,
    new_password_hash: &str,
) -> AppResult<()> {
    sqlx::query(
        r"
        UPDATE users
        SET password_hash = $2, updated_at = NOW()
        WHERE id = $1
        ",
    )
    .bind(user_id)
    .bind(new_password_hash)
    .execute(pool)
    .await
    .map_err(map_db_err)?;
    Ok(())
}

/// Lấy toàn bộ permission override của một user.
pub async fn get_user_permission_overrides(
    pool: &PgPool,
    user_id: Uuid,
) -> AppResult<Vec<UserPermissionRow>> {
    sqlx::query_as::<_, UserPermissionRow>(
        "SELECT user_id, permission, granted FROM user_permissions WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(map_db_err)
}

/// Thay thế danh sách permission override của một user trong transaction.
pub async fn replace_user_permissions(
    pool: &PgPool,
    user_id: Uuid,
    overrides: &[PermissionOverride],
    updated_by: Uuid,
) -> AppResult<()> {
    let mut tx = pool.begin().await.map_err(map_db_err)?;

    sqlx::query("DELETE FROM user_permissions WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(map_db_err)?;

    for item in overrides {
        sqlx::query(
            r"
            INSERT INTO user_permissions (user_id, permission, granted, updated_by)
            VALUES ($1, $2, $3, $4)
            ",
        )
        .bind(user_id)
        .bind(item.permission.as_str())
        .bind(item.granted)
        .bind(updated_by)
        .execute(&mut *tx)
        .await
        .map_err(map_db_err)?;
    }

    tx.commit().await.map_err(map_db_err)?;
    Ok(())
}

/// Xây dựng `Principal` đầy đủ (Role + Permission Overrides) từ `user_id`.
pub async fn load_principal(pool: &PgPool, user_id: Uuid) -> AppResult<Principal> {
    let user = find_user_by_id(pool, user_id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("tài khoản không tồn tại".to_owned()))?;

    if !user.is_active {
        return Err(AppError::Forbidden(
            "tài khoản đã bị vô hiệu hóa".to_owned(),
        ));
    }

    let role = Role::parse(&user.role)?;
    let rows = get_user_permission_overrides(pool, user_id).await?;
    let overrides: Vec<PermissionOverride> = rows
        .into_iter()
        .filter_map(|row| {
            Permission::parse(&row.permission)
                .ok()
                .map(|permission| PermissionOverride {
                    permission,
                    granted: row.granted,
                })
        })
        .collect();

    Ok(Principal {
        user_id: user.id,
        email: user.email,
        username: user.username,
        role,
        permissions: resolve_permissions(role, &overrides),
    })
}

/// Lưu hash refresh token mới vào DB.
pub async fn store_refresh_token(
    pool: &PgPool,
    user_id: Uuid,
    family_id: Uuid,
    token_hash: &str,
    expires_at: DateTime<Utc>,
    user_agent: Option<&str>,
    ip_address: Option<&str>,
) -> AppResult<()> {
    sqlx::query(
        r"
        INSERT INTO refresh_tokens (id, user_id, family_id, token_hash, expires_at, user_agent, ip_address)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(family_id)
    .bind(token_hash)
    .bind(expires_at)
    .bind(user_agent)
    .bind(ip_address)
    .execute(pool)
    .await
    .map_err(map_db_err)?;
    Ok(())
}

/// Tìm refresh token theo SHA-256 hash.
pub async fn find_refresh_token(
    pool: &PgPool,
    token_hash: &str,
) -> AppResult<Option<RefreshTokenRow>> {
    sqlx::query_as::<_, RefreshTokenRow>(
        r"
        SELECT id, user_id, family_id, token_hash, expires_at, revoked_at
        FROM refresh_tokens
        WHERE token_hash = $1
        ",
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await
    .map_err(map_db_err)
}

/// Thu hồi một refresh token cụ thể.
pub async fn revoke_refresh_token(pool: &PgPool, token_id: Uuid) -> AppResult<()> {
    sqlx::query("UPDATE refresh_tokens SET revoked_at = NOW() WHERE id = $1")
        .bind(token_id)
        .execute(pool)
        .await
        .map_err(map_db_err)?;
    Ok(())
}

/// Thu hồi toàn bộ token family (khi phát hiện replay/reuse token đã bị thu hồi).
pub async fn revoke_token_family(pool: &PgPool, family_id: Uuid) -> AppResult<()> {
    sqlx::query(
        "UPDATE refresh_tokens SET revoked_at = NOW() WHERE family_id = $1 AND revoked_at IS NULL",
    )
    .bind(family_id)
    .execute(pool)
    .await
    .map_err(map_db_err)?;
    Ok(())
}

/// Thu hồi toàn bộ refresh token của một user (ví dụ khi đổi mật khẩu hoặc bị khóa tài khoản).
pub async fn revoke_all_user_tokens(pool: &PgPool, user_id: Uuid) -> AppResult<()> {
    sqlx::query(
        "UPDATE refresh_tokens SET revoked_at = NOW() WHERE user_id = $1 AND revoked_at IS NULL",
    )
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(map_db_err)?;
    Ok(())
}

/// Ghi một dòng Audit Log bảo mật.
#[allow(clippy::too_many_arguments)]
pub async fn insert_audit_log(
    pool: &PgPool,
    actor_id: Option<Uuid>,
    actor_username: Option<&str>,
    action: &str,
    resource_type: &str,
    resource_id: Option<&str>,
    status: &str,
    metadata: Value,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
) -> AppResult<()> {
    sqlx::query(
        r"
        INSERT INTO audit_logs
            (id, actor_id, actor_username, action, resource_type, resource_id, status, metadata, ip_address, user_agent)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        ",
    )
    .bind(Uuid::new_v4())
    .bind(actor_id)
    .bind(actor_username)
    .bind(action)
    .bind(resource_type)
    .bind(resource_id)
    .bind(status)
    .bind(metadata)
    .bind(ip_address)
    .bind(user_agent)
    .execute(pool)
    .await
    .map_err(map_db_err)?;
    Ok(())
}

/// Truy vấn các dòng audit log gần nhất.
pub async fn list_audit_logs(pool: &PgPool, limit: i64) -> AppResult<Vec<AuditLogRow>> {
    sqlx::query_as::<_, AuditLogRow>(
        r"
        SELECT id, actor_id, actor_username, action, resource_type, resource_id,
               status, metadata, ip_address, user_agent, created_at
        FROM audit_logs
        ORDER BY created_at DESC
        LIMIT $1
        ",
    )
    .bind(limit.clamp(1, 200))
    .fetch_all(pool)
    .await
    .map_err(map_db_err)
}
