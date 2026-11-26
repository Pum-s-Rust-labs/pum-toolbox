//! Ký/xác thực JWT Access Token và sinh/băm Refresh Token ngẫu nhiên 256-bit.

use chrono::Utc;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use pum_core::{AppError, AppResult, Role};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Payload chuẩn trong JWT Access Token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessClaims {
    pub sub: Uuid,
    pub username: String,
    pub role: Role,
    pub jti: Uuid,
    pub iat: i64,
    pub exp: i64,
}

/// Dịch vụ phát hành và kiểm tra token.
#[derive(Clone)]
pub struct TokenService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    access_ttl_secs: i64,
}

impl TokenService {
    /// Khởi tạo `TokenService` từ khóa bí mật HMAC (tối thiểu 32 bytes).
    #[must_use]
    pub fn new(secret: &str, access_ttl_secs: i64) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            access_ttl_secs,
        }
    }

    /// Phát hành JWT Access Token ngắn hạn.
    pub fn issue_access_token(
        &self,
        user_id: Uuid,
        username: &str,
        role: Role,
    ) -> AppResult<String> {
        let now = Utc::now().timestamp();
        let claims = AccessClaims {
            sub: user_id,
            username: username.to_owned(),
            role,
            jti: Uuid::new_v4(),
            iat: now,
            exp: now + self.access_ttl_secs,
        };
        encode(&Header::new(Algorithm::HS256), &claims, &self.encoding_key)
            .map_err(|err| AppError::Internal(format!("không thể ký JWT: {err}")))
    }

    /// Giải mã và kiểm tra chữ ký + thời hạn của JWT Access Token.
    pub fn verify_access_token(&self, token: &str) -> AppResult<AccessClaims> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        decode::<AccessClaims>(token, &self.decoding_key, &validation)
            .map(|data| data.claims)
            .map_err(|_| {
                AppError::Unauthorized("access token không hợp lệ hoặc đã hết hạn".to_owned())
            })
    }
}

/// Sinh Refresh Token ngẫu nhiên 256-bit (trả về dạng chuỗi hex 64 ký tự).
#[must_use]
pub fn generate_refresh_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// Băm SHA-256 chuỗi bất kỳ (dùng để lưu refresh token hash hoặc IP hash an toàn).
#[must_use]
pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issues_and_verifies_jwt() {
        let svc = TokenService::new("super-secret-key-at-least-32-bytes-long!!", 600);
        let uid = Uuid::new_v4();
        let token = svc
            .issue_access_token(uid, "pum_admin", Role::Admin)
            .expect("should issue token");
        let claims = svc
            .verify_access_token(&token)
            .expect("should verify valid token");

        assert_eq!(claims.sub, uid);
        assert_eq!(claims.username, "pum_admin");
        assert_eq!(claims.role, Role::Admin);
    }

    #[test]
    fn generates_unique_refresh_tokens_and_hashes_deterministically() {
        let t1 = generate_refresh_token();
        let t2 = generate_refresh_token();
        assert_eq!(t1.len(), 64);
        assert_ne!(t1, t2);
        assert_eq!(sha256_hex(&t1), sha256_hex(&t1));
        assert_ne!(sha256_hex(&t1), sha256_hex(&t2));
    }
}
