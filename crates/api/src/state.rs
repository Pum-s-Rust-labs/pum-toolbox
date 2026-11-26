//! Trạng thái chia sẻ an toàn (Shared Application State) của máy chủ Axum.

use std::sync::Arc;
use std::time::Duration;

use pum_core::AppConfig;
use pum_modules::monitor::MonitorEvent;
use pum_security::{RateLimiter, TokenService};
use reqwest::Client;
use sqlx::PgPool;
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub pool: PgPool,
    pub tokens: TokenService,
    pub auth_limiter: Arc<RateLimiter>,
    pub api_limiter: Arc<RateLimiter>,
    pub monitor_tx: broadcast::Sender<MonitorEvent>,
    pub probe_client: Client,
}

impl AppState {
    #[must_use]
    pub fn new(
        config: AppConfig,
        pool: PgPool,
        monitor_tx: broadcast::Sender<MonitorEvent>,
        probe_client: Client,
    ) -> Self {
        let tokens = TokenService::new(&config.jwt_secret, config.jwt_access_ttl_secs);
        Self {
            config: Arc::new(config),
            pool,
            tokens,
            // Tối đa 15 lần thử đăng nhập/đăng ký mỗi phút trên cùng IP
            auth_limiter: Arc::new(RateLimiter::new(15, Duration::from_secs(60))),
            // Tối đa 180 request API mỗi phút trên cùng IP
            api_limiter: Arc::new(RateLimiter::new(180, Duration::from_secs(60))),
            monitor_tx,
            probe_client,
        }
    }
}
