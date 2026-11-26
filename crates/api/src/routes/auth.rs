//! Routes xác thực: đăng ký, đăng nhập (chống brute-force), xoay vòng refresh token, đăng xuất, đổi mật khẩu.

use axum::{Json, extract::State, http::HeaderMap};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use chrono::{Duration as ChronoDuration, Utc};
use pum_core::{
    AppError, Principal, Role,
    validation::{normalize_email, normalize_username},
};
use pum_db::users;
use pum_security::{
    generate_refresh_token, hash_password, sha256_hex, validate_password_strength, verify_password,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{
    error::ApiResult,
    extractors::{AuthUser, RequestMeta},
    state::AppState,
};

const REFRESH_COOKIE_NAME: &str = "pum_refresh_token";

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub identifier: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub token_type: &'static str,
    pub expires_in: i64,
    pub user: Principal,
}

fn build_refresh_cookie(token: String, ttl_days: i64, secure: bool) -> Cookie<'static> {
    Cookie::build((REFRESH_COOKIE_NAME, token))
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(secure)
        .path("/api/v1/auth")
        .max_age(time::Duration::days(ttl_days))
        .build()
}

fn build_clear_cookie(secure: bool) -> Cookie<'static> {
    Cookie::build((REFRESH_COOKIE_NAME, String::new()))
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(secure)
        .path("/api/v1/auth")
        .max_age(time::Duration::ZERO)
        .build()
}

/// `POST /api/v1/auth/register`
pub async fn register(
    State(state): State<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(req): Json<RegisterRequest>,
) -> ApiResult<(CookieJar, Json<AuthResponse>)> {
    let meta = RequestMeta::from_headers(&headers);
    if let Err(retry_after_secs) = state
        .auth_limiter
        .check(&format!("reg:{}", meta.rate_key()))
    {
        return Err(AppError::RateLimited { retry_after_secs }.into());
    }

    let email = normalize_email(&req.email)?;
    let username = normalize_username(&req.username)?;
    validate_password_strength(&req.password)?;

    let password_hash = hash_password(&req.password)?;
    let existing_count = users::count_users(&state.pool).await?;
    let initial_role = if existing_count == 0 {
        Role::SuperAdmin
    } else {
        Role::Member
    };

    let created =
        users::create_user(&state.pool, &email, &username, &password_hash, initial_role).await?;
    let principal = users::load_principal(&state.pool, created.id).await?;

    let access_token =
        state
            .tokens
            .issue_access_token(principal.user_id, &principal.username, principal.role)?;
    let refresh_plain = generate_refresh_token();
    let refresh_hash = sha256_hex(&refresh_plain);
    let expires_at = Utc::now() + ChronoDuration::days(state.config.refresh_token_ttl_days);

    users::store_refresh_token(
        &state.pool,
        principal.user_id,
        Uuid::new_v4(),
        &refresh_hash,
        expires_at,
        meta.user_agent.as_deref(),
        meta.ip.as_deref(),
    )
    .await?;

    let _ = users::insert_audit_log(
        &state.pool,
        Some(principal.user_id),
        Some(&principal.username),
        "auth.register",
        "user",
        Some(&principal.user_id.to_string()),
        "success",
        json!({ "role": principal.role.as_str() }),
        meta.ip.as_deref(),
        meta.user_agent.as_deref(),
    )
    .await;

    let cookie = build_refresh_cookie(
        refresh_plain,
        state.config.refresh_token_ttl_days,
        state.config.cookie_secure,
    );

    Ok((
        jar.add(cookie),
        Json(AuthResponse {
            access_token,
            token_type: "Bearer",
            expires_in: state.config.jwt_access_ttl_secs,
            user: principal,
        }),
    ))
}

/// `POST /api/v1/auth/login`
pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(req): Json<LoginRequest>,
) -> ApiResult<(CookieJar, Json<AuthResponse>)> {
    let meta = RequestMeta::from_headers(&headers);
    if let Err(retry_after_secs) = state
        .auth_limiter
        .check(&format!("login:{}", meta.rate_key()))
    {
        return Err(AppError::RateLimited { retry_after_secs }.into());
    }

    let Some(user) = users::find_user_by_identifier(&state.pool, &req.identifier).await? else {
        return Err(
            AppError::Unauthorized("thông tin đăng nhập không chính xác".to_owned()).into(),
        );
    };

    if !user.is_active {
        return Err(
            AppError::Forbidden("tài khoản của bạn đã bị quản trị viên khóa".to_owned()).into(),
        );
    }

    if let Some(locked_until) = user.locked_until
        && locked_until > Utc::now()
    {
        return Err(AppError::Forbidden(
            "tài khoản tạm khóa 15 phút do đăng nhập sai nhiều lần".to_owned(),
        )
        .into());
    }

    if !verify_password(&req.password, &user.password_hash) {
        let _ = users::record_failed_login(&state.pool, user.id).await;
        let _ = users::insert_audit_log(
            &state.pool,
            Some(user.id),
            Some(&user.username),
            "auth.login",
            "user",
            Some(&user.id.to_string()),
            "failed",
            json!({ "reason": "invalid_password" }),
            meta.ip.as_deref(),
            meta.user_agent.as_deref(),
        )
        .await;
        return Err(
            AppError::Unauthorized("thông tin đăng nhập không chính xác".to_owned()).into(),
        );
    }

    users::reset_failed_logins(&state.pool, user.id).await?;
    let principal = users::load_principal(&state.pool, user.id).await?;

    let access_token =
        state
            .tokens
            .issue_access_token(principal.user_id, &principal.username, principal.role)?;
    let refresh_plain = generate_refresh_token();
    let refresh_hash = sha256_hex(&refresh_plain);
    let expires_at = Utc::now() + ChronoDuration::days(state.config.refresh_token_ttl_days);

    users::store_refresh_token(
        &state.pool,
        principal.user_id,
        Uuid::new_v4(),
        &refresh_hash,
        expires_at,
        meta.user_agent.as_deref(),
        meta.ip.as_deref(),
    )
    .await?;

    let _ = users::insert_audit_log(
        &state.pool,
        Some(principal.user_id),
        Some(&principal.username),
        "auth.login",
        "user",
        Some(&principal.user_id.to_string()),
        "success",
        json!({}),
        meta.ip.as_deref(),
        meta.user_agent.as_deref(),
    )
    .await;

    let cookie = build_refresh_cookie(
        refresh_plain,
        state.config.refresh_token_ttl_days,
        state.config.cookie_secure,
    );

    Ok((
        jar.add(cookie),
        Json(AuthResponse {
            access_token,
            token_type: "Bearer",
            expires_in: state.config.jwt_access_ttl_secs,
            user: principal,
        }),
    ))
}

/// `POST /api/v1/auth/refresh` — Xoay vòng refresh token kèm phát hiện tái sử dụng token.
pub async fn refresh(
    State(state): State<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
) -> ApiResult<(CookieJar, Json<AuthResponse>)> {
    let meta = RequestMeta::from_headers(&headers);
    let raw_cookie = jar
        .get(REFRESH_COOKIE_NAME)
        .map(|c| c.value().to_owned())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| {
            AppError::Unauthorized("không tìm thấy phiên đăng nhập (refresh cookie)".to_owned())
        })?;

    let token_hash = sha256_hex(&raw_cookie);
    let stored = users::find_refresh_token(&state.pool, &token_hash)
        .await?
        .ok_or_else(|| AppError::Unauthorized("refresh token không hợp lệ".to_owned()))?;

    // Phát hiện replay attack: token đã bị thu hồi nhưng vẫn bị gửi lại -> hủy toàn bộ family!
    if stored.revoked_at.is_some() {
        let _ = users::revoke_token_family(&state.pool, stored.family_id).await;
        let _ = users::insert_audit_log(
            &state.pool,
            Some(stored.user_id),
            None,
            "auth.refresh_reuse_detected",
            "refresh_token",
            Some(&stored.family_id.to_string()),
            "security_alert",
            json!({ "action": "revoked_family" }),
            meta.ip.as_deref(),
            meta.user_agent.as_deref(),
        )
        .await;
        return Err(AppError::Unauthorized(
            "phát hiện tái sử dụng token bất thường, toàn bộ phiên đã bị thu hồi".to_owned(),
        )
        .into());
    }

    if stored.expires_at <= Utc::now() {
        let _ = users::revoke_refresh_token(&state.pool, stored.id).await;
        return Err(AppError::Unauthorized("phiên đăng nhập đã hết hạn".to_owned()).into());
    }

    // Xoay vòng: thu hồi token cũ, phát hành token mới cùng family
    users::revoke_refresh_token(&state.pool, stored.id).await?;
    let principal = users::load_principal(&state.pool, stored.user_id).await?;

    let access_token =
        state
            .tokens
            .issue_access_token(principal.user_id, &principal.username, principal.role)?;
    let new_refresh = generate_refresh_token();
    let new_hash = sha256_hex(&new_refresh);
    let expires_at = Utc::now() + ChronoDuration::days(state.config.refresh_token_ttl_days);

    users::store_refresh_token(
        &state.pool,
        principal.user_id,
        stored.family_id,
        &new_hash,
        expires_at,
        meta.user_agent.as_deref(),
        meta.ip.as_deref(),
    )
    .await?;

    let cookie = build_refresh_cookie(
        new_refresh,
        state.config.refresh_token_ttl_days,
        state.config.cookie_secure,
    );

    Ok((
        jar.add(cookie),
        Json(AuthResponse {
            access_token,
            token_type: "Bearer",
            expires_in: state.config.jwt_access_ttl_secs,
            user: principal,
        }),
    ))
}

/// `POST /api/v1/auth/logout`
pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
) -> ApiResult<(CookieJar, Json<serde_json::Value>)> {
    let meta = RequestMeta::from_headers(&headers);
    if let Some(cookie) = jar.get(REFRESH_COOKIE_NAME) {
        let hash = sha256_hex(cookie.value());
        if let Ok(Some(stored)) = users::find_refresh_token(&state.pool, &hash).await {
            let _ = users::revoke_token_family(&state.pool, stored.family_id).await;
            let _ = users::insert_audit_log(
                &state.pool,
                Some(stored.user_id),
                None,
                "auth.logout",
                "user",
                Some(&stored.user_id.to_string()),
                "success",
                json!({}),
                meta.ip.as_deref(),
                meta.user_agent.as_deref(),
            )
            .await;
        }
    }

    let cleared = jar.add(build_clear_cookie(state.config.cookie_secure));
    Ok((cleared, Json(json!({ "ok": true }))))
}

/// `GET /api/v1/auth/me`
pub async fn me(AuthUser(principal): AuthUser) -> Json<Principal> {
    Json(principal)
}

/// `POST /api/v1/auth/change-password`
pub async fn change_password(
    State(state): State<AppState>,
    AuthUser(principal): AuthUser,
    headers: HeaderMap,
    Json(req): Json<ChangePasswordRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let meta = RequestMeta::from_headers(&headers);
    let user = users::find_user_by_id(&state.pool, principal.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("không tìm thấy tài khoản".to_owned()))?;

    if !verify_password(&req.current_password, &user.password_hash) {
        return Err(AppError::Unauthorized("mật khẩu hiện tại không đúng".to_owned()).into());
    }

    validate_password_strength(&req.new_password)?;
    let new_hash = hash_password(&req.new_password)?;
    users::update_user_password(&state.pool, principal.user_id, &new_hash).await?;
    users::revoke_all_user_tokens(&state.pool, principal.user_id).await?;

    let _ = users::insert_audit_log(
        &state.pool,
        Some(principal.user_id),
        Some(&principal.username),
        "auth.change_password",
        "user",
        Some(&principal.user_id.to_string()),
        "success",
        json!({}),
        meta.ip.as_deref(),
        meta.user_agent.as_deref(),
    )
    .await;

    Ok(Json(json!({ "ok": true })))
}
