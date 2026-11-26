//! Điểm khởi chạy mặc định (binary) của workspace.
//! Có thể giữ làm CLI/tool hoặc thêm các crate khác như `crates/api`, `crates/desktop`, `crates/mobile`.

use anyhow::{Context, Result};
use app_core::{AppConfig, app_info, greet};
use tracing::info;
use tracing_subscriber::EnvFilter;

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}

fn main() -> Result<()> {
    init_tracing();

    let config = AppConfig::from_env().context("không thể đọc cấu hình từ biến môi trường")?;
    let info = app_info(&config);

    info!(
        app = %info.name,
        version = %info.version,
        env = ?info.environment,
        "khởi động ứng dụng thành công"
    );

    let message = greet(&info.name).context("không thể tạo lời chào")?;
    println!("{message}");

    Ok(())
}
