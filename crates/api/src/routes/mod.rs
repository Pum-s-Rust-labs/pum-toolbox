//! Tổng hợp toàn bộ router của Pum Toolbox API + Static SPA serving.

pub mod admin;
pub mod auth;
pub mod images;
pub mod links;
pub mod monitors;
pub mod pastes;

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Request, State},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post, put},
};
use pum_core::AppError;
use serde_json::json;
use tower_http::services::{ServeDir, ServeFile};

use crate::{error::ApiError, extractors::RequestMeta, state::AppState};

async fn api_rate_limit(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let meta = RequestMeta::from_headers(req.headers());
    if let Err(retry_after_secs) = state.api_limiter.check(&meta.rate_key()) {
        return ApiError(AppError::RateLimited { retry_after_secs }).into_response();
    }
    next.run(req).await
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "service": "pum-toolbox",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// Xây dựng `Router` hoàn chỉnh bao gồm API v1, public shortlink redirect và React SPA fallback.
pub fn build_router(state: AppState) -> Router {
    let max_body = state
        .config
        .max_upload_bytes
        .saturating_mul(10)
        .min(120 * 1024 * 1024);

    let api_v1 = Router::new()
        // Auth
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route("/auth/refresh", post(auth::refresh))
        .route("/auth/logout", post(auth::logout))
        .route("/auth/me", get(auth::me))
        .route("/auth/change-password", post(auth::change_password))
        // Dashboard
        .route("/dashboard/summary", get(admin::dashboard_summary))
        // Module 1: Shortlink
        .route("/links", get(links::list_links).post(links::create_link))
        .route("/links/{id}", delete(links::delete_link))
        .route("/links/{id}/analytics", get(links::link_analytics))
        .route("/links/{id}/qr", get(links::link_qr))
        // Module 2: Uptime Monitor
        .route("/monitors", get(monitors::list_monitors).post(monitors::create_monitor))
        .route("/monitors/stream", get(monitors::stream_monitors))
        .route(
            "/monitors/{id}",
            patch(monitors::update_monitor).delete(monitors::delete_monitor),
        )
        .route("/monitors/{id}/checks", get(monitors::list_monitor_checks))
        .route("/monitors/{id}/check-now", post(monitors::check_monitor_now))
        // Module 3: Pastebin
        .route("/pastes", get(pastes::list_pastes).post(pastes::create_paste))
        .route("/pastes/{slug}", get(pastes::get_paste))
        .route("/pastes/id/{id}", delete(pastes::delete_paste))
        // Module 4: Image Toolbox
        .route("/images/process", post(images::process_images))
        .route("/images/jobs", get(images::list_image_jobs))
        // Admin & Audit
        .route("/admin/users", get(admin::list_users))
        .route("/admin/users/{id}/role", patch(admin::update_user_role))
        .route("/admin/users/{id}/status", patch(admin::update_user_status))
        .route("/admin/users/{id}/permissions", put(admin::replace_user_permissions))
        .route("/admin/audit-logs", get(admin::list_audit_logs))
        .layer(middleware::from_fn_with_state(state.clone(), api_rate_limit));

    let spa_service = ServeDir::new("web/dist").fallback(ServeFile::new("web/dist/index.html"));

    Router::new()
        .route("/health", get(health))
        .route("/r/{slug}", get(links::redirect_slug))
        .route("/r/{slug}/unlock", post(links::unlock_slug))
        .nest("/api/v1", api_v1)
        .fallback_service(spa_service)
        .layer(DefaultBodyLimit::max(max_body))
        .with_state(state)
}
