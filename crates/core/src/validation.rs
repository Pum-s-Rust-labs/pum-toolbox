//! Các hàm kiểm tra tính hợp lệ dữ liệu đầu vào dùng chung.

use url::Url;

use crate::error::{AppError, AppResult};

/// Kiểm tra và chuẩn hóa địa chỉ email.
pub fn normalize_email(raw: &str) -> AppResult<String> {
    let email = raw.trim().to_ascii_lowercase();
    let Some((local, domain)) = email.split_once('@') else {
        return Err(AppError::Validation(
            "địa chỉ email không hợp lệ".to_owned(),
        ));
    };
    if local.is_empty()
        || domain.len() < 3
        || !domain.contains('.')
        || domain.starts_with('.')
        || domain.ends_with('.')
        || email.contains(char::is_whitespace)
        || email.len() > 254
    {
        return Err(AppError::Validation(
            "địa chỉ email không hợp lệ".to_owned(),
        ));
    }
    Ok(email)
}

/// Kiểm tra và chuẩn hóa username (3-32 ký tự: chữ cái, số, `_`, `-`).
pub fn normalize_username(raw: &str) -> AppResult<String> {
    let username = raw.trim().to_ascii_lowercase();
    if !(3..=32).contains(&username.len()) {
        return Err(AppError::Validation(
            "username phải dài từ 3 đến 32 ký tự".to_owned(),
        ));
    }
    if !username
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(AppError::Validation(
            "username chỉ được chứa chữ cái không dấu, chữ số, `_` và `-`".to_owned(),
        ));
    }
    Ok(username)
}

/// Kiểm tra slug dùng cho shortlink / paste (3-64 ký tự URL-safe).
pub fn normalize_slug(raw: &str) -> AppResult<String> {
    let slug = raw.trim().to_owned();
    if !(3..=64).contains(&slug.len()) {
        return Err(AppError::Validation(
            "slug phải dài từ 3 đến 64 ký tự".to_owned(),
        ));
    }
    if !slug
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(AppError::Validation(
            "slug chỉ được chứa chữ cái, chữ số, `-` và `_`".to_owned(),
        ));
    }
    Ok(slug)
}

/// Kiểm tra URL hợp lệ với giao thức `http` hoặc `https`.
pub fn validate_http_url(raw: &str) -> AppResult<Url> {
    let trimmed = raw.trim();
    if trimmed.len() > 2048 {
        return Err(AppError::Validation("URL vượt quá 2048 ký tự".to_owned()));
    }
    let parsed = Url::parse(trimmed)
        .map_err(|_| AppError::Validation("URL không đúng định dạng".to_owned()))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(AppError::Validation(
            "chỉ chấp nhận URL có giao thức http hoặc https".to_owned(),
        ));
    }
    if parsed.host_str().is_none() {
        return Err(AppError::Validation("URL thiếu tên miền (host)".to_owned()));
    }
    Ok(parsed)
}

/// Chuẩn hóa danh sách tag (tối đa 10 tag, chữ thường, loại bỏ trùng lặp).
#[must_use]
pub fn normalize_tags(tags: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for tag in tags {
        let cleaned = tag.trim().to_ascii_lowercase();
        if !cleaned.is_empty() && cleaned.len() <= 32 && !out.contains(&cleaned) {
            out.push(cleaned);
        }
        if out.len() >= 10 {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_email_and_username() {
        assert_eq!(
            normalize_email("  Admin@PumLabs.dev ").expect("valid email"),
            "admin@pumlabs.dev"
        );
        assert!(normalize_email("invalid-email").is_err());

        assert_eq!(
            normalize_username("  Pum_Dev-01 ").expect("valid username"),
            "pum_dev-01"
        );
        assert!(normalize_username("ab").is_err());
        assert!(normalize_username("bad user!").is_err());
    }

    #[test]
    fn validates_http_url_rejecting_javascript() {
        assert!(validate_http_url("https://github.com/Pum-s-Rust-labs").is_ok());
        assert!(validate_http_url("javascript:alert(1)").is_err());
        assert!(validate_http_url("file:///etc/passwd").is_err());
    }

    #[test]
    fn normalizes_tags_dedup_and_cap() {
        let raw = vec![
            " Rust ".to_owned(),
            "rust".to_owned(),
            "Web".to_owned(),
            "  ".to_owned(),
        ];
        assert_eq!(normalize_tags(&raw), vec!["rust", "web"]);
    }
}
