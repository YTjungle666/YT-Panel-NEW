//! Auth handlers - 登录/注册相关接口

use axum::{
    extract::State,
    http::HeaderMap,
    routing::post,
    Json, Router,
};
use bcrypt::hash;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    AppState, ApiError, ApiResult,
    authenticate, AccessMode, AuthContext,
    get_setting, set_setting, parse_register_config,
    validate_register_username, validate_register_password, validate_register_email,
    default_system_application_value, password_reset_not_configured,
    verify_password_compat, random_token, ok, ok_empty, 
    with_set_cookie, build_session_cookie, build_cleared_session_cookie,
    load_user_by_username, load_user_by_mail, load_user_by_persistent_token,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterCommitRequest {
    pub username: String,
    pub password: String,
    pub email: String,
    #[allow(dead_code)]
    pub email_vcode: Option<String>,
    #[allow(dead_code)]
    pub vcode: Option<String>,
    #[allow(dead_code)]
    pub referral_code: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendResetPasswordVCodeRequest {
    pub email: String,
    #[allow(dead_code)]
    pub verification: Option<Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetPasswordByVCodeRequest {
    pub email: String,
    pub password: String,
    #[serde(rename = "emailVCode", alias = "emailVcode")]
    pub email_vcode: Option<String>,
    #[allow(dead_code)]
    pub verification: Option<Value>,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    #[allow(dead_code)]
    pub vcode: Option<String>,
}

/// 用户注册
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
        return Err(ApiError::new(1403, "Registration is disabled"));
    }

    let username = req.username.trim();
    let password = req.password.trim();
    let email = req.email.trim().to_lowercase();

    validate_register_username(username)?;
    validate_register_password(password)?;
    validate_register_email(&email)?;

    if !register_config.email_suffix.trim().is_empty() {
        let suffix = register_config.email_suffix.trim().to_lowercase();
        if !email.ends_with(&suffix) {
            return Err(ApiError::bad_param(format!("Email must end with {}", suffix)));
        }
    }

    if load_user_by_username(&state.db, username).await?.is_some() {
        return Err(ApiError::new(1401, "The username already exists"));
    }
    if load_user_by_mail(&state.db, &email).await?.is_some() {
        return Err(ApiError::new(1401, "The email already exists"));
    }

    let password_hash = hash(password, 12)
        .map_err(|e| ApiError::new(-1, e.to_string()))?;
    let token = random_token(48);

    let result = sqlx::query(
        "INSERT INTO user (username, password, name, status, role, mail, token, created_at, updated_at) 
         VALUES (?, ?, ?, 1, 2, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)"
    )
    .bind(username)
    .bind(password_hash)
    .bind(username)
    .bind(&email)
    .bind(token)
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

/// 发送重置密码验证码
pub async fn login_send_reset_password_vcode(
    Json(req): Json<SendResetPasswordVCodeRequest>,
) -> ApiResult {
    let email = req.email.trim().to_lowercase();
    if email.is_empty() {
        return Err(ApiError::bad_param("Email is required"));
    }
    validate_register_email(&email)?;
    Err(password_reset_not_configured())
}

/// 通过验证码重置密码
pub async fn login_reset_password_by_vcode(
    Json(req): Json<ResetPasswordByVCodeRequest>,
) -> ApiResult {
    let email = req.email.trim().to_lowercase();
    let password = req.password.trim();
    let email_vcode = req.email_vcode.as_deref().unwrap_or("").trim();

    if email.is_empty() {
        return Err(ApiError::bad_param("Email is required"));
    }
    validate_register_email(&email)?;
    validate_register_password(password)?;
    if email_vcode.is_empty() {
        return Err(ApiError::bad_param("Email verification code is required"));
    }
    Err(password_reset_not_configured())
}

/// 用户登录
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> ApiResult {
    let username = req.username.trim();
    let Some(user) = load_user_by_username(&state.db, username).await? else {
        return Err(ApiError::new(1003, "Incorrect username or password"));
    };

    if !verify_password_compat(&req.password, &user.password).await {
        return Err(ApiError::new(1003, "Incorrect username or password"));
    }
    if user.status != 1 {
        return Err(ApiError::new(1004, "Account disabled or not activated"));
    }

    // 升级旧密码格式
    if !user.password.starts_with("$2") {
        let new_hash = hash(&req.password, 12)
            .map_err(|e| ApiError::new(-1, e.to_string()))?;
        sqlx::query("UPDATE user SET password = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(new_hash)
            .bind(user.id)
            .execute(&state.db)
            .await
            .map_err(|e| ApiError::db(e.to_string()))?;
    }

    let persistent_token = if let Some(token) = user.token.clone().filter(|s| !s.is_empty()) {
        token
    } else {
        let token = random_token(48);
        sqlx::query("UPDATE user SET token = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(&token)
            .bind(user.id)
            .execute(&state.db)
            .await
            .map_err(|e| ApiError::db(e.to_string()))?;
        token
    };

    let response = ok(json!({
        "id": user.id,
        "userId": user.id,
        "username": user.username,
        "name": user.name,
        "headImage": user.head_image,
        "role": user.role,
        "mail": user.mail,
    }));

    with_set_cookie(response, &build_session_cookie(&persistent_token))
}

/// 用户登出
pub async fn logout(State(_state): State<AppState>, _headers: HeaderMap) -> ApiResult {
    with_set_cookie(ok_empty(), &build_cleared_session_cookie())
}

/// 认证路由
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/login", post(login))
        .route("/api/logout", post(logout))
        .route("/api/register/commit", post(register_commit))
        .route("/api/login/sendResetPasswordVCode", post(login_send_reset_password_vcode))
        .route("/api/login/resetPasswordByVCode", post(login_reset_password_by_vcode))
}
