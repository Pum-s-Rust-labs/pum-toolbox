//! Routes quản trị RBAC đa tầng, Dashboard tổng quan và Audit Logs.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
};
use pum_core::{AppError, Permission, PermissionOverride, Role};
use pum_db::{
    models::{AuditLogRow, DashboardSummary, UserPermissionRow, UserPublicView},
    resources, users,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{
    error::ApiResult,
    extractors::{AuthUser, RequestMeta},
    state::AppState,
};

#[derive(Serialize)]
pub struct AdminUserItem {
    #[serde(flatten)]
    pub user: UserPublicView,
    pub overrides: Vec<UserPermissionRow>,
}

#[derive(Deserialize)]
pub struct UpdateRoleRequest {
    pub role: String,
}

#[derive(Deserialize)]
pub struct UpdateStatusRequest {
    pub is_active: bool,
}

#[derive(Deserialize)]
pub struct ReplacePermissionsRequest {
    pub overrides: Vec<PermissionOverrideInput>,
}

#[derive(Deserialize)]
pub struct PermissionOverrideInput {
    pub permission: String,
    pub granted: bool,
}

#[derive(Deserialize)]
pub struct AuditQuery {
    pub limit: Option<i64>,
}

/// `GET /api/v1/dashboard/summary`
pub async fn dashboard_summary(
    State(state): State<AppState>,
    AuthUser(_principal): AuthUser,
) -> ApiResult<Json<DashboardSummary>> {
    let summary = resources::get_dashboard_summary(&state.pool).await?;
    Ok(Json(summary))
}

/// `GET /api/v1/admin/users`
pub async fn list_users(
    State(state): State<AppState>,
    AuthUser(principal): AuthUser,
) -> ApiResult<Json<Vec<AdminUserItem>>> {
    principal.require(Permission::UsersManage)?;
    let all_users = users::list_users(&state.pool).await?;
    let mut out = Vec::with_capacity(all_users.len());
    for row in all_users {
        let overrides = users::get_user_permission_overrides(&state.pool, row.id).await?;
        out.push(AdminUserItem {
            user: row.into(),
            overrides,
        });
    }
    Ok(Json(out))
}

/// `PATCH /api/v1/admin/users/{id}/role`
pub async fn update_user_role(
    State(state): State<AppState>,
    AuthUser(principal): AuthUser,
    headers: HeaderMap,
    Path(target_id): Path<Uuid>,
    Json(req): Json<UpdateRoleRequest>,
) -> ApiResult<Json<UserPublicView>> {
    principal.require(Permission::UsersManage)?;
    let meta = RequestMeta::from_headers(&headers);

    if target_id == principal.user_id {
        return Err(
            AppError::Validation("không thể tự đổi vai trò của chính mình".to_owned()).into(),
        );
    }

    let new_role = Role::parse(&req.role)?;
    let target = users::find_user_by_id(&state.pool, target_id)
        .await?
        .ok_or_else(|| AppError::NotFound("không tìm thấy người dùng".to_owned()))?;
    let current_target_role = Role::parse(&target.role)?;

    if !principal.role.can_manage_role(current_target_role)
        || !principal.role.can_manage_role(new_role)
    {
        return Err(AppError::Forbidden(
            "cấp vai trò của bạn không đủ để thay đổi vai trò mục tiêu này".to_owned(),
        )
        .into());
    }

    if matches!(new_role, Role::Admin | Role::SuperAdmin) {
        principal.require(Permission::RolesAssign)?;
    }

    let updated = users::update_user_role(&state.pool, target_id, new_role).await?;
    let _ = users::insert_audit_log(
        &state.pool,
        Some(principal.user_id),
        Some(&principal.username),
        "admin.update_role",
        "user",
        Some(&target_id.to_string()),
        "success",
        json!({ "from": target.role, "to": new_role.as_str() }),
        meta.ip.as_deref(),
        meta.user_agent.as_deref(),
    )
    .await;

    Ok(Json(updated.into()))
}

/// `PATCH /api/v1/admin/users/{id}/status`
pub async fn update_user_status(
    State(state): State<AppState>,
    AuthUser(principal): AuthUser,
    headers: HeaderMap,
    Path(target_id): Path<Uuid>,
    Json(req): Json<UpdateStatusRequest>,
) -> ApiResult<Json<UserPublicView>> {
    principal.require(Permission::UsersManage)?;
    let meta = RequestMeta::from_headers(&headers);

    if target_id == principal.user_id {
        return Err(
            AppError::Validation("không thể tự khóa tài khoản của chính mình".to_owned()).into(),
        );
    }

    let target = users::find_user_by_id(&state.pool, target_id)
        .await?
        .ok_or_else(|| AppError::NotFound("không tìm thấy người dùng".to_owned()))?;
    let target_role = Role::parse(&target.role)?;
    if !principal.role.can_manage_role(target_role) {
        return Err(AppError::Forbidden(
            "không đủ cấp bậc để thay đổi trạng thái người dùng này".to_owned(),
        )
        .into());
    }

    let updated = users::update_user_status(&state.pool, target_id, req.is_active).await?;
    if !req.is_active {
        let _ = users::revoke_all_user_tokens(&state.pool, target_id).await;
    }

    let _ = users::insert_audit_log(
        &state.pool,
        Some(principal.user_id),
        Some(&principal.username),
        if req.is_active {
            "admin.unlock_user"
        } else {
            "admin.lock_user"
        },
        "user",
        Some(&target_id.to_string()),
        "success",
        json!({ "is_active": req.is_active }),
        meta.ip.as_deref(),
        meta.user_agent.as_deref(),
    )
    .await;

    Ok(Json(updated.into()))
}

/// `PUT /api/v1/admin/users/{id}/permissions`
pub async fn replace_user_permissions(
    State(state): State<AppState>,
    AuthUser(principal): AuthUser,
    headers: HeaderMap,
    Path(target_id): Path<Uuid>,
    Json(req): Json<ReplacePermissionsRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    principal.require(Permission::UsersManage)?;
    let meta = RequestMeta::from_headers(&headers);

    let target = users::find_user_by_id(&state.pool, target_id)
        .await?
        .ok_or_else(|| AppError::NotFound("không tìm thấy người dùng".to_owned()))?;
    let target_role = Role::parse(&target.role)?;
    if !principal.role.can_manage_role(target_role) {
        return Err(AppError::Forbidden(
            "không đủ cấp bậc để chỉnh quyền riêng người dùng này".to_owned(),
        )
        .into());
    }

    let mut parsed = Vec::with_capacity(req.overrides.len());
    for item in &req.overrides {
        let perm = Permission::parse(&item.permission)?;
        if perm == Permission::RolesAssign && !principal.has(Permission::RolesAssign) {
            return Err(
                AppError::Forbidden("không thể cấp quyền `roles:assign`".to_owned()).into(),
            );
        }
        parsed.push(PermissionOverride {
            permission: perm,
            granted: item.granted,
        });
    }

    users::replace_user_permissions(&state.pool, target_id, &parsed, principal.user_id).await?;
    let _ = users::insert_audit_log(
        &state.pool,
        Some(principal.user_id),
        Some(&principal.username),
        "admin.replace_permissions",
        "user",
        Some(&target_id.to_string()),
        "success",
        json!({ "override_count": parsed.len() }),
        meta.ip.as_deref(),
        meta.user_agent.as_deref(),
    )
    .await;

    Ok(Json(json!({ "ok": true })))
}

/// `GET /api/v1/admin/audit-logs`
pub async fn list_audit_logs(
    State(state): State<AppState>,
    AuthUser(principal): AuthUser,
    Query(query): Query<AuditQuery>,
) -> ApiResult<Json<Vec<AuditLogRow>>> {
    principal.require(Permission::AuditRead)?;
    let logs = users::list_audit_logs(&state.pool, query.limit.unwrap_or(100)).await?;
    Ok(Json(logs))
}
