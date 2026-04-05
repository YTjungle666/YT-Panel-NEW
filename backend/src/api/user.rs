use crate::{
    auth::{authenticate, load_user_by_id, SessionManager},
    db::{get_setting, get_user_config, set_user_config},
    error::{ok, ok_empty, ApiError, ApiResult},
    models::{AccessMode, AuthContext},
    state::AppState,
};
use axum::{
    extract::{Json, State},
    http::HeaderMap,
    routing::post,
    Router,
};
use bcrypt::{hash, DEFAULT_COST};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::query_scalar;

#[derive(Debug, Deserialize)]
struct UpdateInfoRequest {
    name: Option<String>,
    head_image: Option<String>,
    mail: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdatePasswordRequest {
    old_password: String,
    new_password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UserInfoResponse {
    id: i64,
    username: String,
    name: String,
    head_image: Option<String>,
    mail: Option<String>,
    referral_code: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/user/getInfo", post(user_get_info))
        .route("/api/user/getAuthInfo", post(user_get_auth_info))
        .route("/api/user/updateInfo", post(user_update_info))
        .route("/api/user/updatePassword", post(user_update_password))
        .route("/api/user/getReferralCode", post(user_get_referral_code))
}

async fn user_get_info(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let user = auth.user;

    ok(UserInfoResponse {
        id: user.id,
        username: user.username,
        name: user.name,
        head_image: user.head_image,
        mail: user.mail,
        referral_code: user.referral_code,
    })
}

async fn user_get_auth_info(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let user = auth.user;

    ok(json!({
        "id": user.id,
        "username": user.username,
        "name": user.name,
        "headImage": user.head_image,
        "mail": user.mail,
        "referralCode": user.referral_code,
        "role": user.role,
        "visitMode": auth.visit_mode
    }))
}

async fn user_update_info(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<UpdateInfoRequest>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    
    sqlx::query(
        "UPDATE user SET name = COALESCE(?, name), head_image = COALESCE(?, head_image), mail = COALESCE(?, mail), update_time = CURRENT_TIMESTAMP WHERE id = ?"
    )
    .bind(req.name)
    .bind(req.head_image)
    .bind(req.mail)
    .bind(auth.user.id)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::db(e.to_string()))?;

    ok_empty()
}

async fn user_update_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<UpdatePasswordRequest>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    
    // Verify old password
    bcrypt::verify(&req.old_password, &auth.user.password)
        .map_err(|_| ApiError::new(1004, "Invalid old password"))?;

    let hashed = hash(&req.new_password, DEFAULT_COST)
        .map_err(|_| ApiError::internal("Failed to hash password"))?;

    sqlx::query("UPDATE user SET password = ?, update_time = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(&hashed)
        .bind(auth.user.id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;

    ok_empty()
}

async fn user_get_referral_code(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    
    let code = if let Some(ref code) = auth.user.referral_code {
        code.clone()
    } else {
        let new_code: String = rand::random::<u32>().to_string();
        sqlx::query("UPDATE user SET referral_code = ? WHERE id = ?")
            .bind(&new_code)
            .bind(auth.user.id)
            .execute(&state.db)
            .await
            .map_err(|e| ApiError::db(e.to_string()))?;
        new_code
    };

    ok(json!({ "referralCode": code }))
}
