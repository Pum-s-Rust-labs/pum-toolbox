//! Điểm khởi chạy chính của máy chủ `pum-toolbox` (Axum + Tokio + PostgreSQL + Security Hardening).

mod error;
mod extractors;
mod routes;
mod state;

use std::net::SocketAddr;

use anyhow::{Context, Result};
use axum::http::{HeaderName, HeaderValue, Method, header};
use pum_core::{
    AppConfig, Role,
    validation::{normalize_email, normalize_username},
};
use pum_db::{connect_pool, run_migrations, users};
use pum_modules::monitor::{build_probe_client, spawn_monitor_worker};
use pum_security::hash_password;
use tokio::sync::broadcast;
use tower_http::{
    compression::CompressionLayer,
    cors::{AllowOrigin, CorsLayer},
    set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
};
use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::state::AppState;

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,pum_api=debug,pum_modules=debug"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}

async fn bootstrap_super_admin(pool: &sqlx::PgPool, config: &AppConfig) -> Result<()> {
    let existing = users::count_users(pool)
        .await
        .context("không thể đếm số người dùng hiện có")?;
    if existing > 0 {
        return Ok(());
    }

    if let (Some(email_raw), Some(username_raw), Some(password)) = (
        config.bootstrap_admin_email.as_deref(),
        config.bootstrap_admin_username.as_deref(),
        config.bootstrap_admin_password.as_deref(),
    ) {
        let email = normalize_email(email_raw)?;
        let username = normalize_username(username_raw)?;
        let hash = hash_password(password)?;
        let user = users::create_user(pool, &email, &username, &hash, Role::SuperAdmin).await?;
        info!(email = %user.email, username = %user.username, "bootstrapped initial SuperAdmin account");
    }

    Ok(())
}

fn build_cors_layer(config: &AppConfig) -> CorsLayer {
    let origins: Vec<HeaderValue> = config
        .cors_allowed_origins
        .iter()
        .filter_map(|origin| HeaderValue::from_str(origin).ok())
        .collect();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE, header::ACCEPT])
        .allow_credentials(true)
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    init_tracing();

    let config = AppConfig::from_env().context("không thể đọc cấu hình từ biến môi trường")?;
    info!(app = %config.app_name, host = %config.host, port = config.port, "starting pum-toolbox server");

    let pool = connect_pool(&config.database_url)
        .await
        .context("kết nối PostgreSQL thất bại")?;
    run_migrations(&pool)
        .await
        .context("chạy PostgreSQL migrations thất bại")?;
    bootstrap_super_admin(&pool, &config).await?;

    let (monitor_tx, _) = broadcast::channel(256);
    let probe_client = build_probe_client()?;
    spawn_monitor_worker(
        pool.clone(),
        monitor_tx.clone(),
        config.allow_private_monitor_targets,
    );

    let cors = build_cors_layer(&config);
    let addr: SocketAddr = format!("{}:{}", config.host, config.port)
        .parse()
        .context("địa chỉ APP_HOST:APP_PORT không hợp lệ")?;

    let state = AppState::new(config, pool, monitor_tx, probe_client);
    let app = routes::build_router(state)
        .layer(cors)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("x-content-type-options"),
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("x-frame-options"),
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("referrer-policy"),
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("permissions-policy"),
            HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
        ));

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("không thể mở cổng {addr}"))?;
    info!(%addr, "pum-toolbox HTTP server listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("HTTP server dừng với lỗi")?;

    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    info!("received shutdown signal, draining connections...");
}
