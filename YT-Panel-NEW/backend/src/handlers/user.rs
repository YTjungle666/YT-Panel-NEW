//! User handlers - 用户信息相关接口

use axum::{
    extract::State,
    http::HeaderMap,
    routing::post,
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use crate::{
    AppState, ApiResult,
    authenticate, AccessMode,
    ok, ok_empty,
    load_user_by_id,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfoRequest {
    pub name: String,
    pub head_image: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

/// 获取用户信息
pub async fn user_get_info(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    Ok(ok(json!({
        "userId": auth.user.id,
        "id": auth.user.id,
        "headImage": auth.user.head_image,
        "name": auth.user.name,
        "role": auth.user.role,
    })))
}

/// 获取认证信息（用于公开访问）
pub async fn user_get_auth_info(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::PublicAllowed).await?;
    Ok(ok(json!({
        "userId": auth.user.id,
        "id": auth.user.id,
        "headImage": auth.user.head_image,
        "name": auth.user.name,
        "role": auth.user.role,
        "visitMode": auth.visit_mode,
    })))
}

/// 更新用户信息
pub async fn user_update_info(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<UpdateInfoRequest>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    
    sqlx::query(
        "UPDATE user SET name = ?, head_image = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?"
    )
    .bind(&req.name)
    .bind(&req.head_image)
    .bind(auth.user.id)
    .execute(&state.db)
    .await
    .map_err(|e| crate::ApiError::db(e.to_string()))?;

    Ok(ok_empty())
}

/// 更新密码
pub async fn user_update_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<UpdatePasswordRequest>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    
    // 验证旧密码
    if !crate::verify_password_compat(&req.old_password, &auth.user.password).await {
        return Err(crate::ApiError::new(1003, "Old password is incorrect"));
    }

    let new_hash = bcrypt::hash(&req.new_password, 12)
        .map_err(|e| crate::ApiError::new(-1, e.to_string()))?;

    sqlx::query("UPDATE user SET password = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(new_hash)
        .bind(auth.user.id)
        .execute(&state.db)
        .await
        .map_err(|e| crate::ApiError::db(e.to_string()))?;

    Ok(ok_empty())
}

/// 获取推荐码
pub async fn user_get_referral_code(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    
    // 如果没有推荐码，生成一个
    let code = if let Some(ref code) = auth.user.referral_code {
        code.clone()
    } else {
        let new_code = crate::random_token(8);
        sqlx::query("UPDATE user SET referral_code = ? WHERE id = ?")
            .bind(&new_code)
            .bind(auth.user.id)
            .execute(&state.db)
            .await
            .map_err(|e| crate::ApiError::db(e.to_string()))?;
        new_code
    };

    Ok(ok(json!({ "referralCode": code })))
}

/// 用户路由
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/user/getInfo", post(user_get_info))
        .route("/api/user/getAuthInfo", post(user_get_auth_info))
        .route("/api/user/updateInfo", post(user_update_info))
        .route("/api/user/updatePassword", post(user_update_password))
        .route("/api/user/getReferralCode", post(user_get_referral_code))
}
