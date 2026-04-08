use std::net::IpAddr;

use axum::{extract::State, http::HeaderMap, response::IntoResponse, Json};
use bcrypt::hash;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    auth::{
        build_cleared_session_cookie, build_session_cookie, cache_authenticated_user,
        invalidate_cached_token, random_token, request_is_https, request_token_value,
        validate_password_by_policy, validate_register_email, validate_register_username,
        verify_password,
    },
    db::{get_setting, load_user_by_mail, load_user_by_username, set_setting},
    error::{ok, ok_empty, with_set_cookie, ApiError, ApiResult},
    models::{default_system_application_value, parse_register_config, AppState},
    utils::{extract_client_ip, is_private_ip},
};

#[derive(serde::Serialize)]
pub struct CryptoKeyResponse {
    code: i32,
    msg: String,
    data: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterCommitRequest {
    username: String,
    password: String,
    email: String,
    #[allow(dead_code)]
    referral_code: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    username: String,
    password: String,
    #[allow(dead_code)]
    vcode: Option<String>,
}

pub async fn get_crypto_key(State(state): State<AppState>) -> ApiResult {
    let key = if let Some(ref configured_key) = state.config.crypto_key {
        configured_key.clone()
    } else {
        match get_setting(&state.db, "public_crypto_key").await? {
            Some(key) if !key.trim().is_empty() => key,
            _ => {
                let generated = random_token(64);
                set_setting(&state.db, "public_crypto_key", &generated).await?;
                generated
            }
        }
    };
    Ok(Json(CryptoKeyResponse {
        code: 200,
        msg: "success".to_string(),
        data: Some(key),
    })
    .into_response())
}

pub async fn register_commit(
    State(state): State<AppState>,
    Json(req): Json<RegisterCommitRequest>,
) -> ApiResult {
    let raw = get_setting(&state.db, "system_application")
        .await?
        .unwrap_or_else(|| default_system_application_value().to_string());
    let value = serde_json::from_str::<Value>(&raw)
        .unwrap_or_else(|_| default_system_application_value());
    let register_config = parse_register_config(value.get("register"));
    if !register_config.open_register {
        return Err(ApiError::new(1403, "注册已关闭"));
    }

    let username = req.username.trim();
    let password = req.password.trim();
    let email = req.email.trim().to_lowercase();

    validate_register_username(username)?;
    validate_password_by_policy(&state.db, password).await?;
    validate_register_email(&email)?;

    if !register_config.email_suffix.trim().is_empty() {
        let suffix = register_config.email_suffix.trim().to_lowercase();
        if !email.ends_with(&suffix) {
            return Err(ApiError::bad_param(format!("邮箱必须以 {} 结尾", suffix)));
        }
    }

    if load_user_by_username(&state.db, username).await?.is_some() {
        return Err(ApiError::new(1401, "账号已存在"));
    }
    if load_user_by_mail(&state.db, &email).await?.is_some() {
        return Err(ApiError::new(1401, "邮箱已存在"));
    }

    let password_hash = hash(password, 12).map_err(|e| ApiError::internal(e.to_string()))?;
    let result = sqlx::query(
        "INSERT INTO user (username, password, name, status, role, mail, token, created_at, updated_at) VALUES (?, ?, ?, 1, 2, ?, '', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    )
    .bind(username)
    .bind(password_hash)
    .bind(username)
    .bind(&email)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::db(e.to_string()))?;

    Ok(ok(json!({
        "id": result.last_insert_rowid(),
        "userId": result.last_insert_rowid(),
        "username": username,
        "name": username,
        "mail": email,
    })))
}

pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<LoginRequest>,
) -> ApiResult {
    let username = req.username.trim();
    let Some(user) = load_user_by_username(&state.db, username).await? else {
        return Err(ApiError::new(1003, "用户名或密码错误"));
    };

    if !verify_password(&req.password, &user.password).await {
        return Err(ApiError::new(1003, "用户名或密码错误"));
    }
    if user.status != 1 {
        return Err(ApiError::new(1004, "账号已停用或未激活"));
    }

    let previous_token = user.token.clone().filter(|value| !value.is_empty());
    let persistent_token = random_token(48);
    sqlx::query("UPDATE user SET token = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(&persistent_token)
        .bind(user.id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;
    invalidate_cached_token(&state, previous_token.as_deref()).await;

    let mut authenticated_user = user.clone();
    authenticated_user.token = Some(persistent_token.clone());
    cache_authenticated_user(&state, &persistent_token, authenticated_user.clone()).await;

    let response = ok(json!({
        "id": authenticated_user.id,
        "userId": authenticated_user.id,
        "username": authenticated_user.username,
        "name": authenticated_user.name,
        "headImage": authenticated_user.head_image,
        "role": authenticated_user.role,
        "mail": authenticated_user.mail,
        "mustChangePassword": authenticated_user.must_change_password == 1,
    }));

    with_set_cookie(
        response,
        &build_session_cookie(&persistent_token, request_is_https(&headers)),
    )
}

pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> ApiResult {
    if let Some(token) = request_token_value(&headers) {
        invalidate_cached_token(&state, Some(&token)).await;
        let _ = sqlx::query("UPDATE user SET token = '' WHERE token = ?")
            .bind(&token)
            .execute(&state.db)
            .await;
    }
    with_set_cookie(
        ok_empty(),
        &build_cleared_session_cookie(request_is_https(&headers)),
    )
}

pub async fn about() -> ApiResult {
    Ok(ok(json!({
        "versionName": env!("CARGO_PKG_VERSION"),
        "versionCode": 1,
    })))
}

pub async fn is_lan(headers: HeaderMap) -> ApiResult {
    let ip = extract_client_ip(&headers);
    let is_lan = ip
        .as_deref()
        .and_then(|s| s.parse::<IpAddr>().ok())
        .map(is_private_ip)
        .unwrap_or(false);
    Ok(ok(json!({ "isLan": is_lan, "clientIp": ip })))
}

pub async fn ping() -> &'static str {
    "pong"
}
