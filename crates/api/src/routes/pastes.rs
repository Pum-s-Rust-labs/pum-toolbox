//! Routes Module 3: Code / Markdown Pastebin (lọc XSS, tự hủy burn-after-reading, tìm kiếm).

use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use chrono::{Duration as ChronoDuration, Utc};
use pum_core::{AppError, Permission, Visibility, validation::normalize_tags};
use pum_db::{
    models::{PasteRow, PasteSummaryRow},
    resources, users,
};
use pum_modules::shortlink::generate_slug;
use pum_security::render_safe_markdown;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::{
    error::ApiResult,
    extractors::{AuthUser, OptionalAuthUser, RequestMeta},
    state::AppState,
};

#[derive(Deserialize)]
pub struct ListPastesQuery {
    pub q: Option<String>,
}

#[derive(Deserialize)]
pub struct CreatePasteRequest {
    pub title: String,
    pub content: String,
    pub language: Option<String>,
    pub tags: Option<Vec<String>>,
    pub visibility: Option<String>,
    pub max_views: Option<i32>,
    pub expires_in_minutes: Option<i64>,
}

/// `GET /api/v1/pastes?q=...`
pub async fn list_pastes(
    State(state): State<AppState>,
    AuthUser(principal): AuthUser,
    Query(query): Query<ListPastesQuery>,
) -> ApiResult<Json<Vec<PasteSummaryRow>>> {
    principal.require(Permission::PastesRead)?;
    let can_view_all = principal.has(Permission::PastesDeleteAny);
    let items = resources::list_pastes(
        &state.pool,
        principal.user_id,
        can_view_all,
        query.q.as_deref(),
    )
    .await?;
    Ok(Json(items))
}

/// `POST /api/v1/pastes`
pub async fn create_paste(
    State(state): State<AppState>,
    AuthUser(principal): AuthUser,
    headers: HeaderMap,
    Json(req): Json<CreatePasteRequest>,
) -> ApiResult<(StatusCode, Json<PasteRow>)> {
    principal.require(Permission::PastesWrite)?;
    let meta = RequestMeta::from_headers(&headers);

    let title = req.title.trim();
    if title.is_empty() || title.len() > 140 {
        return Err(
            AppError::Validation("tiêu đề paste phải dài từ 1 đến 140 ký tự".to_owned()).into(),
        );
    }
    if req.content.trim().is_empty() || req.content.len() > 512_000 {
        return Err(AppError::Validation(
            "nội dung paste không được rỗng và tối đa 500 KB".to_owned(),
        )
        .into());
    }

    let language = req
        .language
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("markdown")
        .to_ascii_lowercase();

    let rendered_html = if language == "markdown" || language == "md" {
        Some(render_safe_markdown(&req.content))
    } else {
        None
    };

    let tags = normalize_tags(req.tags.as_deref().unwrap_or(&[]));
    let visibility = match req.visibility.as_deref() {
        Some(v) => Visibility::parse(v)?,
        None => Visibility::Internal,
    };
    let max_views = req.max_views.filter(|&v| v > 0);
    let expires_at = req
        .expires_in_minutes
        .filter(|&m| m > 0)
        .map(|m| Utc::now() + ChronoDuration::minutes(m.min(60 * 24 * 365)));

    let slug = generate_slug(10);
    let created = resources::create_paste(
        &state.pool,
        principal.user_id,
        &slug,
        title,
        &req.content,
        rendered_html.as_deref(),
        &language,
        &tags,
        visibility,
        max_views,
        expires_at,
    )
    .await?;

    let _ = users::insert_audit_log(
        &state.pool,
        Some(principal.user_id),
        Some(&principal.username),
        "paste.create",
        "paste",
        Some(&created.id.to_string()),
        "success",
        json!({ "slug": created.slug, "visibility": created.visibility }),
        meta.ip.as_deref(),
        meta.user_agent.as_deref(),
    )
    .await;

    Ok((StatusCode::CREATED, Json(created)))
}

/// `GET /api/v1/pastes/{slug}` — Kiểm tra quyền xem + hạn dùng + đếm lượt tự hủy (burn-after-reading).
pub async fn get_paste(
    State(state): State<AppState>,
    OptionalAuthUser(maybe_principal): OptionalAuthUser,
    Path(slug): Path<String>,
) -> ApiResult<Json<PasteRow>> {
    let mut paste = resources::find_paste_by_slug(&state.pool, &slug)
        .await?
        .ok_or_else(|| AppError::NotFound("không tìm thấy paste".to_owned()))?;

    let vis = Visibility::parse(&paste.visibility)?;
    match vis {
        Visibility::Public => {}
        Visibility::Internal => {
            let principal = maybe_principal.ok_or_else(|| {
                AppError::Unauthorized("vui lòng đăng nhập để xem paste nội bộ".to_owned())
            })?;
            principal.require(Permission::PastesRead)?;
        }
        Visibility::Private => {
            let principal = maybe_principal.ok_or_else(|| {
                AppError::Unauthorized("vui lòng đăng nhập để xem paste riêng tư".to_owned())
            })?;
            if !principal.can_read_resource(
                paste.owner_id,
                vis,
                Permission::PastesRead,
                Permission::PastesDeleteAny,
            ) {
                return Err(AppError::Forbidden(
                    "không có quyền xem paste riêng tư này".to_owned(),
                )
                .into());
            }
        }
    }

    if let Some(exp) = paste.expires_at
        && exp <= Utc::now()
    {
        return Err(AppError::Gone("paste này đã hết hạn".to_owned()).into());
    }

    let Some(new_views) = resources::increment_paste_view(&state.pool, paste.id).await? else {
        return Err(
            AppError::Gone("paste này đã đạt giới hạn lượt xem và bị tự hủy".to_owned()).into(),
        );
    };
    paste.view_count = new_views;

    Ok(Json(paste))
}

/// `DELETE /api/v1/pastes/{id}`
pub async fn delete_paste(
    State(state): State<AppState>,
    AuthUser(principal): AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let meta = RequestMeta::from_headers(&headers);
    let paste = resources::find_paste_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound("không tìm thấy paste".to_owned()))?;

    principal.require_mutate_resource(
        paste.owner_id,
        Permission::PastesWrite,
        Permission::PastesDeleteAny,
    )?;

    resources::delete_paste(&state.pool, id).await?;
    let _ = users::insert_audit_log(
        &state.pool,
        Some(principal.user_id),
        Some(&principal.username),
        "paste.delete",
        "paste",
        Some(&id.to_string()),
        "success",
        json!({ "slug": paste.slug }),
        meta.ip.as_deref(),
        meta.user_agent.as_deref(),
    )
    .await;

    Ok(Json(json!({ "deleted": true })))
}
