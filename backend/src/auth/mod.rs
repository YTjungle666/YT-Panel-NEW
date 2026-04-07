use std::time::{Duration, Instant};

use axum::{
    extract::{Request, State},
    http::{header::AUTHORIZATION, HeaderMap, Method},
    middleware::Next,
    response::{IntoResponse, Response},
};
use rand::{distributions::Alphanumeric, Rng};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::error::ApiError;
use crate::{
    db::{get_setting, load_user_by_persistent_token},
    models::{AccessMode, AppState, AuthCacheEntry, AuthContext, CurrentUser},
};

pub const SESSION_COOKIE_NAME: &str = "yt_panel_session";
pub const SESSION_COOKIE_MAX_AGE: i64 = 60 * 60 * 24 * 30;
pub const AUTH_CACHE_TTL: Duration = Duration::from_secs(60 * 15);
pub const AUTH_CACHE_MAX_ENTRIES: usize = 2048;

pub fn random_token(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

pub fn session_cookie_value(headers: &HeaderMap) -> Option<String> {
    headers
        .get("cookie")
        .and_then(|value| value.to_str().ok())
        .and_then(|cookie_header| {
            cookie_header.split(';').find_map(|part| {
                let (key, value) = part.trim().split_once('=')?;
                (key.trim() == SESSION_COOKIE_NAME).then(|| value.trim().to_string())
            })
        })
        .filter(|value| !value.is_empty())
}

pub fn bearer_token_value(headers: &HeaderMap) -> Option<String> {
    headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer ").or_else(|| value.strip_prefix("bearer ")))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

pub fn request_token_value(headers: &HeaderMap) -> Option<String> {
    session_cookie_value(headers).or_else(|| bearer_token_value(headers))
}

fn extract_forwarded_proto(headers: &HeaderMap) -> Option<String> {
    headers
        .get("forwarded")
        .and_then(|value| value.to_str().ok())
        .and_then(|raw| {
            raw.split(';').find_map(|part| {
                part.trim()
                    .strip_prefix("proto=")
                    .map(|value| value.trim_matches('"').to_lowercase())
            })
        })
}

pub fn request_is_https(headers: &HeaderMap) -> bool {
    if matches!(extract_forwarded_proto(headers).as_deref(), Some("https")) {
        return true;
    }

    if headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(|value| value.trim().eq_ignore_ascii_case("https"))
        .unwrap_or(false)
    {
        return true;
    }

    if headers
        .get("x-forwarded-ssl")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.eq_ignore_ascii_case("on"))
        .unwrap_or(false)
    {
        return true;
    }

    headers
        .get("front-end-https")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.eq_ignore_ascii_case("on"))
        .unwrap_or(false)
}

pub fn build_session_cookie(token: &str, secure: bool) -> String {
    let secure_attr = if secure { "; Secure" } else { "" };
    format!(
        "{}={}; Path=/; HttpOnly; SameSite=Lax{}; Max-Age={}",
        SESSION_COOKIE_NAME, token, secure_attr, SESSION_COOKIE_MAX_AGE
    )
}

pub fn build_cleared_session_cookie(secure: bool) -> String {
    let secure_attr = if secure { "; Secure" } else { "" };
    format!(
        "{}=; Path=/; HttpOnly; SameSite=Lax{}; Max-Age=0; Expires=Thu, 01 Jan 1970 00:00:00 GMT",
        SESSION_COOKIE_NAME, secure_attr
    )
}

pub async fn invalidate_cached_token(state: &AppState, token: Option<&str>) {
    if let Some(token) = token.filter(|value| !value.is_empty()) {
        state.auth_cache.write().await.remove(token);
    }
}

pub async fn cache_authenticated_user(state: &AppState, token: &str, user: CurrentUser) {
    if token.is_empty() {
        return;
    }

    let now = Instant::now();
    let mut cache = state.auth_cache.write().await;
    cache.retain(|_, entry| entry.expires_at > now);

    if cache.len() >= AUTH_CACHE_MAX_ENTRIES {
        let mut oldest_key = None::<String>;
        let mut oldest_expiry = None::<Instant>;
        for (key, entry) in cache.iter() {
            if oldest_expiry
                .map(|expiry| entry.expires_at < expiry)
                .unwrap_or(true)
            {
                oldest_expiry = Some(entry.expires_at);
                oldest_key = Some(key.clone());
            }
        }
        if let Some(key) = oldest_key {
            cache.remove(&key);
        }
    }

    cache.insert(
        token.to_string(),
        AuthCacheEntry {
            user,
            expires_at: now + AUTH_CACHE_TTL,
        },
    );
}

pub async fn resolve_user_by_token(
    state: &AppState,
    incoming_token: &str,
) -> Result<Option<CurrentUser>, ApiError> {
    if incoming_token.is_empty() {
        return Ok(None);
    }

    let now = Instant::now();
    {
        let mut cache = state.auth_cache.write().await;
        if let Some(entry) = cache.get_mut(incoming_token) {
            if entry.expires_at > now {
                entry.expires_at = now + AUTH_CACHE_TTL;
                return Ok(Some(entry.user.clone()));
            }
            cache.remove(incoming_token);
        }
    }

    let user = load_user_by_persistent_token(&state.db, incoming_token).await?;
    if let Some(user) = user.clone() {
        if user.status == 1 {
            cache_authenticated_user(state, incoming_token, user.clone()).await;
        } else {
            invalidate_cached_token(state, Some(incoming_token)).await;
        }
    }
    Ok(user)
}

fn password_change_allowed_path(path: &str) -> bool {
    matches!(
        path,
        "/api/user/getInfo" | "/api/user/getAuthInfo" | "/api/user/updatePassword" | "/api/logout"
    )
}

pub async fn enforce_password_change_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    if request.method() == Method::OPTIONS {
        return next.run(request).await;
    }

    if let Some(token) = request_token_value(request.headers()) {
        match resolve_user_by_token(&state, &token).await {
            Ok(Some(user))
                if user.status == 1
                    && user.must_change_password == 1
                    && !password_change_allowed_path(request.uri().path()) =>
            {
                return ApiError::password_change_required().into_response();
            }
            Ok(_) => {}
            Err(err) => return err.into_response(),
        }
    }

    next.run(request).await
}

pub async fn verify_password(plain: &str, stored: &str) -> bool {
    bcrypt::verify(plain, stored).unwrap_or(false)
}

pub fn validate_register_username(username: &str) -> Result<(), ApiError> {
    if !(3..=80).contains(&username.len()) {
        return Err(ApiError::bad_param(
            "Username length must be between 3 and 80 characters",
        ));
    }
    if !username
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '@'))
    {
        return Err(ApiError::bad_param(
            "Username can only contain letters, numbers, _, . and @",
        ));
    }
    Ok(())
}

fn validate_register_password(password: &str) -> Result<(), ApiError> {
    if !(8..=64).contains(&password.len()) {
        return Err(ApiError::bad_param(
            "Password length must be between 8 and 64 characters",
        ));
    }
    if password.chars().any(char::is_whitespace) {
        return Err(ApiError::bad_param("Password cannot contain whitespace"));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PasswordPolicy {
    allow_weak_password: bool,
}

impl Default for PasswordPolicy {
    fn default() -> Self {
        Self {
            allow_weak_password: false,
        }
    }
}

async fn get_password_policy(db: &SqlitePool) -> Result<PasswordPolicy, ApiError> {
    let raw = get_setting(db, "security_password_policy").await?;
    Ok(raw
        .as_deref()
        .and_then(|value| serde_json::from_str::<PasswordPolicy>(value).ok())
        .unwrap_or_default())
}

fn is_weak_password(password: &str) -> bool {
    let lower = password.to_ascii_lowercase();
    let common_passwords = [
        "123456",
        "12345678",
        "password",
        "password123",
        "qwerty",
        "qwerty123",
        "admin",
        "admin123",
        "111111",
        "000000",
        "abc123",
        "letmein",
    ];

    if common_passwords.contains(&lower.as_str()) {
        return true;
    }

    let mut kinds = 0;
    if password.chars().any(|ch| ch.is_ascii_lowercase()) {
        kinds += 1;
    }
    if password.chars().any(|ch| ch.is_ascii_uppercase()) {
        kinds += 1;
    }
    if password.chars().any(|ch| ch.is_ascii_digit()) {
        kinds += 1;
    }
    if password.chars().any(|ch| !ch.is_ascii_alphanumeric()) {
        kinds += 1;
    }

    kinds < 3
}

pub async fn validate_password_by_policy(
    db: &SqlitePool,
    password: &str,
) -> Result<(), ApiError> {
    validate_register_password(password)?;
    let policy = get_password_policy(db).await?;
    if !policy.allow_weak_password && is_weak_password(password) {
        return Err(ApiError::bad_param(
            "Password is too weak. Use at least three character types: uppercase, lowercase, numbers, and symbols",
        ));
    }
    Ok(())
}

pub fn validate_register_email(email: &str) -> Result<(), ApiError> {
    let email_regex = Regex::new(r"^\w+([-.+]\w+)*@\w+([-.]\w+)*\.\w+([-.]\w+)*$")
        .expect("email regex must compile");
    if !email_regex.is_match(email) {
        return Err(ApiError::bad_param("Invalid email address"));
    }
    Ok(())
}

pub async fn authenticate(
    headers: &HeaderMap,
    state: &AppState,
    _mode: AccessMode,
) -> Result<AuthContext, ApiError> {
    if let Some(incoming_token) = request_token_value(headers) {
        if let Some(user) = resolve_user_by_token(state, &incoming_token).await? {
            if user.status == 1 {
                return Ok(AuthContext {
                    user,
                    visit_mode: 0,
                });
            }
            invalidate_cached_token(state, Some(&incoming_token)).await;
            return Err(ApiError::new(1004, "Account disabled or not activated"));
        }
        return Err(ApiError::new(1001, "Not logged in yet"));
    }
    Err(ApiError::new(1000, "Not logged in yet"))
}

pub fn ensure_admin(auth: &AuthContext) -> Result<(), ApiError> {
    if auth.user.role != 1 {
        Err(ApiError::new(1005, "No current permission for operation"))
    } else {
        Ok(())
    }
}
