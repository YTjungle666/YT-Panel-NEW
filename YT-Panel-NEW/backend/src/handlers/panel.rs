//! Panel handlers - 面板核心功能占位符

use axum::{
    extract::State,
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::{
    AppState, ApiResult,
    authenticate, AccessMode,
    ok, list_ok, ok_empty,
};

// 书签列表 - 简化实现
async fn panel_bookmark_get_list(
    State(_state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult {
    let _auth = authenticate(&headers, &_state, AccessMode::PublicAllowed).await?;
    Ok(list_ok::<Vec<Value>>(vec![], 0))
}

// 用户配置
async fn panel_user_config_get(
    State(_state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult {
    let _auth = authenticate(&headers, &_state, AccessMode::LoginRequired).await?;
    Ok(ok(json!({"bookmarkLayout": "grid"})))
}

async fn panel_user_config_set(
    State(_state): State<AppState>,
    headers: HeaderMap,
    Json(_req): Json<Value>,
) -> ApiResult {
    let _auth = authenticate(&headers, &_state, AccessMode::LoginRequired).await?;
    Ok(ok_empty())
}

// 图标组
async fn panel_item_icon_group_get_list(
    State(_state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult {
    let _auth = authenticate(&headers, &_state, AccessMode::PublicAllowed).await?;
    Ok(list_ok::<Vec<Value>>(vec![], 0))
}

// 搜索引擎
async fn panel_search_engine_get_list(
    State(_state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult {
    let _auth = authenticate(&headers, &_state, AccessMode::PublicAllowed).await?;
    Ok(list_ok::<Vec<Value>>(vec![], 0))
}

// 便签
async fn panel_notepad_get(
    State(_state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult {
    let _auth = authenticate(&headers, &_state, AccessMode::LoginRequired).await?;
    Ok(ok(json!({"content": "", "id": 0, "title": ""})))
}

async fn panel_notepad_get_list(
    State(_state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult {
    let _auth = authenticate(&headers, &_state, AccessMode::LoginRequired).await?;
    Ok(list_ok::<Vec<Value>>(vec![], 0))
}

// 路由
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/panel/userConfig/get", post(panel_user_config_get))
        .route("/api/panel/userConfig/set", post(panel_user_config_set))
        .route("/api/panel/bookmark/getList", post(panel_bookmark_get_list))
        .route("/api/panel/itemIconGroup/getList", post(panel_item_icon_group_get_list))
        .route("/api/panel/searchEngine/getList", post(panel_search_engine_get_list))
        .route("/api/panel/notepad/get", get(panel_notepad_get))
        .route("/api/panel/notepad/getList", get(panel_notepad_get_list))
}
