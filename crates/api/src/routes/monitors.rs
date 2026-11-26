//! Routes Module 2: Uptime Monitor + SSRF Guard + Ping ngay + Stream SSE real-time.

use std::convert::Infallible;
use std::time::Duration;

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::sse::{Event, KeepAlive, Sse},
};
use futures_util::stream::Stream;
use pum_core::{AppError, Permission, Visibility};
use pum_db::{
    models::{MonitorCheckRow, MonitorRow},
    resources, users,
};
use pum_modules::monitor::{MonitorEvent, probe_monitor};
use pum_security::validate_outbound_url;
use serde::Deserialize;
use serde_json::json;
use tokio_stream::{StreamExt, wrappers::BroadcastStream};
use uuid::Uuid;

use crate::{
    error::ApiResult,
    extractors::{AuthUser, RequestMeta},
    state::AppState,
};

#[derive(Deserialize)]
pub struct CreateMonitorRequest {
    pub name: String,
    pub url: String,
    pub method: Option<String>,
    pub expected_status: Option<i32>,
    pub interval_secs: Option<i32>,
    pub timeout_ms: Option<i32>,
    pub visibility: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateMonitorRequest {
    pub is_paused: bool,
}

/// `GET /api/v1/monitors`
pub async fn list_monitors(
    State(state): State<AppState>,
    AuthUser(principal): AuthUser,
) -> ApiResult<Json<Vec<MonitorRow>>> {
    principal.require(Permission::MonitorsRead)?;
    let can_view_all = principal.has(Permission::MonitorsDeleteAny);
    let monitors = resources::list_monitors(&state.pool, principal.user_id, can_view_all).await?;
    Ok(Json(monitors))
}

/// `POST /api/v1/monitors` — Có kiểm tra SSRF Guard trước khi lưu.
pub async fn create_monitor(
    State(state): State<AppState>,
    AuthUser(principal): AuthUser,
    headers: HeaderMap,
    Json(req): Json<CreateMonitorRequest>,
) -> ApiResult<(StatusCode, Json<MonitorRow>)> {
    principal.require(Permission::MonitorsWrite)?;
    let meta = RequestMeta::from_headers(&headers);

    let name = req.name.trim();
    if name.is_empty() || name.len() > 100 {
        return Err(
            AppError::Validation("tên monitor phải dài từ 1 đến 100 ký tự".to_owned()).into(),
        );
    }

    let valid_url =
        validate_outbound_url(&req.url, state.config.allow_private_monitor_targets).await?;
    let method = match req
        .method
        .as_deref()
        .unwrap_or("GET")
        .trim()
        .to_ascii_uppercase()
        .as_str()
    {
        "HEAD" => "HEAD",
        "POST" => "POST",
        _ => "GET",
    };
    let expected_status = req.expected_status.unwrap_or(200).clamp(100, 599);
    let interval_secs = req.interval_secs.unwrap_or(60).clamp(15, 3600);
    let timeout_ms = req.timeout_ms.unwrap_or(5000).clamp(500, 30_000);
    let visibility = match req.visibility.as_deref() {
        Some(v) => Visibility::parse(v)?,
        None => Visibility::Internal,
    };

    let created = resources::create_monitor(
        &state.pool,
        principal.user_id,
        name,
        valid_url.as_str(),
        method,
        expected_status,
        interval_secs,
        timeout_ms,
        visibility,
    )
    .await?;

    // Kích hoạt lần ping đầu tiên ở nền ngay sau khi tạo
    let pool = state.pool.clone();
    let client = state.probe_client.clone();
    let tx = state.monitor_tx.clone();
    let allow_private = state.config.allow_private_monitor_targets;
    let monitor_clone = created.clone();
    tokio::spawn(async move {
        let _ = probe_monitor(&pool, &client, &tx, &monitor_clone, allow_private).await;
    });

    let _ = users::insert_audit_log(
        &state.pool,
        Some(principal.user_id),
        Some(&principal.username),
        "monitor.create",
        "monitor",
        Some(&created.id.to_string()),
        "success",
        json!({ "name": created.name, "url": created.url }),
        meta.ip.as_deref(),
        meta.user_agent.as_deref(),
    )
    .await;

    Ok((StatusCode::CREATED, Json(created)))
}

/// `PATCH /api/v1/monitors/{id}` — Tạm dừng hoặc chạy tiếp monitor.
pub async fn update_monitor(
    State(state): State<AppState>,
    AuthUser(principal): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateMonitorRequest>,
) -> ApiResult<Json<MonitorRow>> {
    let monitor = resources::find_monitor_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound("không tìm thấy monitor".to_owned()))?;

    principal.require_mutate_resource(
        monitor.owner_id,
        Permission::MonitorsWrite,
        Permission::MonitorsDeleteAny,
    )?;

    let updated = resources::set_monitor_paused(&state.pool, id, req.is_paused).await?;
    Ok(Json(updated))
}

/// `DELETE /api/v1/monitors/{id}`
pub async fn delete_monitor(
    State(state): State<AppState>,
    AuthUser(principal): AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let meta = RequestMeta::from_headers(&headers);
    let monitor = resources::find_monitor_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound("không tìm thấy monitor".to_owned()))?;

    principal.require_mutate_resource(
        monitor.owner_id,
        Permission::MonitorsWrite,
        Permission::MonitorsDeleteAny,
    )?;

    resources::delete_monitor(&state.pool, id).await?;
    let _ = users::insert_audit_log(
        &state.pool,
        Some(principal.user_id),
        Some(&principal.username),
        "monitor.delete",
        "monitor",
        Some(&id.to_string()),
        "success",
        json!({ "name": monitor.name }),
        meta.ip.as_deref(),
        meta.user_agent.as_deref(),
    )
    .await;

    Ok(Json(json!({ "deleted": true })))
}

/// `GET /api/v1/monitors/{id}/checks`
pub async fn list_monitor_checks(
    State(state): State<AppState>,
    AuthUser(principal): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<MonitorCheckRow>>> {
    principal.require(Permission::MonitorsRead)?;
    let monitor = resources::find_monitor_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound("không tìm thấy monitor".to_owned()))?;

    let vis = Visibility::parse(&monitor.visibility)?;
    if !principal.can_read_resource(
        monitor.owner_id,
        vis,
        Permission::MonitorsRead,
        Permission::MonitorsDeleteAny,
    ) {
        return Err(
            AppError::Forbidden("không có quyền xem lịch sử monitor này".to_owned()).into(),
        );
    }

    let checks = resources::list_monitor_checks(&state.pool, id, 60).await?;
    Ok(Json(checks))
}

/// `POST /api/v1/monitors/{id}/check-now`
pub async fn check_monitor_now(
    State(state): State<AppState>,
    AuthUser(principal): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<MonitorEvent>> {
    let monitor = resources::find_monitor_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound("không tìm thấy monitor".to_owned()))?;

    principal.require_mutate_resource(
        monitor.owner_id,
        Permission::MonitorsWrite,
        Permission::MonitorsDeleteAny,
    )?;

    let event = probe_monitor(
        &state.pool,
        &state.probe_client,
        &state.monitor_tx,
        &monitor,
        state.config.allow_private_monitor_targets,
    )
    .await?;

    Ok(Json(event))
}

/// `GET /api/v1/monitors/stream` — Server-Sent Events (SSE) đẩy trạng thái uptime real-time.
pub async fn stream_monitors(
    State(state): State<AppState>,
    AuthUser(principal): AuthUser,
) -> ApiResult<Sse<impl Stream<Item = Result<Event, Infallible>>>> {
    principal.require(Permission::MonitorsRead)?;
    let rx = state.monitor_tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|msg| match msg {
        Ok(event) => serde_json::to_string(&event)
            .ok()
            .map(|payload| Ok(Event::default().event("monitor_check").data(payload))),
        Err(_) => None,
    });

    Ok(Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    ))
}
