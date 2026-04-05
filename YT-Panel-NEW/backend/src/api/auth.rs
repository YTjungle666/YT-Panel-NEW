use crate::{
    auth::{authenticate, invalidate_session_mappings, load_user_by_username, SessionManager},
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
use bcrypt::{hash, verify, DEFAULT_COST};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::query_scalar;

#[derive(Debug, Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LoginResponse {
    token: String,
    name: String,
    head_image: Option<String>,
    role: i64,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/login", post(login))
        .route("/api/logout", post(logout))
}

async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> ApiResult {
    let user = load_user_by_username(&state.db, &req.username)
        .await?
        .ok_or_else(|| ApiError::new(1002, "Invalid username or password"))?;

    if user.status != 1 {
        return Err(ApiError::new(1003, "User is disabled"));
    }

    verify(&req.password, &user.password)
        .map_err(|_| ApiError::new(1002, "Invalid username or password"))?;

    // Generate new session token
    let session_token: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();

    // Store mapping in sessions
    if let Some(ref persistent) = user.token {
        state.sessions.write().await.insert(session_token.clone(), persistent.clone());
    }

    let resp = LoginResponse {
        token: session_token,
        name: user.name,
        head_image: user.head_image,
        role: user.role,
    };

    let mut response = ok(resp);
    let cookie = SessionManager::build_session_cookie(&session_token);
    if let Ok(value) = axum::http::HeaderValue::from_str(&cookie) {
        response.headers_mut().insert(
            axum::http::header::SET_COOKIE,
            value,
        );
    }
    Ok(response)
}

async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::PublicAllowed).await.ok();
    
    if let Some(AuthContext { user, .. }) = auth {
        invalidate_session_mappings(&state, user.token.as_deref()).await;
    }

    let mut response = ok_empty();
    let cookie = SessionManager::build_cleared_session_cookie();
    if let Ok(value) = axum::http::HeaderValue::from_str(&cookie) {
        response.headers_mut().insert(
            axum::http::header::SET_COOKIE,
            value,
        );
    }
    Ok(response)
}
