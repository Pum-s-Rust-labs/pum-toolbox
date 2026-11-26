//! Repository cho 4 module nghiệp vụ: Shortlink, Uptime Monitor, Pastebin và `ImageLab`.

use chrono::{DateTime, Utc};
use pum_core::{AppResult, Visibility};
use sqlx::PgPool;
use uuid::Uuid;

use crate::map_db_err;
use crate::models::{
    DailyClickStat, DashboardSummary, DeviceClickStat, ImageJobRow, MonitorCheckRow, MonitorRow,
    PasteRow, PasteSummaryRow, ShortLinkRow,
};

// ==================== SHORTLINK ====================

#[allow(clippy::too_many_arguments)]
pub async fn create_short_link(
    pool: &PgPool,
    owner_id: Uuid,
    slug: &str,
    target_url: &str,
    title: Option<&str>,
    visibility: Visibility,
    password_hash: Option<&str>,
    expires_at: Option<DateTime<Utc>>,
) -> AppResult<ShortLinkRow> {
    sqlx::query_as::<_, ShortLinkRow>(
        r"
        INSERT INTO short_links (id, owner_id, slug, target_url, title, visibility, password_hash, expires_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id, owner_id, slug, target_url, title, visibility, password_hash,
                  (password_hash IS NOT NULL) AS has_password,
                  expires_at, click_count, is_active, created_at
        ",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(slug)
    .bind(target_url)
    .bind(title)
    .bind(visibility.as_str())
    .bind(password_hash)
    .bind(expires_at)
    .fetch_one(pool)
    .await
    .map_err(map_db_err)
}

pub async fn list_short_links(
    pool: &PgPool,
    viewer_id: Uuid,
    can_view_all_private: bool,
) -> AppResult<Vec<ShortLinkRow>> {
    sqlx::query_as::<_, ShortLinkRow>(
        r"
        SELECT id, owner_id, slug, target_url, title, visibility, password_hash,
               (password_hash IS NOT NULL) AS has_password,
               expires_at, click_count, is_active, created_at
        FROM short_links
        WHERE ($2 = TRUE OR owner_id = $1 OR visibility IN ('internal', 'public'))
        ORDER BY created_at DESC
        LIMIT 200
        ",
    )
    .bind(viewer_id)
    .bind(can_view_all_private)
    .fetch_all(pool)
    .await
    .map_err(map_db_err)
}

pub async fn find_short_link_by_id(pool: &PgPool, id: Uuid) -> AppResult<Option<ShortLinkRow>> {
    sqlx::query_as::<_, ShortLinkRow>(
        r"
        SELECT id, owner_id, slug, target_url, title, visibility, password_hash,
               (password_hash IS NOT NULL) AS has_password,
               expires_at, click_count, is_active, created_at
        FROM short_links
        WHERE id = $1
        ",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(map_db_err)
}

pub async fn find_short_link_by_slug(pool: &PgPool, slug: &str) -> AppResult<Option<ShortLinkRow>> {
    sqlx::query_as::<_, ShortLinkRow>(
        r"
        SELECT id, owner_id, slug, target_url, title, visibility, password_hash,
               (password_hash IS NOT NULL) AS has_password,
               expires_at, click_count, is_active, created_at
        FROM short_links
        WHERE slug = $1 AND is_active = TRUE
        ",
    )
    .bind(slug)
    .fetch_optional(pool)
    .await
    .map_err(map_db_err)
}

pub async fn delete_short_link(pool: &PgPool, id: Uuid) -> AppResult<()> {
    sqlx::query("DELETE FROM short_links WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(map_db_err)?;
    Ok(())
}

pub async fn record_link_click(
    pool: &PgPool,
    link_id: Uuid,
    referer: Option<&str>,
    user_agent: Option<&str>,
    device_type: &str,
    ip_hash: Option<&str>,
) -> AppResult<()> {
    let mut tx = pool.begin().await.map_err(map_db_err)?;
    sqlx::query(
        r"
        INSERT INTO link_clicks (id, link_id, referer, user_agent, device_type, ip_hash)
        VALUES ($1, $2, $3, $4, $5, $6)
        ",
    )
    .bind(Uuid::new_v4())
    .bind(link_id)
    .bind(referer)
    .bind(user_agent)
    .bind(device_type)
    .bind(ip_hash)
    .execute(&mut *tx)
    .await
    .map_err(map_db_err)?;

    sqlx::query(
        "UPDATE short_links SET click_count = click_count + 1, updated_at = NOW() WHERE id = $1",
    )
    .bind(link_id)
    .execute(&mut *tx)
    .await
    .map_err(map_db_err)?;

    tx.commit().await.map_err(map_db_err)?;
    Ok(())
}

pub async fn get_link_daily_stats(pool: &PgPool, link_id: Uuid) -> AppResult<Vec<DailyClickStat>> {
    sqlx::query_as::<_, DailyClickStat>(
        r"
        SELECT TO_CHAR(clicked_at AT TIME ZONE 'UTC', 'YYYY-MM-DD') AS day,
               COUNT(*)::BIGINT AS clicks
        FROM link_clicks
        WHERE link_id = $1 AND clicked_at >= NOW() - INTERVAL '14 days'
        GROUP BY day
        ORDER BY day ASC
        ",
    )
    .bind(link_id)
    .fetch_all(pool)
    .await
    .map_err(map_db_err)
}

pub async fn get_link_device_stats(
    pool: &PgPool,
    link_id: Uuid,
) -> AppResult<Vec<DeviceClickStat>> {
    sqlx::query_as::<_, DeviceClickStat>(
        r"
        SELECT device_type, COUNT(*)::BIGINT AS clicks
        FROM link_clicks
        WHERE link_id = $1
        GROUP BY device_type
        ORDER BY clicks DESC
        ",
    )
    .bind(link_id)
    .fetch_all(pool)
    .await
    .map_err(map_db_err)
}

// ==================== UPTIME MONITOR ====================

#[allow(clippy::too_many_arguments)]
pub async fn create_monitor(
    pool: &PgPool,
    owner_id: Uuid,
    name: &str,
    url: &str,
    method: &str,
    expected_status: i32,
    interval_secs: i32,
    timeout_ms: i32,
    visibility: Visibility,
) -> AppResult<MonitorRow> {
    sqlx::query_as::<_, MonitorRow>(
        r"
        INSERT INTO monitors (id, owner_id, name, url, method, expected_status, interval_secs, timeout_ms, visibility)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id, owner_id, name, url, method, expected_status, interval_secs, timeout_ms,
                  visibility, is_paused, last_status, last_status_code, last_latency_ms,
                  last_error, last_checked_at, created_at
        ",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(name)
    .bind(url)
    .bind(method)
    .bind(expected_status)
    .bind(interval_secs)
    .bind(timeout_ms)
    .bind(visibility.as_str())
    .fetch_one(pool)
    .await
    .map_err(map_db_err)
}

pub async fn list_monitors(
    pool: &PgPool,
    viewer_id: Uuid,
    can_view_all_private: bool,
) -> AppResult<Vec<MonitorRow>> {
    sqlx::query_as::<_, MonitorRow>(
        r"
        SELECT id, owner_id, name, url, method, expected_status, interval_secs, timeout_ms,
               visibility, is_paused, last_status, last_status_code, last_latency_ms,
               last_error, last_checked_at, created_at
        FROM monitors
        WHERE ($2 = TRUE OR owner_id = $1 OR visibility IN ('internal', 'public'))
        ORDER BY created_at DESC
        LIMIT 200
        ",
    )
    .bind(viewer_id)
    .bind(can_view_all_private)
    .fetch_all(pool)
    .await
    .map_err(map_db_err)
}

pub async fn list_due_monitors(pool: &PgPool) -> AppResult<Vec<MonitorRow>> {
    sqlx::query_as::<_, MonitorRow>(
        r"
        SELECT id, owner_id, name, url, method, expected_status, interval_secs, timeout_ms,
               visibility, is_paused, last_status, last_status_code, last_latency_ms,
               last_error, last_checked_at, created_at
        FROM monitors
        WHERE is_paused = FALSE
          AND (
              last_checked_at IS NULL
              OR last_checked_at <= NOW() - make_interval(secs => interval_secs)
          )
        ORDER BY last_checked_at ASC NULLS FIRST
        LIMIT 50
        ",
    )
    .fetch_all(pool)
    .await
    .map_err(map_db_err)
}

pub async fn find_monitor_by_id(pool: &PgPool, id: Uuid) -> AppResult<Option<MonitorRow>> {
    sqlx::query_as::<_, MonitorRow>(
        r"
        SELECT id, owner_id, name, url, method, expected_status, interval_secs, timeout_ms,
               visibility, is_paused, last_status, last_status_code, last_latency_ms,
               last_error, last_checked_at, created_at
        FROM monitors
        WHERE id = $1
        ",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(map_db_err)
}

pub async fn set_monitor_paused(pool: &PgPool, id: Uuid, is_paused: bool) -> AppResult<MonitorRow> {
    sqlx::query_as::<_, MonitorRow>(
        r"
        UPDATE monitors
        SET is_paused = $2, updated_at = NOW()
        WHERE id = $1
        RETURNING id, owner_id, name, url, method, expected_status, interval_secs, timeout_ms,
                  visibility, is_paused, last_status, last_status_code, last_latency_ms,
                  last_error, last_checked_at, created_at
        ",
    )
    .bind(id)
    .bind(is_paused)
    .fetch_one(pool)
    .await
    .map_err(map_db_err)
}

pub async fn delete_monitor(pool: &PgPool, id: Uuid) -> AppResult<()> {
    sqlx::query("DELETE FROM monitors WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(map_db_err)?;
    Ok(())
}

pub async fn record_monitor_check(
    pool: &PgPool,
    monitor_id: Uuid,
    is_up: bool,
    status_code: Option<i32>,
    latency_ms: i32,
    error_message: Option<&str>,
) -> AppResult<MonitorCheckRow> {
    let mut tx = pool.begin().await.map_err(map_db_err)?;

    let check = sqlx::query_as::<_, MonitorCheckRow>(
        r"
        INSERT INTO monitor_checks (id, monitor_id, is_up, status_code, latency_ms, error_message)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, monitor_id, is_up, status_code, latency_ms, error_message, checked_at
        ",
    )
    .bind(Uuid::new_v4())
    .bind(monitor_id)
    .bind(is_up)
    .bind(status_code)
    .bind(latency_ms)
    .bind(error_message)
    .fetch_one(&mut *tx)
    .await
    .map_err(map_db_err)?;

    let status_label = if is_up { "up" } else { "down" };
    sqlx::query(
        r"
        UPDATE monitors
        SET last_status = $2,
            last_status_code = $3,
            last_latency_ms = $4,
            last_error = $5,
            last_checked_at = $6,
            updated_at = NOW()
        WHERE id = $1
        ",
    )
    .bind(monitor_id)
    .bind(status_label)
    .bind(status_code)
    .bind(latency_ms)
    .bind(error_message)
    .bind(check.checked_at)
    .execute(&mut *tx)
    .await
    .map_err(map_db_err)?;

    tx.commit().await.map_err(map_db_err)?;
    Ok(check)
}

pub async fn list_monitor_checks(
    pool: &PgPool,
    monitor_id: Uuid,
    limit: i64,
) -> AppResult<Vec<MonitorCheckRow>> {
    sqlx::query_as::<_, MonitorCheckRow>(
        r"
        SELECT id, monitor_id, is_up, status_code, latency_ms, error_message, checked_at
        FROM monitor_checks
        WHERE monitor_id = $1
        ORDER BY checked_at DESC
        LIMIT $2
        ",
    )
    .bind(monitor_id)
    .bind(limit.clamp(1, 200))
    .fetch_all(pool)
    .await
    .map_err(map_db_err)
}

// ==================== PASTEBIN ====================

#[allow(clippy::too_many_arguments)]
pub async fn create_paste(
    pool: &PgPool,
    owner_id: Uuid,
    slug: &str,
    title: &str,
    content: &str,
    rendered_html: Option<&str>,
    language: &str,
    tags: &[String],
    visibility: Visibility,
    max_views: Option<i32>,
    expires_at: Option<DateTime<Utc>>,
) -> AppResult<PasteRow> {
    sqlx::query_as::<_, PasteRow>(
        r"
        INSERT INTO pastes (id, owner_id, slug, title, content, rendered_html, language, tags, visibility, max_views, expires_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        RETURNING id, owner_id, slug, title, content, rendered_html, language, tags,
                  visibility, max_views, view_count, expires_at, created_at
        ",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(slug)
    .bind(title)
    .bind(content)
    .bind(rendered_html)
    .bind(language)
    .bind(tags)
    .bind(visibility.as_str())
    .bind(max_views)
    .bind(expires_at)
    .fetch_one(pool)
    .await
    .map_err(map_db_err)
}

pub async fn list_pastes(
    pool: &PgPool,
    viewer_id: Uuid,
    can_view_all_private: bool,
    search_query: Option<&str>,
) -> AppResult<Vec<PasteSummaryRow>> {
    let pattern = search_query
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| format!("%{s}%"));

    sqlx::query_as::<_, PasteSummaryRow>(
        r"
        SELECT id, owner_id, slug, title, language, tags, visibility, max_views, view_count, expires_at, created_at
        FROM pastes
        WHERE ($2 = TRUE OR owner_id = $1 OR visibility IN ('internal', 'public'))
          AND (expires_at IS NULL OR expires_at > NOW())
          AND (max_views IS NULL OR view_count < max_views)
          AND ($3::TEXT IS NULL OR title ILIKE $3 OR content ILIKE $3 OR array_to_string(tags, ' ') ILIKE $3)
        ORDER BY created_at DESC
        LIMIT 100
        ",
    )
    .bind(viewer_id)
    .bind(can_view_all_private)
    .bind(pattern)
    .fetch_all(pool)
    .await
    .map_err(map_db_err)
}

pub async fn find_paste_by_slug(pool: &PgPool, slug: &str) -> AppResult<Option<PasteRow>> {
    sqlx::query_as::<_, PasteRow>(
        r"
        SELECT id, owner_id, slug, title, content, rendered_html, language, tags,
               visibility, max_views, view_count, expires_at, created_at
        FROM pastes
        WHERE slug = $1
        ",
    )
    .bind(slug)
    .fetch_optional(pool)
    .await
    .map_err(map_db_err)
}

pub async fn find_paste_by_id(pool: &PgPool, id: Uuid) -> AppResult<Option<PasteRow>> {
    sqlx::query_as::<_, PasteRow>(
        r"
        SELECT id, owner_id, slug, title, content, rendered_html, language, tags,
               visibility, max_views, view_count, expires_at, created_at
        FROM pastes
        WHERE id = $1
        ",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(map_db_err)
}

pub async fn increment_paste_view(pool: &PgPool, id: Uuid) -> AppResult<Option<i32>> {
    sqlx::query_scalar::<_, i32>(
        r"
        UPDATE pastes
        SET view_count = view_count + 1, updated_at = NOW()
        WHERE id = $1
          AND (expires_at IS NULL OR expires_at > NOW())
          AND (max_views IS NULL OR view_count < max_views)
        RETURNING view_count
        ",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(map_db_err)
}

pub async fn delete_paste(pool: &PgPool, id: Uuid) -> AppResult<()> {
    sqlx::query("DELETE FROM pastes WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(map_db_err)?;
    Ok(())
}

// ==================== IMAGE JOBS ====================

#[allow(clippy::too_many_arguments)]
pub async fn insert_image_job(
    pool: &PgPool,
    owner_id: Uuid,
    file_count: i32,
    output_format: &str,
    quality: i32,
    original_bytes: i64,
    processed_bytes: i64,
    duration_ms: i32,
) -> AppResult<ImageJobRow> {
    sqlx::query_as::<_, ImageJobRow>(
        r"
        INSERT INTO image_jobs (id, owner_id, file_count, output_format, quality, original_bytes, processed_bytes, duration_ms)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id, owner_id, file_count, output_format, quality, original_bytes, processed_bytes, duration_ms, created_at
        ",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(file_count)
    .bind(output_format)
    .bind(quality)
    .bind(original_bytes)
    .bind(processed_bytes)
    .bind(duration_ms)
    .fetch_one(pool)
    .await
    .map_err(map_db_err)
}

pub async fn list_image_jobs(
    pool: &PgPool,
    owner_id: Uuid,
    can_view_all: bool,
) -> AppResult<Vec<ImageJobRow>> {
    sqlx::query_as::<_, ImageJobRow>(
        r"
        SELECT id, owner_id, file_count, output_format, quality, original_bytes, processed_bytes, duration_ms, created_at
        FROM image_jobs
        WHERE ($2 = TRUE OR owner_id = $1)
        ORDER BY created_at DESC
        LIMIT 50
        ",
    )
    .bind(owner_id)
    .bind(can_view_all)
    .fetch_all(pool)
    .await
    .map_err(map_db_err)
}

// ==================== DASHBOARD SUMMARY ====================

pub async fn get_dashboard_summary(pool: &PgPool) -> AppResult<DashboardSummary> {
    let (total_links, total_clicks): (i64, i64) = sqlx::query_as(
        "SELECT COUNT(*)::BIGINT, COALESCE(SUM(click_count), 0)::BIGINT FROM short_links",
    )
    .fetch_one(pool)
    .await
    .map_err(map_db_err)?;

    let (total_monitors, monitors_up, monitors_down): (i64, i64, i64) = sqlx::query_as(
        r"
        SELECT
            COUNT(*)::BIGINT,
            COUNT(*) FILTER (WHERE last_status = 'up')::BIGINT,
            COUNT(*) FILTER (WHERE last_status = 'down')::BIGINT
        FROM monitors
        ",
    )
    .fetch_one(pool)
    .await
    .map_err(map_db_err)?;

    let total_pastes: i64 = sqlx::query_scalar(
        r"
        SELECT COUNT(*)::BIGINT
        FROM pastes
        WHERE (expires_at IS NULL OR expires_at > NOW())
          AND (max_views IS NULL OR view_count < max_views)
        ",
    )
    .fetch_one(pool)
    .await
    .map_err(map_db_err)?;

    let (total_image_jobs, total_bytes_saved): (i64, i64) = sqlx::query_as(
        r"
        SELECT
            COUNT(*)::BIGINT,
            COALESCE(SUM(GREATEST(original_bytes - processed_bytes, 0)), 0)::BIGINT
        FROM image_jobs
        ",
    )
    .fetch_one(pool)
    .await
    .map_err(map_db_err)?;

    let total_users: i64 = sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM users")
        .fetch_one(pool)
        .await
        .map_err(map_db_err)?;

    Ok(DashboardSummary {
        total_links,
        total_clicks,
        total_monitors,
        monitors_up,
        monitors_down,
        total_pastes,
        total_image_jobs,
        total_bytes_saved,
        total_users,
    })
}
