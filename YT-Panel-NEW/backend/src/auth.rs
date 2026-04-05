use crate::{
    error::{ApiError, ApiResult},
    models::{AccessMode, AuthContext, CurrentUser},
    state::AppState,
};
use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue},
    response::Response,
};
use serde::Serialize;
use sqlx::{query_as, Row, SqlitePool};
use std::collections::HashMap;

// Password hashing utilities using bcrypt
// These functions use spawn_blocking to avoid blocking the async runtime

/// Hash a password using bcrypt (non-blocking)
pub async fn hash_password(password: &str) -> Result<String, ApiError> {
    let password = password.to_string();
    tokio::task::spawn_blocking(move || {
        bcrypt::hash(&password, bcrypt::DEFAULT_COST)
    })
    .await
    .map_err(|e| ApiError::internal(format!("Password hashing failed: {}", e)))?
    .map_err(|e| ApiError::internal(format!("Password hashing error: {}", e)))
}

/// Verify a password against a hash (non-blocking)
pub async fn verify_password(password: &str, hash: &str) -> Result<bool, ApiError> {
    let password = password.to_string();
    let hash = hash.to_string();
    tokio::task::spawn_blocking(move || {
        bcrypt::verify(&password, &hash)
    })
    .await
    .map_err(|e| ApiError::internal(format!("Password verification failed: {}", e)))?
    .map_err(|e| ApiError::internal(format!("Password verification error: {}", e)))
}


const SESSION_COOKIE_NAME: &str = "yt_panel_session";
const SESSION_COOKIE_MAX_AGE: i64 = 60 * 60 * 24 * 30; // 30 days

#[derive(Debug, Clone)]
pub struct SessionManager;

impl SessionManager {
    pub fn get_token(headers: &HeaderMap) -> Option<String> {
        // Priority: Cookie > Authorization header > token header
        Self::session_cookie_value(headers)
            .or_else(|| Self::bearer_token(headers))
            .or_else(|| Self::header_token(headers))
    }

    fn session_cookie_value(headers: &HeaderMap) -> Option<String> {
        headers
            .get(header::COOKIE)
            .and_then(|v| v.to_str().ok())
            .and_then(|cookies| {
                cookies.split(';').find_map(|cookie| {
                    let mut parts = cookie.trim().splitn(2, '=');
                    let name = parts.next()?;
                    let value = parts.next()?;
                    if name == SESSION_COOKIE_NAME {
                        Some(value.to_string())
                    } else {
                        None
                    }
                })
            })
    }

    fn bearer_token(headers: &HeaderMap) -> Option<String> {
        headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|value| {
                value
                    .strip_prefix("Bearer ")
                    .or_else(|| value.strip_prefix("bearer "))
            })
            .map(|v| v.trim().to_string())
    }

    fn header_token(headers: &HeaderMap) -> Option<String> {
        headers
            .get("token")
            .and_then(|v| v.to_str().ok())
            .map(|v| v.to_string())
    }

    pub fn build_session_cookie(token: &str) -> String {
        format!(
            "{}={}; Max-Age={}; Path=/; HttpOnly; SameSite=Lax",
            SESSION_COOKIE_NAME, token, SESSION_COOKIE_MAX_AGE
        )
    }

    pub fn build_cleared_session_cookie() -> String {
        format!("{}=; Max-Age=0; Path=/; HttpOnly; SameSite=Lax", SESSION_COOKIE_NAME)
    }

    pub fn with_cookie(response: Response, cookie: &str) -> Response {
        let mut resp = response;
        if let Ok(value) = HeaderValue::from_str(cookie) {
            resp.headers_mut().insert(header::SET_COOKIE, value);
        }
        resp
    }
}

/// Authenticate user based on headers and access mode
pub async fn authenticate(
    headers: &HeaderMap,
    state: &AppState,
    mode: AccessMode,
) -> Result<AuthContext, ApiError> {
    let incoming_token = SessionManager::get_token(headers).unwrap_or_default();

    if !incoming_token.is_empty() {
        // Check session mapping first
        let sessions = state.sessions.read().await;
        if let Some(mapped) = sessions.get(&incoming_token) {
            if let Some(user) = load_user_by_persistent_token(&state.db, mapped).await? {
                return Ok(AuthContext { user, visit_mode: 0 });
            }
        }
        drop(sessions);

        // Try direct token lookup
        if let Some(user) = load_user_by_persistent_token(&state.db, &incoming_token).await? {
            return Ok(AuthContext { user, visit_mode: 0 });
        }
    }

    // Check if public user is configured and allowed
    if let AccessMode::PublicAllowed = mode {
        if let Some(public_id) = state.config.public_user_id {
            if let Some(user) = load_user_by_id(&state.db, public_id).await? {
                return Ok(AuthContext { user, visit_mode: 1 });
            }
        }
    }

    if matches!(mode, AccessMode::LoginRequired) {
        return Err(ApiError::unauthorized("Not logged in yet"));
    }

    Err(ApiError::unauthorized("Not logged in yet"))
}

/// Invalidate session mappings for a given persistent token
pub async fn invalidate_session_mappings(
    state: &AppState,
    persistent_token: Option<&str>,
) {
    if let Some(token) = persistent_token {
        // LRU cache 会自动淘汰，这里可以查找并删除特定会话
        // 但为了性能，我们依赖 LRU 自动淘汰 + 重新登录
        // 如果需要强制登出，可以记录黑名单
        let _ = token;
    }
}

pub async fn load_user_by_id(db: &SqlitePool, id: i64) -> Result<Option<CurrentUser>, ApiError> {
    let row = sqlx::query(
        r#"SELECT id, username, password, name, head_image, status, role, mail, referral_code, token 
           FROM user WHERE id = ?"#,
    )
    .bind(id)
    .fetch_optional(db)
    .await
    .map_err(|e| ApiError::db(e.to_string()))?;

    Ok(row.map(|r| CurrentUser {
        id: r.get("id"),
        username: r.get("username"),
        password: r.get("password"),
        name: r.get("name"),
        head_image: r.get("head_image"),
        status: r.get("status"),
        role: r.get("role"),
        mail: r.get("mail"),
        referral_code: r.get("referral_code"),
        token: r.get("token"),
    }))
}

pub async fn load_user_by_persistent_token(
    db: &SqlitePool,
    token: &str,
) -> Result<Option<CurrentUser>, ApiError> {
    let row = sqlx::query(
        r#"SELECT id, username, password, name, head_image, status, role, mail, referral_code, token 
           FROM user WHERE token = ? AND status = 1"#,
    )
    .bind(token)
    .fetch_optional(db)
    .await
    .map_err(|e| ApiError::db(e.to_string()))?;

    Ok(row.map(|r| CurrentUser {
        id: r.get("id"),
        username: r.get("username"),
        password: r.get("password"),
        name: r.get("name"),
        head_image: r.get("head_image"),
        status: r.get("status"),
        role: r.get("role"),
        mail: r.get("mail"),
        referral_code: r.get("referral_code"),
        token: r.get("token"),
    }))
}

pub async fn load_user_by_username(
    db: &SqlitePool,
    username: &str,
) -> Result<Option<CurrentUser>, ApiError> {
    let row = sqlx::query(
        r#"SELECT id, username, password, name, head_image, status, role, mail, referral_code, token 
           FROM user WHERE username = ?"#,
    )
    .bind(username)
    .fetch_optional(db)
    .await
    .map_err(|e| ApiError::db(e.to_string()))?;

    Ok(row.map(|r| CurrentUser {
        id: r.get("id"),
        username: r.get("username"),
        password: r.get("password"),
        name: r.get("name"),
        head_image: r.get("head_image"),
        status: r.get("status"),
        role: r.get("role"),
        mail: r.get("mail"),
        referral_code: r.get("referral_code"),
        token: r.get("token"),
    }))
}
