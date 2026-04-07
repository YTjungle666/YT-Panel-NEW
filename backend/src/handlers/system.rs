use std::collections::HashMap;

use axum::{extract::State, http::HeaderMap, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::{QueryBuilder, Row, Sqlite};
use sysinfo::{Disks, System};

use crate::{
    auth::{authenticate, ensure_admin},
    db::{get_setting, set_setting},
    error::{list_ok, ok, ok_empty, ApiError, ApiResult},
    models::{default_system_application_value, parse_register_config, AccessMode, AppState},
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoticeRequest {
    display_type: Vec<i64>,
}

#[derive(Deserialize)]
pub struct NameRequest {
    name: String,
}

#[derive(Deserialize)]
pub struct ModuleConfigSaveRequest {
    name: String,
    value: Value,
}

#[derive(Deserialize)]
pub struct SystemSettingSetRequest {
    settings: HashMap<String, Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemSettingGetRequest {
    config_names: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemSettingSingleRequest {
    config_name: String,
}

#[derive(Deserialize)]
pub struct DiskPathRequest {
    path: String,
}

pub async fn notice_get_list(
    State(state): State<AppState>,
    Json(req): Json<NoticeRequest>,
) -> ApiResult {
    if req.display_type.is_empty() {
        return Ok(list_ok(Vec::<Value>::new(), 0));
    }
    let mut builder = QueryBuilder::<Sqlite>::new(
        "SELECT id, title, content, display_type, one_read, url, is_login, user_id, created_at, updated_at \
         FROM notice WHERE display_type IN (",
    );
    {
        let mut separated = builder.separated(", ");
        for item in &req.display_type {
            separated.push_bind(item);
        }
    }
    builder.push(")");
    let rows = builder
        .build()
        .fetch_all(&state.db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;
    let list: Vec<Value> = rows
        .into_iter()
        .map(|row| {
            json!({
                "id": row.get::<i64, _>("id"),
                "title": row.try_get::<Option<String>, _>("title").unwrap_or(None),
                "content": row.try_get::<Option<String>, _>("content").unwrap_or(None),
                "displayType": row.get::<i64, _>("display_type"),
                "oneRead": row.try_get::<Option<i64>, _>("one_read").unwrap_or(Some(0)).unwrap_or(0),
                "url": row.try_get::<Option<String>, _>("url").unwrap_or(None),
                "isLogin": row.try_get::<Option<i64>, _>("is_login").unwrap_or(Some(0)).unwrap_or(0),
                "userId": row.try_get::<Option<i64>, _>("user_id").unwrap_or(None),
                "createTime": row.try_get::<Option<String>, _>("created_at").unwrap_or(None),
                "updateTime": row.try_get::<Option<String>, _>("updated_at").unwrap_or(None),
            })
        })
        .collect();
    let count = list.len() as i64;
    Ok(list_ok(list, count))
}

pub async fn module_config_get(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<NameRequest>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let row = sqlx::query("SELECT value_json FROM module_config WHERE user_id = ? AND name = ? LIMIT 1")
        .bind(auth.user.id)
        .bind(req.name)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;
    let value = row
        .and_then(|r| r.try_get::<Option<String>, _>("value_json").ok().flatten())
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .unwrap_or(Value::Null);
    Ok(ok(value))
}

pub async fn module_config_save(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ModuleConfigSaveRequest>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let value_json = serde_json::to_string(&req.value).unwrap_or_else(|_| "{}".into());
    let existing: Option<i64> =
        sqlx::query_scalar("SELECT id FROM module_config WHERE user_id = ? AND name = ?")
            .bind(auth.user.id)
            .bind(&req.name)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| ApiError::db(e.to_string()))?;
    if let Some(id) = existing {
        sqlx::query("UPDATE module_config SET value_json = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(value_json)
            .bind(id)
            .execute(&state.db)
            .await
            .map_err(|e| ApiError::db(e.to_string()))?;
    } else {
        sqlx::query("INSERT INTO module_config (user_id, name, value_json, created_at, updated_at) VALUES (?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
            .bind(auth.user.id)
            .bind(req.name)
            .bind(value_json)
            .execute(&state.db)
            .await
            .map_err(|e| ApiError::db(e.to_string()))?;
    }
    Ok(ok_empty())
}

pub async fn system_setting_set(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<SystemSettingSetRequest>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    ensure_admin(&auth)?;
    for (k, v) in req.settings {
        let value = if v.is_string() {
            v.as_str().unwrap_or_default().to_string()
        } else {
            serde_json::to_string(&v).unwrap_or_else(|_| "{}".into())
        };
        set_setting(&state.db, &k, &value).await?;
    }
    Ok(ok_empty())
}

pub async fn system_setting_get(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<SystemSettingGetRequest>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    ensure_admin(&auth)?;
    let mut result = serde_json::Map::new();
    if let Some(names) = req.config_names {
        for name in names {
            if let Some(value) = get_setting(&state.db, &name).await? {
                result.insert(name, Value::String(value));
            }
        }
    } else {
        let rows = sqlx::query("SELECT config_name, config_value FROM system_setting")
            .fetch_all(&state.db)
            .await
            .map_err(|e| ApiError::db(e.to_string()))?;
        for row in rows {
            result.insert(
                row.get::<String, _>("config_name"),
                Value::String(row.get::<String, _>("config_value")),
            );
        }
    }
    Ok(ok(Value::Object(result)))
}

pub async fn system_setting_get_single(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<SystemSettingSingleRequest>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    ensure_admin(&auth)?;
    let value = get_setting(&state.db, &req.config_name).await?.unwrap_or_default();
    Ok(ok(json!({ "configName": req.config_name, "configValue": value })))
}

pub async fn system_monitor_get_all(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult {
    let _auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    Ok(ok(build_monitor_payload(None)))
}

pub async fn system_monitor_get_cpu(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult {
    let _auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    Ok(ok(build_cpu_payload()))
}

pub async fn system_monitor_get_memory(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult {
    let _auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    Ok(ok(build_memory_payload()))
}

pub async fn system_monitor_get_disk(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<DiskPathRequest>,
) -> ApiResult {
    let _auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    Ok(ok(build_disk_payload(Some(req.path))))
}

pub async fn system_monitor_get_mountpoints(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult {
    let _auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let disks = Disks::new_with_refreshed_list();
    let list: Vec<Value> = disks
        .list()
        .iter()
        .map(|d| {
            json!({
                "device": d.name().to_string_lossy(),
                "mountpoint": d.mount_point().to_string_lossy(),
                "fstype": d.file_system().to_string_lossy(),
            })
        })
        .collect();
    Ok(ok(list))
}

pub async fn openness_login_config(State(state): State<AppState>) -> ApiResult {
    let raw = get_setting(&state.db, "system_application")
        .await?
        .unwrap_or_else(|| default_system_application_value().to_string());
    let value = serde_json::from_str::<Value>(&raw)
        .unwrap_or_else(|_| default_system_application_value());
    let register = parse_register_config(value.get("register"));
    Ok(ok(json!({
        "loginCaptcha": value.get("loginCaptcha").and_then(|v| v.as_bool()).unwrap_or(false),
        "register": register,
    })))
}

pub async fn openness_get_disclaimer(State(state): State<AppState>) -> ApiResult {
    Ok(ok(get_setting(&state.db, "disclaimer").await?.unwrap_or_default()))
}

pub async fn openness_get_about_description(
    State(state): State<AppState>,
) -> ApiResult {
    Ok(ok(get_setting(&state.db, "web_about_description").await?.unwrap_or_default()))
}

fn build_cpu_payload() -> Value {
    let mut system = System::new_all();
    system.refresh_cpu_all();
    let cpus = system.cpus();
    let usages: Vec<f32> = cpus.iter().map(|cpu| cpu.cpu_usage()).collect();
    json!({
        "coreCount": cpus.len(),
        "cpuNum": cpus.len(),
        "model": cpus.first().map(|c| c.brand().to_string()).unwrap_or_default(),
        "usages": usages,
    })
}

fn build_memory_payload() -> Value {
    let mut system = System::new_all();
    system.refresh_memory();
    let total = system.total_memory();
    let used = system.used_memory();
    let free = total.saturating_sub(used);
    let used_percent = if total == 0 {
        0.0
    } else {
        (used as f64 / total as f64) * 100.0
    };
    json!({
        "total": total,
        "used": used,
        "free": free,
        "usedPercent": used_percent,
    })
}

fn build_disk_payload(path: Option<String>) -> Value {
    let disks = Disks::new_with_refreshed_list();
    let wanted = path.unwrap_or_else(|| "/".into());
    let mut best: Option<Value> = None;
    let mut best_len = 0usize;
    for disk in disks.list() {
        let mount = disk.mount_point().to_string_lossy().to_string();
        if wanted.starts_with(&mount) && mount.len() >= best_len {
            best_len = mount.len();
            let total = disk.total_space();
            let free = disk.available_space();
            let used = total.saturating_sub(free);
            let used_percent = if total == 0 {
                0.0
            } else {
                (used as f64 / total as f64) * 100.0
            };
            best = Some(json!({
                "mountpoint": mount,
                "total": total,
                "used": used,
                "free": free,
                "usedPercent": used_percent,
            }));
        }
    }
    best.unwrap_or_else(|| {
        json!({
            "mountpoint": wanted,
            "total": 0,
            "used": 0,
            "free": 0,
            "usedPercent": 0,
        })
    })
}

fn build_monitor_payload(path: Option<String>) -> Value {
    json!({
        "cpuInfo": build_cpu_payload(),
        "diskInfo": [build_disk_payload(path)],
        "netIOCountersInfo": [],
        "memoryInfo": build_memory_payload(),
    })
}
