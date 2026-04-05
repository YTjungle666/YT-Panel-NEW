//! System handlers - 系统设置和监控（简化版）

use axum::{
    extract::State,
    http::HeaderMap,
    routing::post,
    Json, Router,
};
use serde::{Deserialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

use crate::{
    AppState, ApiResult, ApiError,
    authenticate, AccessMode, ensure_admin,
    ok, list_ok, ok_empty,
};

// 模块配置
#[derive(Deserialize)]
struct NameRequest {
    name: String,
}

async fn module_config_get(
    State(_state): State<AppState>,
    headers: HeaderMap,
    Json(_req): Json<NameRequest>,
) -> ApiResult {
    let _auth = authenticate(&headers, &_state, AccessMode::PublicAllowed).await?;
    Ok(ok(Value::Null))
}

#[derive(Deserialize)]
struct ModuleConfigSaveRequest {
    name: String,
    value: Value,
}

async fn module_config_save(
    State(_state): State<AppState>,
    headers: HeaderMap,
    Json(_req): Json<ModuleConfigSaveRequest>,
) -> ApiResult {
    let _auth = authenticate(&headers, &_state, AccessMode::LoginRequired).await?;
    Ok(ok_empty())
}

// 系统设置
#[derive(Deserialize)]
struct SystemSettingSetRequest {
    settings: HashMap<String, Value>,
}

async fn system_setting_set(
    State(_state): State<AppState>,
    headers: HeaderMap,
    Json(_req): Json<SystemSettingSetRequest>,
) -> ApiResult {
    let auth = authenticate(&headers, &_state, AccessMode::LoginRequired).await?;
    ensure_admin(&auth)?;
    Ok(ok_empty())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SystemSettingGetRequest {
    config_names: Option<Vec<String>>,
}

async fn system_setting_get(
    State(_state): State<AppState>,
    headers: HeaderMap,
    Json(_req): Json<SystemSettingGetRequest>,
) -> ApiResult {
    let auth = authenticate(&headers, &_state, AccessMode::LoginRequired).await?;
    ensure_admin(&auth)?;
    Ok(ok(json!({})))
}

// 系统监控
async fn system_monitor_get_all(
    State(_state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult {
    let _auth = authenticate(&headers, &_state, AccessMode::LoginRequired).await?;
    Ok(ok(json!({
        "cpu": {"percent": 0},
        "memory": {"total": 0, "used": 0, "available": 0, "percent": 0},
        "disk": {"total": 0, "used": 0, "free": 0, "percent": 0}
    })))
}

async fn system_monitor_get_cpu(
    State(_state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult {
    let _auth = authenticate(&headers, &_state, AccessMode::LoginRequired).await?;
    Ok(ok(json!({"percent": 0})))
}

async fn system_monitor_get_memory(
    State(_state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult {
    let _auth = authenticate(&headers, &_state, AccessMode::LoginRequired).await?;
    Ok(ok(json!({"total": 0, "used": 0, "available": 0, "percent": 0})))
}

#[derive(Deserialize)]
struct DiskPathRequest {
    path: String,
}

async fn system_monitor_get_disk(
    State(_state): State<AppState>,
    headers: HeaderMap,
    Json(_req): Json<DiskPathRequest>,
) -> ApiResult {
    let _auth = authenticate(&headers, &_state, AccessMode::LoginRequired).await?;
    Ok(ok(json!({"total": 0, "used": 0, "free": 0, "percent": 0})))
}

async fn system_monitor_get_mountpoints(
    State(_state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult {
    let _auth = authenticate(&headers, &_state, AccessMode::LoginRequired).await?;
    Ok(ok::<Vec<String>>(vec![]))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/system/moduleConfig/getByName", post(module_config_get))
        .route("/api/system/moduleConfig/save", post(module_config_save))
        .route("/api/system/setting/set", post(system_setting_set))
        .route("/api/system/setting/get", post(system_setting_get))
        .route("/api/system/monitor/getAll", post(system_monitor_get_all))
        .route("/api/system/monitor/getCpuState", post(system_monitor_get_cpu))
        .route("/api/system/monitor/getMemonyState", post(system_monitor_get_memory))
        .route("/api/system/monitor/getDiskStateByPath", post(system_monitor_get_disk))
        .route("/api/system/monitor/getDiskMountpoints", post(system_monitor_get_mountpoints))
}
