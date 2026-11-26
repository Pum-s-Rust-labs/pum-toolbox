//! Băm và xác minh mật khẩu bằng Argon2id (khuyến nghị bởi OWASP).

use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use pum_core::{AppError, AppResult};

/// Kiểm tra chính sách độ mạnh mật khẩu tối thiểu.
pub fn validate_password_strength(password: &str) -> AppResult<()> {
    if password.len() < 8 {
        return Err(AppError::Validation(
            "mật khẩu phải có ít nhất 8 ký tự".to_owned(),
        ));
    }
    if password.len() > 128 {
        return Err(AppError::Validation(
            "mật khẩu không được vượt quá 128 ký tự".to_owned(),
        ));
    }
    let has_upper = password.chars().any(char::is_uppercase);
    let has_lower = password.chars().any(char::is_lowercase);
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    if !(has_upper && has_lower && has_digit) {
        return Err(AppError::Validation(
            "mật khẩu phải bao gồm ít nhất 1 chữ hoa, 1 chữ thường và 1 chữ số".to_owned(),
        ));
    }
    Ok(())
}

/// Băm mật khẩu bằng Argon2id kèm salt ngẫu nhiên từ OS CSPRNG.
pub fn hash_password(password: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|err| AppError::Internal(format!("không thể băm mật khẩu: {err}")))
}

/// Xác minh mật khẩu thuần với chuỗi PHC hash đã lưu.
#[must_use]
pub fn verify_password(password: &str, encoded_hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(encoded_hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_and_verifies_password() {
        let secret = "StrongPass123!";
        assert!(validate_password_strength(secret).is_ok());
        let hash = hash_password(secret).expect("argon2 hash should succeed");
        assert!(hash.starts_with("$argon2id$"));
        assert!(verify_password(secret, &hash));
        assert!(!verify_password("WrongPass123!", &hash));
    }

    #[test]
    fn rejects_weak_passwords() {
        assert!(validate_password_strength("short1A").is_err());
        assert!(validate_password_strength("alllowercase1").is_err());
        assert!(validate_password_strength("NoDigitsHere").is_err());
    }
}
