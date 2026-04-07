use axum::http::{header::AUTHORIZATION, HeaderMap};
use rand::{distributions::Alphanumeric, Rng};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::error::ApiError;
use crate::{
    db::{get_setting, load_user_by_id, load_user_by_persistent_token, parse_public_user_id_setting},
    models::{AccessMode, AppState, AuthContext},
};

pub const SESSION_COOKIE_NAME: &str = "yt_panel_session";
pub const SESSION_COOKIE_MAX_AGE: i64 = 60 * 60 * 24 * 30;

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
    mode: AccessMode,
) -> Result<AuthContext, ApiError> {
    if let Some(incoming_token) = request_token_value(headers) {
        if let Some(cached_user) = state.auth_cache.read().await.get(&incoming_token).cloned() {
            if cached_user.status == 1 {
                return Ok(AuthContext {
                    user: cached_user,
                    visit_mode: 0,
                });
            }
            state.auth_cache.write().await.remove(&incoming_token);
        }

        if let Some(user) = load_user_by_persistent_token(&state.db, &incoming_token).await? {
            if user.status == 1 {
                state
                    .auth_cache
                    .write()
                    .await
                    .insert(incoming_token.clone(), user.clone());
                return Ok(AuthContext { user, visit_mode: 0 });
            }

            state.auth_cache.write().await.remove(&incoming_token);
            if matches!(mode, AccessMode::LoginRequired) {
                return Err(ApiError::new(1004, "Account disabled or not activated"));
            }
        }

        if matches!(mode, AccessMode::LoginRequired) {
            return Err(ApiError::new(1001, "Not logged in yet"));
        }
    }

    if matches!(mode, AccessMode::LoginRequired) {
        return Err(ApiError::new(1000, "Not logged in yet"));
    }

    let public_user_id = match get_setting(&state.db, "panel_public_user_id").await? {
        Some(value) => parse_public_user_id_setting(&value),
        None => state.config.public_user_id,
    }
    .ok_or_else(|| ApiError::new(1001, "Not logged in yet"))?;

    let user = load_user_by_id(&state.db, public_user_id)
        .await?
        .ok_or_else(|| ApiError::new(1001, "Not logged in yet"))?;
    if user.status != 1 {
        return Err(ApiError::new(1001, "Not logged in yet"));
    }

    Ok(AuthContext { user, visit_mode: 1 })
}

pub fn ensure_admin(auth: &AuthContext) -> Result<(), ApiError> {
    if auth.user.role != 1 {
        Err(ApiError::new(1005, "No current permission for operation"))
    } else {
        Ok(())
    }
}
