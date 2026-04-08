use std::{
    collections::{HashMap, HashSet},
    net::IpAddr,
    path::Path,
};

use axum::{
    extract::{Multipart, Query, State},
    http::HeaderMap,
    Json,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use mime_guess::MimeGuess;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::{Row, SqlitePool};
use tokio::net::lookup_host;
use url::Url;

use crate::{
    auth::authenticate,
    error::{list_ok, ok, ok_empty, ApiError, ApiResult},
    models::{AccessMode, AppState, BookmarkNode},
    utils::{is_private_ip, parse_i64, parse_opt_string, parse_string, save_upload_field},
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserConfigSetRequest {
    panel: Value,
    search_engine: Option<Value>,
}

#[derive(Deserialize)]
pub struct IdsRequest {
    ids: Vec<i64>,
}

#[derive(Deserialize)]
pub struct FaviconRequest {
    url: String,
}

#[derive(Deserialize)]
pub struct NotepadQuery {
    id: Option<i64>,
}

pub async fn panel_user_config_get(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let row = sqlx::query("SELECT panel_json, search_engine_json FROM user_config WHERE user_id = ?")
        .bind(auth.user.id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;
    let Some(row) = row else {
        return Err(ApiError::new(-1, "未找到数据记录"));
    };

    let panel = row
        .try_get::<Option<String>, _>("panel_json")
        .unwrap_or(None)
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or(Value::Null);
    let search_engine = row
        .try_get::<Option<String>, _>("search_engine_json")
        .unwrap_or(None)
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or(Value::Null);

    Ok(ok(json!({
        "userId": auth.user.id,
        "panel": panel,
        "searchEngine": search_engine,
    })))
}

pub async fn panel_user_config_set(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<UserConfigSetRequest>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let panel_json = serde_json::to_string(&req.panel).unwrap_or_else(|_| "{}".into());
    let search_engine_json =
        serde_json::to_string(&req.search_engine.unwrap_or(Value::Null)).unwrap_or_else(|_| "{}".into());
    let exists: Option<i64> = sqlx::query_scalar("SELECT user_id FROM user_config WHERE user_id = ?")
        .bind(auth.user.id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;

    if exists.is_some() {
        sqlx::query("UPDATE user_config SET panel_json = ?, search_engine_json = ? WHERE user_id = ?")
            .bind(panel_json)
            .bind(search_engine_json)
            .bind(auth.user.id)
            .execute(&state.db)
            .await
            .map_err(|e| ApiError::db(e.to_string()))?;
    } else {
        sqlx::query("INSERT INTO user_config (user_id, panel_json, search_engine_json) VALUES (?, ?, ?)")
            .bind(auth.user.id)
            .bind(panel_json)
            .bind(search_engine_json)
            .execute(&state.db)
            .await
            .map_err(|e| ApiError::db(e.to_string()))?;
    }

    Ok(ok_empty())
}

pub async fn panel_item_icon_group_get_list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let mut rows = sqlx::query(
        "SELECT id, icon, title, description, sort, created_at, updated_at \
         FROM item_icon_group WHERE user_id = ? ORDER BY sort ASC, created_at ASC",
    )
    .bind(auth.user.id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::db(e.to_string()))?;

    if rows.is_empty() {
        sqlx::query(
            "INSERT INTO item_icon_group (icon, title, description, sort, user_id, created_at, updated_at) \
             VALUES (?, ?, '', 0, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind("material-symbols:ad-group-outline")
        .bind("APP")
        .bind(auth.user.id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;

        rows = sqlx::query(
            "SELECT id, icon, title, description, sort, created_at, updated_at \
             FROM item_icon_group WHERE user_id = ? ORDER BY sort ASC, created_at ASC",
        )
        .bind(auth.user.id)
        .fetch_all(&state.db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;
    }

    let list: Vec<Value> = rows
        .into_iter()
        .map(|row| {
            json!({
                "id": row.get::<i64, _>("id"),
                "icon": row.try_get::<Option<String>, _>("icon").unwrap_or(None),
                "title": row.try_get::<Option<String>, _>("title").unwrap_or(None),
                "description": row.try_get::<Option<String>, _>("description").unwrap_or(None),
                "sort": row.try_get::<Option<i64>, _>("sort").unwrap_or(Some(0)).unwrap_or(0),
                "createTime": row.try_get::<Option<String>, _>("created_at").unwrap_or(None),
                "updateTime": row.try_get::<Option<String>, _>("updated_at").unwrap_or(None),
            })
        })
        .collect();
    let count = list.len() as i64;
    Ok(list_ok(list, count))
}

pub async fn panel_item_icon_group_edit(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<Value>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let id = parse_i64(req.get("id"));
    let icon = parse_opt_string(req.get("icon"));
    let title = parse_string(req.get("title"));
    let description = parse_string(req.get("description"));
    let sort = parse_i64(req.get("sort"));

    if id > 0 {
        sqlx::query(
            "UPDATE item_icon_group SET icon = ?, title = ?, description = ?, sort = ?, \
             updated_at = CURRENT_TIMESTAMP WHERE id = ? AND user_id = ?",
        )
        .bind(icon.clone())
        .bind(title.clone())
        .bind(description.clone())
        .bind(sort)
        .bind(id)
        .bind(auth.user.id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;

        Ok(ok(json!({
            "id": id,
            "icon": icon,
            "title": title,
            "description": description,
            "sort": sort,
            "userId": auth.user.id,
        })))
    } else {
        let res = sqlx::query(
            "INSERT INTO item_icon_group (icon, title, description, sort, user_id, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind(icon.clone())
        .bind(title.clone())
        .bind(description.clone())
        .bind(sort)
        .bind(auth.user.id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;

        Ok(ok(json!({
            "id": res.last_insert_rowid(),
            "icon": icon,
            "title": title,
            "description": description,
            "sort": sort,
            "userId": auth.user.id,
        })))
    }
}

pub async fn panel_item_icon_group_deletes(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<IdsRequest>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM item_icon_group WHERE user_id = ?")
        .bind(auth.user.id)
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);
    if req.ids.len() as i64 >= count {
        return Err(ApiError::new(1201, "请至少保留一个"));
    }

    let mut tx = state.db.begin().await.map_err(|e| ApiError::db(e.to_string()))?;
    for id in req.ids {
        sqlx::query("DELETE FROM item_icon WHERE item_icon_group_id = ? AND user_id = ?")
            .bind(id)
            .bind(auth.user.id)
            .execute(&mut *tx)
            .await
            .map_err(|e| ApiError::db(e.to_string()))?;
        sqlx::query("DELETE FROM item_icon_group WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(auth.user.id)
            .execute(&mut *tx)
            .await
            .map_err(|e| ApiError::db(e.to_string()))?;
    }
    tx.commit().await.map_err(|e| ApiError::db(e.to_string()))?;
    Ok(ok_empty())
}

pub async fn panel_item_icon_group_save_sort(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<Value>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let sort_items = req
        .get("sortItems")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();

    let mut tx = state.db.begin().await.map_err(|e| ApiError::db(e.to_string()))?;
    for item in sort_items {
        let id = parse_i64(item.get("id"));
        let sort = parse_i64(item.get("sort"));
        sqlx::query(
            "UPDATE item_icon_group SET sort = ?, updated_at = CURRENT_TIMESTAMP \
             WHERE id = ? AND user_id = ?",
        )
        .bind(sort)
        .bind(id)
        .bind(auth.user.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;
    }
    tx.commit().await.map_err(|e| ApiError::db(e.to_string()))?;
    Ok(ok_empty())
}

pub async fn panel_item_icon_get_list_by_group_id(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<Value>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let group_id = parse_i64(req.get("itemIconGroupId"));
    let rows = sqlx::query(
        "SELECT id, created_at, updated_at, icon_json, title, url, lan_url, description, \
         open_method, sort, item_icon_group_id, lan_only \
         FROM item_icon WHERE item_icon_group_id = ? AND user_id = ? ORDER BY sort ASC, created_at ASC",
    )
    .bind(group_id)
    .bind(auth.user.id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::db(e.to_string()))?;

    let list: Vec<Value> = rows
        .into_iter()
        .map(|row| {
            let icon_json = row
                .try_get::<Option<String>, _>("icon_json")
                .unwrap_or(None)
                .unwrap_or_else(|| "{}".into());
            let icon = serde_json::from_str::<Value>(&icon_json).unwrap_or(Value::Null);
            json!({
                "id": row.get::<i64, _>("id"),
                "createTime": row.try_get::<Option<String>, _>("created_at").unwrap_or(None),
                "updateTime": row.try_get::<Option<String>, _>("updated_at").unwrap_or(None),
                "icon": icon,
                "title": row.try_get::<Option<String>, _>("title").unwrap_or(None),
                "url": row.try_get::<Option<String>, _>("url").unwrap_or(None),
                "lanUrl": row.try_get::<Option<String>, _>("lan_url").unwrap_or(None),
                "description": row.try_get::<Option<String>, _>("description").unwrap_or(None),
                "openMethod": row.try_get::<Option<i64>, _>("open_method").unwrap_or(Some(1)).unwrap_or(1),
                "sort": row.try_get::<Option<i64>, _>("sort").unwrap_or(Some(0)).unwrap_or(0),
                "itemIconGroupId": row.try_get::<Option<i64>, _>("item_icon_group_id").unwrap_or(Some(0)).unwrap_or(0),
                "lanOnly": row.try_get::<Option<i64>, _>("lan_only").unwrap_or(Some(0)).unwrap_or(0),
            })
        })
        .collect();
    let count = list.len() as i64;
    Ok(list_ok(list, count))
}

pub async fn panel_item_icon_edit(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<Value>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let id = parse_i64(req.get("id"));
    let title = parse_string(req.get("title"));
    let url = parse_string(req.get("url"));
    let lan_url = parse_opt_string(req.get("lanUrl"));
    let description = parse_string(req.get("description"));
    let open_method = parse_i64(req.get("openMethod"));
    let sort = parse_i64(req.get("sort"));
    let group_id = parse_i64(req.get("itemIconGroupId"));
    let lan_only = parse_i64(req.get("lanOnly"));
    if group_id == 0 {
        return Err(ApiError::bad_param("Group is mandatory"));
    }
    let icon_json =
        serde_json::to_string(req.get("icon").unwrap_or(&Value::Null)).unwrap_or_else(|_| "{}".into());

    if id > 0 {
        sqlx::query(
            "UPDATE item_icon SET icon_json = ?, title = ?, url = ?, lan_url = ?, description = ?, \
             open_method = ?, sort = ?, item_icon_group_id = ?, lan_only = ?, updated_at = CURRENT_TIMESTAMP \
             WHERE id = ? AND user_id = ?",
        )
        .bind(icon_json.clone())
        .bind(title.clone())
        .bind(url.clone())
        .bind(lan_url.clone())
        .bind(description.clone())
        .bind(open_method)
        .bind(sort)
        .bind(group_id)
        .bind(lan_only)
        .bind(id)
        .bind(auth.user.id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;

        Ok(ok(json!({
            "id": id,
            "title": title,
            "url": url,
            "lanUrl": lan_url,
            "description": description,
            "openMethod": open_method,
            "sort": sort,
            "itemIconGroupId": group_id,
            "lanOnly": lan_only,
            "icon": req.get("icon").cloned().unwrap_or(Value::Null),
        })))
    } else {
        let res = sqlx::query(
            "INSERT INTO item_icon (icon_json, title, url, lan_url, description, open_method, sort, \
             item_icon_group_id, lan_only, user_id, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, 9999, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind(icon_json)
        .bind(title.clone())
        .bind(url.clone())
        .bind(lan_url.clone())
        .bind(description.clone())
        .bind(open_method)
        .bind(group_id)
        .bind(lan_only)
        .bind(auth.user.id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;

        Ok(ok(json!({
            "id": res.last_insert_rowid(),
            "title": title,
            "url": url,
            "lanUrl": lan_url,
            "description": description,
            "openMethod": open_method,
            "sort": 9999,
            "itemIconGroupId": group_id,
            "lanOnly": lan_only,
            "icon": req.get("icon").cloned().unwrap_or(Value::Null),
        })))
    }
}

pub async fn panel_item_icon_add_multiple(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<Value>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let items = req.as_array().cloned().unwrap_or_default();
    let mut created = Vec::<Value>::new();
    let mut tx = state.db.begin().await.map_err(|e| ApiError::db(e.to_string()))?;

    for item in items {
        let title = parse_string(item.get("title"));
        let url = parse_string(item.get("url"));
        let lan_url = parse_opt_string(item.get("lanUrl"));
        let description = parse_string(item.get("description"));
        let open_method = parse_i64(item.get("openMethod"));
        let parsed_sort = parse_i64(item.get("sort"));
        let sort = if parsed_sort > 0 { parsed_sort } else { 9999 };
        let group_id = parse_i64(item.get("itemIconGroupId"));
        let lan_only = parse_i64(item.get("lanOnly"));
        if group_id == 0 {
            continue;
        }

        let icon_json =
            serde_json::to_string(item.get("icon").unwrap_or(&Value::Null)).unwrap_or_else(|_| "{}".into());
        let res = sqlx::query(
            "INSERT INTO item_icon (icon_json, title, url, lan_url, description, open_method, sort, \
             item_icon_group_id, lan_only, user_id, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind(icon_json)
        .bind(title.clone())
        .bind(url.clone())
        .bind(lan_url.clone())
        .bind(description.clone())
        .bind(open_method)
        .bind(sort)
        .bind(group_id)
        .bind(lan_only)
        .bind(auth.user.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;

        created.push(json!({
            "id": res.last_insert_rowid(),
            "title": title,
            "url": url,
            "lanUrl": lan_url,
            "description": description,
            "openMethod": open_method,
            "sort": sort,
            "itemIconGroupId": group_id,
            "lanOnly": lan_only,
            "icon": item.get("icon").cloned().unwrap_or(Value::Null),
        }));
    }

    tx.commit().await.map_err(|e| ApiError::db(e.to_string()))?;
    Ok(ok(created))
}

pub async fn panel_item_icon_deletes(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<IdsRequest>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let mut tx = state.db.begin().await.map_err(|e| ApiError::db(e.to_string()))?;
    for id in req.ids {
        sqlx::query("DELETE FROM item_icon WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(auth.user.id)
            .execute(&mut *tx)
            .await
            .map_err(|e| ApiError::db(e.to_string()))?;
    }
    tx.commit().await.map_err(|e| ApiError::db(e.to_string()))?;
    Ok(ok_empty())
}

pub async fn panel_item_icon_save_sort(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<Value>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let group_id = parse_i64(req.get("itemIconGroupId"));
    let items = req
        .get("sortItems")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();

    let mut tx = state.db.begin().await.map_err(|e| ApiError::db(e.to_string()))?;
    for item in items {
        let id = parse_i64(item.get("id"));
        let sort = parse_i64(item.get("sort"));
        sqlx::query(
            "UPDATE item_icon SET sort = ?, updated_at = CURRENT_TIMESTAMP \
             WHERE id = ? AND item_icon_group_id = ? AND user_id = ?",
        )
        .bind(sort)
        .bind(id)
        .bind(group_id)
        .bind(auth.user.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;
    }
    tx.commit().await.map_err(|e| ApiError::db(e.to_string()))?;
    Ok(ok_empty())
}

fn build_favicon_client() -> Result<reqwest::Client, ApiError> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(std::time::Duration::from_secs(3))
        .timeout(std::time::Duration::from_secs(6))
        .build()
        .map_err(|e| ApiError::new(-1, e.to_string()))
}

const MAX_FAVICON_HTML_BYTES: usize = 512 * 1024;
const MAX_FAVICON_MANIFEST_BYTES: usize = 256 * 1024;
const MAX_FAVICON_IMAGE_BYTES: usize = 512 * 1024;

fn is_blocked_outbound_hostname(host: &str) -> bool {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    matches!(
        host.as_str(),
        "localhost"
            | "metadata"
            | "metadata.google.internal"
            | "metadata.aliyun.internal"
            | "instance-data"
    ) || host.ends_with(".localhost")
        || host.ends_with(".local")
}

fn is_unsafe_outbound_ip(ip: IpAddr) -> bool {
    is_private_ip(ip) || ip.is_multicast() || ip.is_unspecified()
}

async fn ensure_safe_outbound_url(url: &Url) -> Result<(), ApiError> {
    if !matches!(url.scheme(), "http" | "https") {
        return Err(ApiError::new(-1, "无效或不安全的 URL"));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(ApiError::new(-1, "无效或不安全的 URL"));
    }

    let host = url
        .host_str()
        .ok_or_else(|| ApiError::new(-1, "无效或不安全的 URL"))?;

    if is_blocked_outbound_hostname(host) {
        return Err(ApiError::new(-1, "无效或不安全的 URL"));
    }

    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_unsafe_outbound_ip(ip) {
            return Err(ApiError::new(-1, "无效或不安全的 URL"));
        }
        return Ok(());
    }

    let port = url.port_or_known_default().unwrap_or(80);
    let mut resolved_any = false;
    let addrs = lookup_host((host, port))
        .await
        .map_err(|_| ApiError::new(-1, "无效或不安全的 URL"))?;
    for addr in addrs {
        resolved_any = true;
        if is_unsafe_outbound_ip(addr.ip()) {
            return Err(ApiError::new(-1, "无效或不安全的 URL"));
        }
    }

    if !resolved_any {
        return Err(ApiError::new(-1, "无效或不安全的 URL"));
    }

    Ok(())
}

static ATTR_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"([A-Za-z_:][-A-Za-z0-9_:.]*)\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'=<>`]+))"#)
        .expect("attribute regex must compile")
});

static LINK_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"(?is)<link\b[^>]*>"#).expect("link regex must compile"));

fn normalize_attr_value(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string()
}

fn parse_html_attributes(tag: &str) -> HashMap<String, String> {
    let mut attrs = HashMap::new();

    for cap in ATTR_REGEX.captures_iter(tag) {
        let key = cap
            .get(1)
            .map(|m| m.as_str().to_ascii_lowercase())
            .unwrap_or_default();
        let value = cap
            .get(2)
            .or_else(|| cap.get(3))
            .or_else(|| cap.get(4))
            .map(|m| normalize_attr_value(m.as_str()))
            .unwrap_or_default();

        if !key.is_empty() {
            attrs.insert(key, value);
        }
    }

    attrs
}

fn score_icon_candidate(rel: &str, sizes: &str, icon_type: &str) -> i32 {
    let rel_lower = rel.to_ascii_lowercase();
    let type_lower = icon_type.to_ascii_lowercase();
    let mut score = 0;

    if rel_lower.contains("apple-touch-icon") {
        score += 120;
    } else if rel_lower.contains("shortcut icon") {
        score += 100;
    } else if rel_lower.contains("icon") {
        score += 90;
    }

    if type_lower.contains("svg") {
        score += 35;
    } else if type_lower.contains("png") {
        score += 25;
    } else if type_lower.contains("ico") {
        score += 20;
    }

    for token in sizes.split_whitespace() {
        if let Some((w, h)) = token.split_once('x') {
            if let (Ok(w), Ok(h)) = (w.parse::<i32>(), h.parse::<i32>()) {
                score += w.min(h).min(256);
                break;
            }
        }
    }

    score
}

fn extract_icon_candidates_from_html(base_url: &Url, html: &str) -> (Vec<Url>, Vec<Url>) {
    let mut icon_candidates: Vec<(i32, Url)> = Vec::new();
    let mut manifest_candidates: Vec<Url> = Vec::new();

    for link_tag in LINK_REGEX.find_iter(html) {
        let attrs = parse_html_attributes(link_tag.as_str());
        let rel = attrs.get("rel").cloned().unwrap_or_default();
        let href = attrs.get("href").cloned().unwrap_or_default();
        if href.is_empty() {
            continue;
        }

        let rel_lower = rel.to_ascii_lowercase();
        let joined = match base_url.join(&href) {
            Ok(url) => url,
            Err(_) => continue,
        };

        if rel_lower.contains("manifest") {
            manifest_candidates.push(joined);
            continue;
        }

        if rel_lower.contains("icon") {
            let score = score_icon_candidate(
                &rel,
                attrs.get("sizes").map(|value| value.as_str()).unwrap_or(""),
                attrs.get("type").map(|value| value.as_str()).unwrap_or(""),
            );
            icon_candidates.push((score, joined));
        }
    }

    icon_candidates.sort_by(|a, b| b.0.cmp(&a.0));
    (
        icon_candidates.into_iter().map(|(_, url)| url).collect(),
        manifest_candidates,
    )
}

fn extract_manifest_icon_candidates(manifest_url: &Url, manifest_text: &str) -> Vec<Url> {
    let mut candidates: Vec<(i32, Url)> = Vec::new();
    let value = serde_json::from_str::<Value>(manifest_text).unwrap_or(Value::Null);

    if let Some(icons) = value.get("icons").and_then(|value| value.as_array()) {
        for icon in icons {
            let Some(src) = icon.get("src").and_then(|value| value.as_str()) else {
                continue;
            };
            let Ok(url) = manifest_url.join(src) else {
                continue;
            };
            let score = score_icon_candidate(
                "icon",
                icon.get("sizes").and_then(|value| value.as_str()).unwrap_or(""),
                icon.get("type").and_then(|value| value.as_str()).unwrap_or(""),
            );
            candidates.push((score, url));
        }
    }

    candidates.sort_by(|a, b| b.0.cmp(&a.0));
    candidates.into_iter().map(|(_, url)| url).collect()
}

fn default_bookmark_icon_data_url() -> String {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64" fill="none"><defs><linearGradient id="bookmarkBg" x1="8" y1="8" x2="56" y2="56" gradientUnits="userSpaceOnUse"><stop stop-color="#4F8CFF"/><stop offset="1" stop-color="#2563EB"/></linearGradient></defs><rect x="8" y="8" width="48" height="48" rx="14" fill="url(#bookmarkBg)"/><path d="M24 18C24 16.8954 24.8954 16 26 16H38C39.1046 16 40 16.8954 40 18V46L32 39.5L24 46V18Z" fill="white" fill-opacity="0.96"/></svg>"##;
    format!("data:image/svg+xml;base64,{}", B64.encode(svg.as_bytes()))
}

fn favicon_cache_key(url: &str) -> String {
    format!("{:x}", md5::compute(url.trim().to_ascii_lowercase()))
}

async fn favicon_cache_get(db: &SqlitePool, url: &str) -> Result<Option<String>, ApiError> {
    let cache_key = favicon_cache_key(url);
    sqlx::query_scalar::<_, String>("SELECT icon_data_url FROM favicon_cache WHERE cache_key = ?")
        .bind(cache_key)
        .fetch_optional(db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))
}

async fn favicon_cache_put(db: &SqlitePool, url: &str, icon_data_url: &str) -> Result<(), ApiError> {
    let cache_key = favicon_cache_key(url);
    sqlx::query(
        "INSERT INTO favicon_cache (cache_key, source_url, icon_data_url, created_at, updated_at) \
         VALUES (?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) \
         ON CONFLICT(cache_key) DO UPDATE SET source_url = excluded.source_url, \
         icon_data_url = excluded.icon_data_url, updated_at = CURRENT_TIMESTAMP",
    )
    .bind(cache_key)
    .bind(url.trim())
    .bind(icon_data_url)
    .execute(db)
    .await
    .map_err(|e| ApiError::db(e.to_string()))?;
    Ok(())
}

async fn resolve_site_favicon_with_cache(state: &AppState, url: &str) -> Result<String, ApiError> {
    if let Some(cached) = favicon_cache_get(&state.db, url).await? {
        return Ok(cached);
    }

    let icon_url = resolve_site_favicon_data_url(url).await?;
    favicon_cache_put(&state.db, url, &icon_url).await?;
    Ok(icon_url)
}

async fn fetch_html_document(client: &reqwest::Client, url: &Url) -> Result<Option<String>, ApiError> {
    if ensure_safe_outbound_url(url).await.is_err() {
        return Ok(None);
    }

    let resp = match client
        .get(url.clone())
        .header(reqwest::header::USER_AGENT, "YT-Panel/1.0")
        .header(reqwest::header::ACCEPT, "text/html,application/xhtml+xml")
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(_) => return Ok(None),
    };

    if !resp.status().is_success() {
        return Ok(None);
    }

    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !content_type.contains("text/html") && !content_type.contains("application/xhtml") {
        return Ok(None);
    }

    let Some(text) = read_response_text_limited(resp, MAX_FAVICON_HTML_BYTES).await? else {
        return Ok(None);
    };
    if text.trim().is_empty() {
        return Ok(None);
    }

    Ok(Some(text))
}

async fn fetch_manifest_document(
    client: &reqwest::Client,
    url: &Url,
) -> Result<Option<String>, ApiError> {
    if ensure_safe_outbound_url(url).await.is_err() {
        return Ok(None);
    }

    let resp = match client
        .get(url.clone())
        .header(reqwest::header::USER_AGENT, "YT-Panel/1.0")
        .header(
            reqwest::header::ACCEPT,
            "application/manifest+json,application/json,text/plain",
        )
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(_) => return Ok(None),
    };

    if !resp.status().is_success() {
        return Ok(None);
    }

    let Some(text) = read_response_text_limited(resp, MAX_FAVICON_MANIFEST_BYTES).await? else {
        return Ok(None);
    };
    if text.trim().is_empty() {
        return Ok(None);
    }

    Ok(Some(text))
}

async fn fetch_favicon_data_url(
    client: &reqwest::Client,
    url: &Url,
) -> Result<Option<String>, ApiError> {
    if ensure_safe_outbound_url(url).await.is_err() {
        return Ok(None);
    }

    let resp = match client
        .get(url.clone())
        .header(reqwest::header::USER_AGENT, "YT-Panel/1.0")
        .header(reqwest::header::ACCEPT, "image/*,*/*;q=0.8")
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(_) => return Ok(None),
    };

    if !resp.status().is_success() {
        return Ok(None);
    }

    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();

    let guessed = MimeGuess::from_path(url.path())
        .first_raw()
        .unwrap_or("image/x-icon")
        .to_string();

    let mime = if content_type.starts_with("image/") {
        content_type
    } else if guessed.starts_with("image/") {
        guessed
    } else {
        return Ok(None);
    };

    if mime.eq_ignore_ascii_case("image/svg+xml") {
        return Ok(None);
    }

    let Some(bytes) = read_response_bytes_limited(resp, MAX_FAVICON_IMAGE_BYTES).await? else {
        return Ok(None);
    };
    if bytes.is_empty() {
        return Ok(None);
    }

    Ok(Some(format!("data:{};base64,{}", mime, B64.encode(bytes))))
}

async fn read_response_bytes_limited(
    mut resp: reqwest::Response,
    max_bytes: usize,
) -> Result<Option<Vec<u8>>, ApiError> {
    if resp
        .content_length()
        .map(|length| length > max_bytes as u64)
        .unwrap_or(false)
    {
        return Ok(None);
    }

    let mut buffer = Vec::<u8>::new();
    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| ApiError::new(-1, e.to_string()))?
    {
        if buffer.len() + chunk.len() > max_bytes {
            return Ok(None);
        }
        buffer.extend_from_slice(&chunk);
    }

    Ok(Some(buffer))
}

async fn read_response_text_limited(
    resp: reqwest::Response,
    max_bytes: usize,
) -> Result<Option<String>, ApiError> {
    let Some(bytes) = read_response_bytes_limited(resp, max_bytes).await? else {
        return Ok(None);
    };
    Ok(Some(String::from_utf8_lossy(&bytes).into_owned()))
}

async fn resolve_site_favicon_data_url(url: &str) -> Result<String, ApiError> {
    let parsed = Url::parse(url).map_err(|_| ApiError::new(-1, "无效或不安全的 URL"))?;
    ensure_safe_outbound_url(&parsed).await?;

    let host = parsed
        .host_str()
        .ok_or_else(|| ApiError::new(-1, "无效或不安全的 URL"))?;
    let origin = parsed.origin().ascii_serialization();
    let is_proxmox = host.to_ascii_lowercase().contains("pve")
        || parsed.port_or_known_default() == Some(8006)
        || url.to_ascii_lowercase().contains("proxmox");

    let client = build_favicon_client()?;
    let mut candidates: Vec<Url> = Vec::new();
    let mut seen = HashSet::new();

    if let Some(html) = fetch_html_document(&client, &parsed).await? {
        let (html_icons, manifest_urls) = extract_icon_candidates_from_html(&parsed, &html);
        for icon_url in html_icons {
            if ensure_safe_outbound_url(&icon_url).await.is_ok() && seen.insert(icon_url.to_string()) {
                candidates.push(icon_url);
            }
        }

        for manifest_url in manifest_urls {
            if let Some(manifest_text) = fetch_manifest_document(&client, &manifest_url).await? {
                for icon_url in extract_manifest_icon_candidates(&manifest_url, &manifest_text) {
                    if ensure_safe_outbound_url(&icon_url).await.is_ok() && seen.insert(icon_url.to_string()) {
                        candidates.push(icon_url);
                    }
                }
            }
        }
    }

    let fallback_candidates = [
        format!("{}/favicon.ico", origin.trim_end_matches('/')),
        format!("{}/favicon.png", origin.trim_end_matches('/')),
        format!("{}/apple-touch-icon.png", origin.trim_end_matches('/')),
    ];
    for candidate in fallback_candidates {
        if let Ok(url) = Url::parse(&candidate) {
            if ensure_safe_outbound_url(&url).await.is_ok() && seen.insert(url.to_string()) {
                candidates.push(url);
            }
        }
    }

    if is_proxmox {
        for candidate in [
            format!("{}/pve2/images/logo-128.png", origin.trim_end_matches('/')),
            format!("{}/images/logo-128.png", origin.trim_end_matches('/')),
        ] {
            if let Ok(url) = Url::parse(&candidate) {
                if ensure_safe_outbound_url(&url).await.is_ok() && seen.insert(url.to_string()) {
                    candidates.push(url);
                }
            }
        }
    }

    if let Ok(google_s2) = Url::parse(&format!(
        "https://www.google.com/s2/favicons?domain={}&sz=64",
        host
    )) {
        if ensure_safe_outbound_url(&google_s2).await.is_ok() && seen.insert(google_s2.to_string()) {
            candidates.push(google_s2);
        }
    }

    for candidate in candidates {
        if let Some(icon_url) = fetch_favicon_data_url(&client, &candidate).await? {
            return Ok(icon_url);
        }
    }

    Ok(default_bookmark_icon_data_url())
}

pub async fn panel_item_icon_get_site_favicon(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<FaviconRequest>,
) -> ApiResult {
    let _auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let icon_url = resolve_site_favicon_with_cache(&state, &req.url).await?;
    Ok(ok(json!({ "iconUrl": icon_url })))
}

pub async fn panel_bookmark_get_list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let rows = sqlx::query(
        "SELECT id, created_at, icon_json, title, url, lan_url, sort, is_folder, parent_url, parent_id \
         FROM bookmark WHERE user_id = ? ORDER BY sort ASC, created_at ASC",
    )
    .bind(auth.user.id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::db(e.to_string()))?;

    let mut grouped: HashMap<i64, Vec<BookmarkNode>> = HashMap::new();
    for row in rows {
        let node = BookmarkNode {
            id: row.get::<i64, _>("id"),
            create_time: row.try_get::<Option<String>, _>("created_at").unwrap_or(None),
            icon_json: row.try_get::<Option<String>, _>("icon_json").unwrap_or(None),
            title: row
                .try_get::<Option<String>, _>("title")
                .unwrap_or(None)
                .unwrap_or_default(),
            url: row
                .try_get::<Option<String>, _>("url")
                .unwrap_or(None)
                .unwrap_or_default(),
            lan_url: row.try_get::<Option<String>, _>("lan_url").unwrap_or(None),
            sort: row.try_get::<Option<i64>, _>("sort").unwrap_or(Some(0)).unwrap_or(0),
            is_folder: row
                .try_get::<Option<i64>, _>("is_folder")
                .unwrap_or(Some(0))
                .unwrap_or(0),
            parent_url: row.try_get::<Option<String>, _>("parent_url").unwrap_or(None),
            parent_id: row
                .try_get::<Option<i64>, _>("parent_id")
                .unwrap_or(Some(0))
                .unwrap_or(0),
            children: Vec::new(),
        };
        grouped.entry(node.parent_id).or_default().push(node);
    }

    fn build_tree(parent_id: i64, grouped: &HashMap<i64, Vec<BookmarkNode>>) -> Vec<BookmarkNode> {
        let mut nodes = grouped.get(&parent_id).cloned().unwrap_or_default();
        nodes.sort_by_key(|node| (node.sort, node.title.clone()));
        for node in &mut nodes {
            node.children = build_tree(node.id, grouped);
        }
        nodes
    }

    let tree = build_tree(0, &grouped);
    let count = tree.len() as i64;
    Ok(list_ok(tree, count))
}

pub async fn panel_bookmark_add(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<Value>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let title = parse_string(req.get("title"));
    let url = parse_string(req.get("url"));
    let lan_url = parse_opt_string(req.get("lanUrl"));
    let parent_url = parse_opt_string(req.get("parentUrl"));
    let parent_id = parse_i64(req.get("parentId"));
    let is_folder = parse_i64(req.get("isFolder"));
    let icon_json = req
        .get("iconJson")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
        .unwrap_or_default();

    let max_sort: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(sort), 0) FROM bookmark WHERE user_id = ? AND parent_id = ?",
    )
    .bind(auth.user.id)
    .bind(parent_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let res = sqlx::query(
        "INSERT INTO bookmark (title, url, lan_url, sort, is_folder, parent_url, parent_id, icon_json, user_id, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)",
    )
    .bind(title.clone())
    .bind(url.clone())
    .bind(lan_url.clone())
    .bind(max_sort + 1)
    .bind(is_folder)
    .bind(parent_url.clone())
    .bind(parent_id)
    .bind(icon_json.clone())
    .bind(auth.user.id)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::db(e.to_string()))?;

    Ok(ok(json!({
        "id": res.last_insert_rowid(),
        "title": title,
        "url": url,
        "lanUrl": lan_url,
        "sort": max_sort + 1,
        "isFolder": is_folder,
        "parentUrl": parent_url,
        "parentId": parent_id,
        "iconJson": icon_json,
    })))
}

pub async fn panel_bookmark_add_multiple(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<Value>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let items = if let Some(array) = req.as_array() {
        array.clone()
    } else if let Some(array) = req.get("Bookmarks").and_then(|value| value.as_array()) {
        array.clone()
    } else if let Some(array) = req.get("bookmarks").and_then(|value| value.as_array()) {
        array.clone()
    } else {
        Vec::new()
    };

    let mut inserted = Vec::new();
    let mut temp_id_map = HashMap::new();
    let mut tx = state.db.begin().await.map_err(|e| ApiError::db(e.to_string()))?;

    for item in items {
        let title = parse_string(item.get("title"));
        let url = parse_string(item.get("url"));
        let lan_url = parse_opt_string(item.get("lanUrl"));
        let is_folder = parse_i64(item.get("isFolder"));
        let parent_url = parse_opt_string(item.get("parentUrl"));
        let parent_temp_id = parse_i64(item.get("parentTempId"));
        let mut parent_id = parse_i64(item.get("parentId").or_else(|| item.get("folderId")));
        if parent_temp_id > 0 {
            if let Some(mapped_parent_id) = temp_id_map.get(&parent_temp_id) {
                parent_id = *mapped_parent_id;
            }
        }

        let parsed_sort = parse_i64(item.get("sort"));
        let sort = if parsed_sort > 0 { parsed_sort } else { 9999 };
        let icon_json = if let Some(icon) = item.get("iconJson") {
            icon.as_str().unwrap_or_default().to_string()
        } else if let Some(icon) = item.get("icon") {
            if icon.is_string() {
                icon.as_str().unwrap_or_default().to_string()
            } else {
                serde_json::to_string(icon).unwrap_or_default()
            }
        } else {
            String::new()
        };

        let res = sqlx::query(
            "INSERT INTO bookmark (title, url, lan_url, sort, is_folder, parent_url, parent_id, icon_json, user_id, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)",
        )
        .bind(title.clone())
        .bind(url.clone())
        .bind(lan_url.clone())
        .bind(sort)
        .bind(is_folder)
        .bind(parent_url.clone())
        .bind(parent_id)
        .bind(icon_json.clone())
        .bind(auth.user.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;

        let new_id = res.last_insert_rowid();
        let temp_id = parse_i64(item.get("tempId"));
        if temp_id > 0 {
            temp_id_map.insert(temp_id, new_id);
        }

        inserted.push(json!({
            "id": new_id,
            "title": title,
            "url": url,
            "lanUrl": lan_url,
            "sort": sort,
            "isFolder": is_folder,
            "parentUrl": parent_url,
            "parentId": parent_id,
            "iconJson": icon_json,
        }));
    }

    tx.commit().await.map_err(|e| ApiError::db(e.to_string()))?;
    Ok(ok(json!({ "count": inserted.len(), "list": inserted })))
}

pub async fn panel_bookmark_update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<Value>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let id = parse_i64(req.get("id"));
    let title = parse_string(req.get("title"));
    let url = parse_string(req.get("url"));
    let lan_url = parse_opt_string(req.get("lanUrl"));
    let parent_url = parse_opt_string(req.get("parentUrl"));
    let parent_id = parse_i64(req.get("parentId"));
    let sort = parse_i64(req.get("sort"));
    let icon_json = req
        .get("iconJson")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
        .unwrap_or_default();

    sqlx::query(
        "UPDATE bookmark SET title = ?, url = ?, lan_url = ?, parent_url = ?, parent_id = ?, sort = ?, icon_json = ? \
         WHERE id = ? AND user_id = ?",
    )
    .bind(title.clone())
    .bind(url.clone())
    .bind(lan_url.clone())
    .bind(parent_url.clone())
    .bind(parent_id)
    .bind(sort)
    .bind(icon_json.clone())
    .bind(id)
    .bind(auth.user.id)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::db(e.to_string()))?;

    Ok(ok(json!({
        "id": id,
        "title": title,
        "url": url,
        "lanUrl": lan_url,
        "parentUrl": parent_url,
        "parentId": parent_id,
        "sort": sort,
        "iconJson": icon_json,
    })))
}

pub async fn panel_bookmark_deletes(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<IdsRequest>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let mut to_delete = req.ids.clone();
    let mut tx = state.db.begin().await.map_err(|e| ApiError::db(e.to_string()))?;
    let mut idx = 0usize;

    while idx < to_delete.len() {
        let current = to_delete[idx];
        let child_rows = sqlx::query("SELECT id FROM bookmark WHERE user_id = ? AND parent_id = ?")
            .bind(auth.user.id)
            .bind(current)
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| ApiError::db(e.to_string()))?;
        for row in child_rows {
            let child_id = row.get::<i64, _>("id");
            if !to_delete.contains(&child_id) {
                to_delete.push(child_id);
            }
        }
        idx += 1;
    }

    for id in to_delete {
        sqlx::query("DELETE FROM bookmark WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(auth.user.id)
            .execute(&mut *tx)
            .await
            .map_err(|e| ApiError::db(e.to_string()))?;
    }
    tx.commit().await.map_err(|e| ApiError::db(e.to_string()))?;
    Ok(ok_empty())
}

pub async fn panel_notepad_get(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<NotepadQuery>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let row = if let Some(id) = query.id {
        sqlx::query(
            "SELECT id, user_id, title, content, created_at, updated_at \
             FROM notepad WHERE user_id = ? AND id = ? LIMIT 1",
        )
        .bind(auth.user.id)
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?
    } else {
        sqlx::query(
            "SELECT id, user_id, title, content, created_at, updated_at \
             FROM notepad WHERE user_id = ? ORDER BY updated_at DESC LIMIT 1",
        )
        .bind(auth.user.id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?
    };

    if let Some(row) = row {
        Ok(ok(json!({
            "id": row.get::<i64, _>("id"),
            "userId": row.get::<i64, _>("user_id"),
            "title": row.try_get::<Option<String>, _>("title").unwrap_or(None),
            "content": row.try_get::<Option<String>, _>("content").unwrap_or(None),
            "createdAt": row.try_get::<Option<String>, _>("created_at").unwrap_or(None),
            "updatedAt": row.try_get::<Option<String>, _>("updated_at").unwrap_or(None),
        })))
    } else {
        Ok(ok(Value::Null))
    }
}

pub async fn panel_notepad_get_list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let rows = sqlx::query(
        "SELECT id, user_id, title, content, created_at, updated_at \
         FROM notepad WHERE user_id = ? ORDER BY updated_at DESC",
    )
    .bind(auth.user.id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::db(e.to_string()))?;

    let list: Vec<Value> = rows
        .into_iter()
        .map(|row| {
            json!({
                "id": row.get::<i64, _>("id"),
                "userId": row.get::<i64, _>("user_id"),
                "title": row.try_get::<Option<String>, _>("title").unwrap_or(None),
                "content": row.try_get::<Option<String>, _>("content").unwrap_or(None),
                "createdAt": row.try_get::<Option<String>, _>("created_at").unwrap_or(None),
                "updatedAt": row.try_get::<Option<String>, _>("updated_at").unwrap_or(None),
            })
        })
        .collect();
    Ok(ok(list))
}

pub async fn panel_notepad_save(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<Value>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let id = parse_i64(req.get("id"));
    let title = parse_string(req.get("title"));
    let content = parse_string(req.get("content"));

    if id > 0 {
        sqlx::query(
            "UPDATE notepad SET title = ?, content = ?, updated_at = CURRENT_TIMESTAMP \
             WHERE id = ? AND user_id = ?",
        )
        .bind(title.clone())
        .bind(content.clone())
        .bind(id)
        .bind(auth.user.id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;

        Ok(ok(json!({
            "id": id,
            "userId": auth.user.id,
            "title": title,
            "content": content,
        })))
    } else {
        let res = sqlx::query(
            "INSERT INTO notepad (user_id, title, content, created_at, updated_at) \
             VALUES (?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind(auth.user.id)
        .bind(title.clone())
        .bind(content.clone())
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;

        Ok(ok(json!({
            "id": res.last_insert_rowid(),
            "userId": auth.user.id,
            "title": title,
            "content": content,
        })))
    }
}

pub async fn panel_notepad_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<Value>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let id = parse_i64(req.get("id"));
    sqlx::query("DELETE FROM notepad WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(auth.user.id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;
    Ok(ok_empty())
}

pub async fn panel_notepad_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    if let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::new(1300, e.to_string()))?
    {
        let file_name = field.file_name().unwrap_or("notepad.bin").to_string();
        let ext = Path::new(&file_name)
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_lowercase();
        let allow = [
            "png", "jpg", "gif", "jpeg", "webp", "ico", "txt", "md", "json", "pdf", "doc",
            "docx", "xls", "xlsx",
        ];
        if !allow.contains(&ext.as_str()) {
            return Err(ApiError::new(-1, "当前文件类型不允许上传"));
        }

        let (relative_db_path, public_url, ext) =
            save_upload_field(&state, auth.user.id, field, Some("notepad")).await?;
        sqlx::query(
            "INSERT INTO file (src, user_id, file_name, method, ext, created_at, updated_at) \
             VALUES (?, ?, ?, 0, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind(relative_db_path)
        .bind(auth.user.id)
        .bind(file_name.clone())
        .bind(ext)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;

        return Ok(ok(json!({
            "url": public_url,
            "name": file_name,
            "type": MimeGuess::from_path(&file_name).first_or_octet_stream().to_string(),
        })));
    }

    Err(ApiError::new(1300, "上传失败"))
}

pub async fn panel_search_engine_get_list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let rows = sqlx::query(
        "SELECT id, icon_src, title, url, sort, user_id, created_at, updated_at \
         FROM search_engine WHERE user_id = ? AND deleted_at IS NULL ORDER BY sort ASC",
    )
    .bind(auth.user.id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::db(e.to_string()))?;

    let list: Vec<Value> = rows
        .into_iter()
        .map(|row| {
            json!({
                "id": row.get::<i64, _>("id"),
                "iconSrc": row.try_get::<Option<String>, _>("icon_src").unwrap_or(None),
                "title": row.try_get::<Option<String>, _>("title").unwrap_or(None),
                "url": row.try_get::<Option<String>, _>("url").unwrap_or(None),
                "sort": row.try_get::<Option<i64>, _>("sort").unwrap_or(Some(0)).unwrap_or(0),
                "userId": row.try_get::<Option<i64>, _>("user_id").unwrap_or(None),
                "createTime": row.try_get::<Option<String>, _>("created_at").unwrap_or(None),
                "updateTime": row.try_get::<Option<String>, _>("updated_at").unwrap_or(None),
            })
        })
        .collect();
    let count = list.len() as i64;
    Ok(list_ok(list, count))
}

pub async fn panel_search_engine_add(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<Value>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let title = parse_string(req.get("title"));
    let url = parse_string(req.get("url"));
    let icon_src = parse_string(req.get("iconSrc"));
    let max_sort: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(sort), 0) FROM search_engine WHERE user_id = ? AND deleted_at IS NULL",
    )
    .bind(auth.user.id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let res = sqlx::query(
        "INSERT INTO search_engine (icon_src, title, url, sort, user_id, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    )
    .bind(icon_src.clone())
    .bind(title.clone())
    .bind(url.clone())
    .bind(max_sort + 1)
    .bind(auth.user.id)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::db(e.to_string()))?;

    Ok(ok(json!({
        "id": res.last_insert_rowid(),
        "iconSrc": icon_src,
        "title": title,
        "url": url,
        "sort": max_sort + 1,
        "userId": auth.user.id,
    })))
}

pub async fn panel_search_engine_update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<Value>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let id = parse_i64(req.get("id"));
    let title = parse_string(req.get("title"));
    let url = parse_string(req.get("url"));
    let icon_src = parse_string(req.get("iconSrc"));
    let sort = parse_i64(req.get("sort"));

    sqlx::query(
        "UPDATE search_engine SET icon_src = ?, title = ?, url = ?, sort = ?, updated_at = CURRENT_TIMESTAMP \
         WHERE id = ? AND user_id = ?",
    )
    .bind(icon_src.clone())
    .bind(title.clone())
    .bind(url.clone())
    .bind(sort)
    .bind(id)
    .bind(auth.user.id)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::db(e.to_string()))?;

    Ok(ok(json!({
        "id": id,
        "iconSrc": icon_src,
        "title": title,
        "url": url,
        "sort": sort,
    })))
}

pub async fn panel_search_engine_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<Value>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let id = parse_i64(req.get("id"));
    sqlx::query("DELETE FROM search_engine WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(auth.user.id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;
    Ok(ok_empty())
}

pub async fn panel_search_engine_update_sort(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<Value>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let items = req
        .get("items")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();

    let mut tx = state.db.begin().await.map_err(|e| ApiError::db(e.to_string()))?;
    for item in items {
        let id = parse_i64(item.get("id"));
        let sort = parse_i64(item.get("sort"));
        sqlx::query(
            "UPDATE search_engine SET sort = ?, updated_at = CURRENT_TIMESTAMP \
             WHERE id = ? AND user_id = ?",
        )
        .bind(sort)
        .bind(id)
        .bind(auth.user.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;
    }
    tx.commit().await.map_err(|e| ApiError::db(e.to_string()))?;
    Ok(ok_empty())
}
