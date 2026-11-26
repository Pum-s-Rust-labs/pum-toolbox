//! Module 1: Rút gọn link, sinh mã QR SVG và phân tích lượt click.

use pum_core::{AppError, AppResult};
use qrcode::{QrCode, render::svg};
use rand::Rng;

const SLUG_ALPHABET: &[u8] = b"23456789abcdefghjkmnpqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ";

/// Sinh slug ngẫu nhiên an toàn URL (tránh các ký tự dễ nhầm như `0/O`, `1/l/I`).
#[must_use]
pub fn generate_slug(len: usize) -> String {
    let mut rng = rand::rngs::OsRng;
    (0..len)
        .map(|_| {
            let idx = rng.gen_range(0..SLUG_ALPHABET.len());
            SLUG_ALPHABET[idx] as char
        })
        .collect()
}

/// Tạo mã QR định dạng SVG cho một URL bất kỳ.
pub fn generate_qr_svg(content: &str) -> AppResult<String> {
    let code = QrCode::new(content.as_bytes())
        .map_err(|err| AppError::Internal(format!("không thể tạo mã QR: {err}")))?;
    Ok(code
        .render::<svg::Color<'_>>()
        .min_dimensions(240, 240)
        .dark_color(svg::Color("#0f172a"))
        .light_color(svg::Color("#ffffff"))
        .build())
}

/// Phân loại thiết bị từ chuỗi `User-Agent`.
#[must_use]
pub fn classify_device(user_agent: Option<&str>) -> &'static str {
    let Some(ua) = user_agent else {
        return "unknown";
    };
    let lower = ua.to_ascii_lowercase();
    if lower.contains("bot")
        || lower.contains("spider")
        || lower.contains("crawl")
        || lower.contains("curl")
    {
        "bot"
    } else if lower.contains("ipad") || lower.contains("tablet") {
        "tablet"
    } else if lower.contains("mobile") || lower.contains("android") || lower.contains("iphone") {
        "mobile"
    } else {
        "desktop"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_slug_and_qr_svg() {
        let slug = generate_slug(8);
        assert_eq!(slug.len(), 8);
        let svg = generate_qr_svg("https://github.com/Pum-s-Rust-labs").expect("qr svg");
        assert!(svg.contains("<svg"));
    }

    #[test]
    fn classifies_user_agents() {
        assert_eq!(
            classify_device(Some("Mozilla/5.0 (iPhone; CPU iPhone OS 17)")),
            "mobile"
        );
        assert_eq!(classify_device(Some("curl/8.4.0")), "bot");
        assert_eq!(
            classify_device(Some("Mozilla/5.0 (Windows NT 10.0; Win64)")),
            "desktop"
        );
    }
}
