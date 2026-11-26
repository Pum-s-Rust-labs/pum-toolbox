//! Module 2: Uptime Monitor bất đồng bộ bằng Tokio + SSRF Guard + Broadcast SSE real-time.

use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use pum_core::{AppError, AppResult};
use pum_db::{models::MonitorRow, resources};
use pum_security::validate_outbound_url;
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokio::sync::broadcast;
use tracing::{debug, warn};
use uuid::Uuid;

/// Sự kiện trạng thái monitor đẩy real-time qua Server-Sent Events (SSE).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorEvent {
    pub monitor_id: Uuid,
    pub name: String,
    pub url: String,
    pub is_up: bool,
    pub status_code: Option<i32>,
    pub latency_ms: i32,
    pub error_message: Option<String>,
    pub checked_at: DateTime<Utc>,
}

/// Tạo HTTP client dùng chung cho worker kiểm tra uptime (không tự động follow redirect vô hạn).
pub fn build_probe_client() -> AppResult<Client> {
    Client::builder()
        .redirect(reqwest::redirect::Policy::limited(3))
        .user_agent(
            "PumToolbox-UptimeMonitor/0.1 (+https://github.com/Pum-s-Rust-labs/pum-toolbox)",
        )
        .build()
        .map_err(|err| AppError::Internal(format!("không thể khởi tạo HTTP client: {err}")))
}

/// Thực hiện kiểm tra ngay 1 monitor, lưu kết quả vào PostgreSQL và phát sự kiện SSE.
pub async fn probe_monitor(
    pool: &PgPool,
    client: &Client,
    tx: &broadcast::Sender<MonitorEvent>,
    monitor: &MonitorRow,
    allow_private: bool,
) -> AppResult<MonitorEvent> {
    let start = Instant::now();

    let (is_up, status_code, error_message) =
        match validate_outbound_url(&monitor.url, allow_private).await {
            Err(err) => (false, None, Some(err.to_string())),
            Ok(valid_url) => {
                let method = match monitor.method.as_str() {
                    "HEAD" => Method::HEAD,
                    "POST" => Method::POST,
                    _ => Method::GET,
                };
                let timeout =
                    Duration::from_millis(u64::try_from(monitor.timeout_ms).unwrap_or(5000));
                match client
                    .request(method, valid_url)
                    .timeout(timeout)
                    .send()
                    .await
                {
                    Ok(resp) => {
                        let code = i32::from(resp.status().as_u16());
                        if code == monitor.expected_status {
                            (true, Some(code), None)
                        } else {
                            (
                                false,
                                Some(code),
                                Some(format!(
                                    "nhận mã HTTP {code}, kỳ vọng {}",
                                    monitor.expected_status
                                )),
                            )
                        }
                    }
                    Err(err) => (false, None, Some(err.to_string())),
                }
            }
        };

    let latency_ms = i32::try_from(start.elapsed().as_millis()).unwrap_or(i32::MAX);
    let check = resources::record_monitor_check(
        pool,
        monitor.id,
        is_up,
        status_code,
        latency_ms,
        error_message.as_deref(),
    )
    .await?;

    let event = MonitorEvent {
        monitor_id: monitor.id,
        name: monitor.name.clone(),
        url: monitor.url.clone(),
        is_up,
        status_code,
        latency_ms,
        error_message,
        checked_at: check.checked_at,
    };

    let _ = tx.send(event.clone());
    Ok(event)
}

/// Khởi chạy worker nền Tokio định kỳ kiểm tra toàn bộ monitor đang hoạt động song song.
pub fn spawn_monitor_worker(
    pool: PgPool,
    tx: broadcast::Sender<MonitorEvent>,
    allow_private: bool,
) {
    tokio::spawn(async move {
        let client = match build_probe_client() {
            Ok(c) => c,
            Err(err) => {
                warn!("failed to start monitor worker: {err}");
                return;
            }
        };

        let mut ticker = tokio::time::interval(Duration::from_secs(10));
        loop {
            ticker.tick().await;
            let due = match resources::list_due_monitors(&pool).await {
                Ok(list) => list,
                Err(err) => {
                    warn!("failed to load due monitors: {err}");
                    continue;
                }
            };

            if !due.is_empty() {
                debug!(count = due.len(), "probing due uptime monitors");
            }

            let mut handles = Vec::with_capacity(due.len());
            for monitor in due {
                let pool_clone = pool.clone();
                let client_clone = client.clone();
                let tx_clone = tx.clone();
                handles.push(tokio::spawn(async move {
                    if let Err(err) = probe_monitor(
                        &pool_clone,
                        &client_clone,
                        &tx_clone,
                        &monitor,
                        allow_private,
                    )
                    .await
                    {
                        warn!(monitor_id = %monitor.id, "monitor probe failed: {err}");
                    }
                }));
            }

            for handle in handles {
                let _ = handle.await;
            }
        }
    });
}
