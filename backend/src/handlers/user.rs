use axum::{extract::State, http::HeaderMap, Json};
use bcrypt::hash;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    auth::{
        authenticate, build_cleared_session_cookie, ensure_admin, invalidate_cached_token,
        random_token, request_is_https, validate_password_by_policy, validate_register_email,
        validate_register_username, verify_password,
    },
    db::{load_user_by_id, load_user_by_mail, load_user_by_username},
    error::{list_ok, ok, ok_empty, with_set_cookie, ApiError, ApiResult},
    models::{build_user_payload, row_to_user_payload, AccessMode, AppState},
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfoRequest {
    head_image: Option<String>,
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePasswordRequest {
    old_password: String,
    new_password: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AdminUserUpsertRequest {
    id: Option<i64>,
    username: String,
    password: Option<String>,
    name: Option<String>,
    head_image: Option<String>,
    status: Option<i64>,
    role: Option<i64>,
    mail: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AdminUsersListRequest {
    page: Option<i64>,
    limit: Option<i64>,
    #[serde(rename = "keyword", alias = "keyWord")]
    keyword: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminUsersDeleteRequest {
    user_ids: Vec<i64>,
}

pub async fn user_get_info(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    Ok(ok(json!({
        "userId": auth.user.id,
        "id": auth.user.id,
        "username": auth.user.username,
        "headImage": auth.user.head_image,
        "name": auth.user.name,
        "role": auth.user.role,
        "mail": auth.user.mail,
        "mustChangePassword": auth.user.must_change_password == 1,
    })))
}

pub async fn user_get_auth_info(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    Ok(ok(json!({
        "user": {
            "id": auth.user.id,
            "username": auth.user.username,
            "name": auth.user.name,
            "headImage": auth.user.head_image,
            "role": auth.user.role,
            "mustChangePassword": auth.user.must_change_password == 1,
        },
        "visitMode": auth.visit_mode,
    })))
}

pub async fn user_update_info(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<UpdateInfoRequest>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    if req.name.trim().len() < 2 || req.name.trim().len() > 15 {
        return Err(ApiError::bad_param("name length invalid"));
    }
    sqlx::query(
        "UPDATE user SET head_image = ?, name = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(req.head_image)
    .bind(req.name.trim())
    .bind(auth.user.id)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::db(e.to_string()))?;
    invalidate_cached_token(&state, auth.user.token.as_deref()).await;
    Ok(ok_empty())
}

pub async fn user_update_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<UpdatePasswordRequest>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let Some(fresh) = load_user_by_id(&state.db, auth.user.id).await? else {
        return Err(ApiError::new(1006, "Account does not exist"));
    };
    if !verify_password(&req.old_password, &fresh.password).await {
        return Err(ApiError::new(1007, "Old password error"));
    }
    validate_password_by_policy(&state.db, &req.new_password).await?;
    let new_hash = hash(req.new_password.trim(), 12).map_err(|e| ApiError::internal(e.to_string()))?;
    sqlx::query(
        "UPDATE user SET password = ?, token = '', must_change_password = 0, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(new_hash)
    .bind(auth.user.id)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::db(e.to_string()))?;
    invalidate_cached_token(&state, fresh.token.as_deref()).await;
    with_set_cookie(
        ok_empty(),
        &build_cleared_session_cookie(request_is_https(&headers)),
    )
}

pub async fn user_get_referral_code(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    if let Some(code) = auth.user.referral_code.filter(|v| !v.is_empty()) {
        return Ok(ok(json!({ "referralCode": code })));
    }
    let code = random_token(8).to_uppercase();
    sqlx::query("UPDATE user SET referral_code = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(&code)
        .bind(auth.user.id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;
    Ok(ok(json!({ "referralCode": code })))
}

pub async fn panel_users_create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<AdminUserUpsertRequest>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    ensure_admin(&auth)?;

    let username = req.username.trim();
    if username.len() < 5 {
        return Err(ApiError::bad_param("The account must be no less than 5 characters long"));
    }
    validate_register_username(username)?;

    let password = req.password.as_deref().unwrap_or("").trim();
    if password.is_empty() {
        return Err(ApiError::bad_param("Password is required"));
    }
    validate_password_by_policy(&state.db, password).await?;

    if load_user_by_username(&state.db, username).await?.is_some() {
        return Err(ApiError::new(1401, "The username already exists"));
    }

    let mail = req
        .mail
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_lowercase);
    if let Some(mail_value) = mail.as_deref() {
        validate_register_email(mail_value)?;
        if load_user_by_mail(&state.db, mail_value).await?.is_some() {
            return Err(ApiError::new(1401, "The email already exists"));
        }
    }

    let role = req.role.unwrap_or(2).clamp(1, 2);
    let status = req.status.unwrap_or(1);
    let name = req
        .name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(username)
        .to_string();
    let password_hash = hash(password, 12).map_err(|e| ApiError::internal(e.to_string()))?;

    let result = sqlx::query(
        "INSERT INTO user (username, password, name, head_image, status, role, mail, token, must_change_password, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, '', 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    )
    .bind(username)
    .bind(password_hash)
    .bind(&name)
    .bind(req.head_image.clone())
    .bind(status)
    .bind(role)
    .bind(mail.clone())
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::db(e.to_string()))?;

    let id = result.last_insert_rowid();
    Ok(ok(build_user_payload(
        id,
        username.to_string(),
        name,
        req.head_image,
        status,
        role,
        mail,
        None,
        None,
        None,
        0,
    )))
}

pub async fn panel_users_update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<AdminUserUpsertRequest>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    ensure_admin(&auth)?;

    let id = req.id.unwrap_or_default();
    if id <= 0 {
        return Err(ApiError::bad_param("User id is required"));
    }

    let Some(existing) = load_user_by_id(&state.db, id).await? else {
        return Err(ApiError::new(1006, "Account does not exist"));
    };

    let username = req.username.trim();
    if username.len() < 3 {
        return Err(ApiError::bad_param("The account must be no less than 3 characters long"));
    }
    validate_register_username(username)?;

    if let Some(found) = load_user_by_username(&state.db, username).await? {
        if found.id != id {
            return Err(ApiError::new(1401, "The username already exists"));
        }
    }

    let mail = req
        .mail
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_lowercase);
    if let Some(mail_value) = mail.as_deref() {
        validate_register_email(mail_value)?;
        if let Some(found) = load_user_by_mail(&state.db, mail_value).await? {
            if found.id != id {
                return Err(ApiError::new(1401, "The email already exists"));
            }
        }
    }

    let role = req.role.unwrap_or(existing.role).clamp(1, 2);
    let status = req.status.unwrap_or(existing.status);
    let name = req
        .name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(existing.name.as_str())
        .to_string();
    let password = req.password.as_deref().unwrap_or("").trim();

    if existing.role == 1 && role != 1 {
        let admin_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM user WHERE role = 1 AND id != ?")
                .bind(id)
                .fetch_one(&state.db)
                .await
                .map_err(|e| ApiError::db(e.to_string()))?;
        if admin_count == 0 {
            return Err(ApiError::new(1201, "Please keep at least one"));
        }
    }

    if password.is_empty() || password == "-" {
        sqlx::query("UPDATE user SET username = ?, name = ?, head_image = ?, status = ?, role = ?, mail = ?, token = '', updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(username)
            .bind(&name)
            .bind(req.head_image.clone())
            .bind(status)
            .bind(role)
            .bind(mail.clone())
            .bind(id)
            .execute(&state.db)
            .await
            .map_err(|e| ApiError::db(e.to_string()))?;
    } else {
        validate_password_by_policy(&state.db, password).await?;
        let password_hash = hash(password, 12).map_err(|e| ApiError::internal(e.to_string()))?;
        sqlx::query("UPDATE user SET username = ?, password = ?, name = ?, head_image = ?, status = ?, role = ?, mail = ?, token = '', must_change_password = 0, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(username)
            .bind(password_hash)
            .bind(&name)
            .bind(req.head_image.clone())
            .bind(status)
            .bind(role)
            .bind(mail.clone())
            .bind(id)
            .execute(&state.db)
            .await
            .map_err(|e| ApiError::db(e.to_string()))?;
    }

    invalidate_cached_token(&state, existing.token.as_deref()).await;

    Ok(ok(build_user_payload(
        id,
        username.to_string(),
        name,
        req.head_image,
        status,
        role,
        mail,
        existing.referral_code,
        None,
        None,
        if password.is_empty() || password == "-" {
            existing.must_change_password
        } else {
            0
        },
    )))
}

pub async fn panel_users_get_list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<AdminUsersListRequest>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    ensure_admin(&auth)?;

    let page = req.page.unwrap_or(1).max(1);
    let limit = req.limit.unwrap_or(10).clamp(1, 200);
    let keyword = req.keyword.unwrap_or_default().trim().to_string();
    let like = format!("%{}%", keyword);
    let offset = (page - 1) * limit;

    let rows = sqlx::query(
        "SELECT id, username, name, head_image, status, role, mail, referral_code, token, created_at, updated_at, must_change_password FROM user WHERE (? = '' OR name LIKE ? OR username LIKE ?) ORDER BY id ASC LIMIT ? OFFSET ?",
    )
    .bind(&keyword)
    .bind(&like)
    .bind(&like)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::db(e.to_string()))?;

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM user WHERE (? = '' OR name LIKE ? OR username LIKE ?)",
    )
    .bind(&keyword)
    .bind(&like)
    .bind(&like)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::db(e.to_string()))?;

    let list: Vec<Value> = rows.into_iter().map(row_to_user_payload).collect();
    Ok(list_ok(list, count))
}

pub async fn panel_users_deletes(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<AdminUsersDeleteRequest>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    ensure_admin(&auth)?;

    if req.user_ids.is_empty() {
        return Ok(ok_empty());
    }

    let mut removed_tokens = Vec::<String>::new();
    let mut tx = state.db.begin().await.map_err(|e| ApiError::db(e.to_string()))?;
    for user_id in &req.user_ids {
        if let Some(token) = sqlx::query_scalar::<_, Option<String>>(
            "SELECT token FROM user WHERE id = ?",
        )
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?
        .flatten()
        .filter(|value| !value.is_empty())
        {
            removed_tokens.push(token);
        }

        for sql in [
            "DELETE FROM item_icon WHERE user_id = ?",
            "DELETE FROM item_icon_group WHERE user_id = ?",
            "DELETE FROM module_config WHERE user_id = ?",
            "DELETE FROM user_config WHERE user_id = ?",
            "DELETE FROM bookmark WHERE user_id = ?",
            "DELETE FROM notepad WHERE user_id = ?",
            "DELETE FROM search_engine WHERE user_id = ?",
            "DELETE FROM file WHERE user_id = ?",
            "DELETE FROM notice WHERE user_id = ?",
            "DELETE FROM user WHERE id = ?",
        ] {
            sqlx::query(sql)
                .bind(user_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| ApiError::db(e.to_string()))?;
        }
    }

    let admin_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user WHERE role = 1")
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;
    if admin_count == 0 {
        tx.rollback().await.map_err(|e| ApiError::db(e.to_string()))?;
        return Err(ApiError::new(1201, "Please keep at least one"));
    }

    tx.commit().await.map_err(|e| ApiError::db(e.to_string()))?;

    for token in &removed_tokens {
        invalidate_cached_token(&state, Some(token.as_str())).await;
    }

    Ok(ok_empty())
}
