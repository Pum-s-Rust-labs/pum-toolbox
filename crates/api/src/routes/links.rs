//! Routes Module 1: Shortlink + Mã QR SVG + Thống kê lượt click + Redirect công khai `/r/:slug`.

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Redirect, Response},
};
use chrono::{Duration as ChronoDuration, Utc};
use pum_core::{
    AppError, Permission, Visibility,
    validation::{normalize_slug, validate_http_url},
};
use pum_db::{
    models::{DailyClickStat, DeviceClickStat, ShortLinkRow},
    resources, users,
};
use pum_modules::shortlink::{classify_device, generate_qr_svg, generate_slug};
use pum_security::{hash_password, sha256_hex, verify_password};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{
    error::ApiResult,
    extractors::{AuthUser, RequestMeta},
    state::AppState,
};

#[derive(Deserialize)]
pub struct CreateLinkRequest {
    pub target_url: String,
    pub slug: Option<String>,
    pub title: Option<String>,
    pub visibility: Option<String>,
    pub password: Option<String>,
    pub expires_in_hours: Option<i64>,
}

#[derive(Deserialize)]
pub struct UnlockLinkRequest {
    pub password: String,
}

#[derive(Serialize)]
pub struct LinkAnalyticsResponse {
    pub link: ShortLinkRow,
    pub short_url: String,
    pub daily: Vec<DailyClickStat>,
    pub devices: Vec<DeviceClickStat>,
}

/// `GET /api/v1/links`
pub async fn list_links(
    State(state): State<AppState>,
    AuthUser(principal): AuthUser,
) -> ApiResult<Json<Vec<ShortLinkRow>>> {
    principal.require(Permission::LinksRead)?;
    let can_view_all = principal.has(Permission::LinksDeleteAny);
    let items = resources::list_short_links(&state.pool, principal.user_id, can_view_all).await?;
    Ok(Json(items))
}

/// `POST /api/v1/links`
pub async fn create_link(
    State(state): State<AppState>,
    AuthUser(principal): AuthUser,
    headers: HeaderMap,
    Json(req): Json<CreateLinkRequest>,
) -> ApiResult<(StatusCode, Json<ShortLinkRow>)> {
    principal.require(Permission::LinksWrite)?;
    let meta = RequestMeta::from_headers(&headers);

    let validated_url = validate_http_url(&req.target_url)?;
    let slug = match req.slug.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(custom) => normalize_slug(custom)?,
        None => generate_slug(7),
    };
    let visibility = match req.visibility.as_deref() {
        Some(v) => Visibility::parse(v)?,
        None => Visibility::Public,
    };
    let password_hash = match req.password.as_deref().filter(|p| !p.is_empty()) {
        Some(pass) => Some(hash_password(pass)?),
        None => None,
    };
    let expires_at = req
        .expires_in_hours
        .filter(|&h| h > 0)
        .map(|h| Utc::now() + ChronoDuration::hours(h.min(24 * 365)));

    let title = req
        .title
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let created = resources::create_short_link(
        &state.pool,
        principal.user_id,
        &slug,
        validated_url.as_str(),
        title,
        visibility,
        password_hash.as_deref(),
        expires_at,
    )
    .await?;

    let _ = users::insert_audit_log(
        &state.pool,
        Some(principal.user_id),
        Some(&principal.username),
        "link.create",
        "short_link",
        Some(&created.id.to_string()),
        "success",
        json!({ "slug": created.slug }),
        meta.ip.as_deref(),
        meta.user_agent.as_deref(),
    )
    .await;

    Ok((StatusCode::CREATED, Json(created)))
}

/// `GET /api/v1/links/{id}/analytics`
pub async fn link_analytics(
    State(state): State<AppState>,
    AuthUser(principal): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<LinkAnalyticsResponse>> {
    principal.require(Permission::LinksRead)?;
    let link = resources::find_short_link_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound("không tìm thấy shortlink".to_owned()))?;

    let vis = Visibility::parse(&link.visibility)?;
    if !principal.can_read_resource(
        link.owner_id,
        vis,
        Permission::LinksRead,
        Permission::LinksDeleteAny,
    ) {
        return Err(AppError::Forbidden("không có quyền xem thống kê link này".to_owned()).into());
    }

    let daily = resources::get_link_daily_stats(&state.pool, id).await?;
    let devices = resources::get_link_device_stats(&state.pool, id).await?;
    let short_url = format!("{}/r/{}", state.config.public_base_url, link.slug);

    Ok(Json(LinkAnalyticsResponse {
        link,
        short_url,
        daily,
        devices,
    }))
}

/// `GET /api/v1/links/{id}/qr`
pub async fn link_qr(
    State(state): State<AppState>,
    AuthUser(principal): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Response> {
    principal.require(Permission::LinksRead)?;
    let link = resources::find_short_link_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound("không tìm thấy shortlink".to_owned()))?;

    let vis = Visibility::parse(&link.visibility)?;
    if !principal.can_read_resource(
        link.owner_id,
        vis,
        Permission::LinksRead,
        Permission::LinksDeleteAny,
    ) {
        return Err(AppError::Forbidden("không có quyền xem mã QR link này".to_owned()).into());
    }

    let short_url = format!("{}/r/{}", state.config.public_base_url, link.slug);
    let svg = generate_qr_svg(&short_url)?;

    Ok((
        [(header::CONTENT_TYPE, "image/svg+xml; charset=utf-8")],
        svg,
    )
        .into_response())
}

/// `DELETE /api/v1/links/{id}`
pub async fn delete_link(
    State(state): State<AppState>,
    AuthUser(principal): AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let meta = RequestMeta::from_headers(&headers);
    let link = resources::find_short_link_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound("không tìm thấy shortlink".to_owned()))?;

    principal.require_mutate_resource(
        link.owner_id,
        Permission::LinksWrite,
        Permission::LinksDeleteAny,
    )?;

    resources::delete_short_link(&state.pool, id).await?;
    let _ = users::insert_audit_log(
        &state.pool,
        Some(principal.user_id),
        Some(&principal.username),
        "link.delete",
        "short_link",
        Some(&id.to_string()),
        "success",
        json!({ "slug": link.slug }),
        meta.ip.as_deref(),
        meta.user_agent.as_deref(),
    )
    .await;

    Ok(Json(json!({ "deleted": true })))
}

/// `GET /r/{slug}` — Redirect công khai + ghi nhận lượt click bất đồng bộ.
pub async fn redirect_slug(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(slug): Path<String>,
) -> ApiResult<Response> {
    let link = resources::find_short_link_by_slug(&state.pool, &slug)
        .await?
        .ok_or_else(|| AppError::NotFound("liên kết rút gọn không tồn tại".to_owned()))?;

    if let Some(exp) = link.expires_at
        && exp <= Utc::now()
    {
        return Err(AppError::Gone("liên kết rút gọn này đã hết hạn".to_owned()).into());
    }

    if link.has_password {
        return Ok((
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "requires_password": true,
                "slug": link.slug,
                "title": link.title,
                "message": "liên kết này được bảo vệ bằng mật khẩu — gửi POST /r/{slug}/unlock"
            })),
        )
            .into_response());
    }

    spawn_record_click(&state, link.id, &headers);
    Ok(Redirect::temporary(&link.target_url).into_response())
}

/// `POST /r/{slug}/unlock` — Mở khóa link có mật khẩu.
pub async fn unlock_slug(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(slug): Path<String>,
    Json(req): Json<UnlockLinkRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let link = resources::find_short_link_by_slug(&state.pool, &slug)
        .await?
        .ok_or_else(|| AppError::NotFound("liên kết rút gọn không tồn tại".to_owned()))?;

    if let Some(exp) = link.expires_at
        && exp <= Utc::now()
    {
        return Err(AppError::Gone("liên kết rút gọn này đã hết hạn".to_owned()).into());
    }

    if let Some(hash) = link.password_hash.as_deref()
        && !verify_password(&req.password, hash)
    {
        return Err(AppError::Unauthorized("mật khẩu liên kết không đúng".to_owned()).into());
    }

    spawn_record_click(&state, link.id, &headers);
    Ok(Json(json!({ "target_url": link.target_url })))
}

fn spawn_record_click(state: &AppState, link_id: Uuid, headers: &HeaderMap) {
    let meta = RequestMeta::from_headers(headers);
    let referer = headers
        .get(header::REFERER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.chars().take(256).collect::<String>());
    let device = classify_device(meta.user_agent.as_deref()).to_owned();
    let ip_hash = meta.ip.as_deref().map(sha256_hex);
    let pool = state.pool.clone();

    tokio::spawn(async move {
        let _ = resources::record_link_click(
            &pool,
            link_id,
            referer.as_deref(),
            meta.user_agent.as_deref(),
            &device,
            ip_hash.as_deref(),
        )
        .await;
    });
}
