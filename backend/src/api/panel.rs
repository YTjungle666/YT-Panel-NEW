use axum::{
    extract::{Query, State},
    Json,
};
use reqwest;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

use crate::{
    auth::{self, Claims},
    db,
    error::{AppError, Result},
    models::{
        Bookmark, BookmarkForm, BookmarkTreeNode, IconGroup, IconGroupForm, IconItem,
        IconItemForm, Notepad, NotepadForm, Notice, PageForm, SearchEngine, SearchEngineForm,
        User, UserConfig, UserForm,
    },
    AppState,
};

// ==================== 用户配置 ====================

#[derive(Debug, Deserialize)]
pub struct UserConfigGetReq {
    pub key: String,
}

#[derive(Debug, Serialize)]
pub struct UserConfigGetResp {
    pub code: i32,
    pub msg: String,
    pub data: Option<Value>,
}

pub async fn user_config_get(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Json(req): Json<UserConfigGetReq>,
) -> Result<Json<UserConfigGetResp>> {
    let config = sqlx::query_as::<_, UserConfig>(
        "SELECT * FROM user_configs WHERE user_id = ? AND config_key = ?"
    )
    .bind(claims.user_id)
    .bind(&req.key)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let value = config.and_then(|c| c.config_value);

    Ok(Json(UserConfigGetResp {
        code: 200,
        msg: "success".to_string(),
        data: value,
    }))
}

#[derive(Debug, Deserialize)]
pub struct UserConfigSetReq {
    pub key: String,
    pub value: Value,
}

#[derive(Debug, Serialize)]
pub struct UserConfigSetResp {
    pub code: i32,
    pub msg: String,
}

pub async fn user_config_set(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Json(req): Json<UserConfigSetReq>,
) -> Result<Json<UserConfigSetResp>> {
    let config_str = serde_json::to_string(&req.value)
        .map_err(|e| AppError::Validation(e.to_string()))?;

    sqlx::query(
        "INSERT INTO user_configs (user_id, config_key, config_value) VALUES (?, ?, ?)
         ON CONFLICT(user_id, config_key) DO UPDATE SET config_value = excluded.config_value"
    )
    .bind(claims.user_id)
    .bind(&req.key)
    .bind(&config_str)
    .execute(&state.db)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(UserConfigSetResp {
        code: 200,
        msg: "success".to_string(),
    }))
}

// 用户响应结构体（不含敏感信息）
#[derive(Debug, Serialize, FromRow)]
pub struct UserResponse {
    pub id: i64,
    pub username: String,
    pub role: String,
    pub status: i32,
    pub created_at: String,
    pub updated_at: String,
}

// ==================== 用户管理(仅限管理员) ====================

#[derive(Debug, Deserialize)]
pub struct UserCreateReq {
    pub username: String,
    pub password: String,
    pub role: String,
}

pub async fn users_create(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Json(req): Json<UserCreateReq>,
) -> Result<Json<Value>> {
    auth::require_admin(&claims)?;

    let password_hash = crate::auth::hash_password(&req.password).await?;

    sqlx::query(
        "INSERT INTO users (username, password_hash, role) VALUES (?, ?, ?)"
    )
    .bind(&req.username)
    .bind(&password_hash)
    .bind(&req.role)
    .execute(&state.db)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "code": 200,
        "msg": "success"
    })))
}

#[derive(Debug, Deserialize)]
pub struct UserUpdateReq {
    pub id: i64,
    pub username: Option<String>,
    pub password: Option<String>,
    pub role: Option<String>,
    pub status: Option<i32>,
}

pub async fn users_update(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Json(req): Json<UserUpdateReq>,
) -> Result<Json<Value>> {
    auth::require_admin(&claims)?;

    let mut query_parts = Vec::new();
    
    if let Some(ref username) = req.username {
        query_parts.push(format!("username = '{}'", username));
    }
    if let Some(ref role) = req.role {
        query_parts.push(format!("role = '{}'", role));
    }
    if let Some(status) = req.status {
        query_parts.push(format!("status = {}", status));
    }

    if let Some(ref password) = req.password {
        let password_hash = crate::auth::hash_password(password)?;
        query_parts.push(format!("password_hash = '{}'", password_hash));
    }

    if query_parts.is_empty() {
        return Err(AppError::Validation("No fields to update".to_string()));
    }

    let sql = format!("UPDATE users SET {} WHERE id = {}", query_parts.join(", "), req.id);
    
    sqlx::query(&sql)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "code": 200,
        "msg": "success"
    })))
}

#[derive(Debug, Deserialize)]
pub struct UserListReq {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct UserListResp {
    pub code: i32,
    pub msg: String,
    pub data: Vec<UserResponse>,
    pub total: i64,
}

pub async fn users_get_list(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Json(req): Json<UserListReq>,
) -> Result<Json<UserListResp>> {
    auth::require_admin(&claims)?;

    let page = req.page.unwrap_or(1);
    let page_size = req.page_size.unwrap_or(20);
    let offset = (page - 1) * page_size;

    let users = sqlx::query_as::<_, UserResponse>(
        "SELECT id, username, role, status, created_at, updated_at FROM users LIMIT ? OFFSET ?"
    )
    .bind(page_size)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(UserListResp {
        code: 200,
        msg: "success".to_string(),
        data: users,
        total,
    }))
}

#[derive(Debug, Deserialize)]
pub struct UserDeletesReq {
    pub ids: Vec<i64>,
}

pub async fn users_deletes(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Json(req): Json<UserDeletesReq>,
) -> Result<Json<Value>> {
    auth::require_admin(&claims)?;

    let placeholders: Vec<String> = req.ids.iter().map(|_| "?".to_string()).collect();
    let sql = format!("DELETE FROM users WHERE id IN ({})", placeholders.join(","));

    let mut query = sqlx::query(&sql);
    for id in &req.ids {
        query = query.bind(id);
    }

    query.execute(&state.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "code": 200,
        "msg": "success"
    })))
}

pub async fn users_get_public_visit_user(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>> {
    let user = sqlx::query_as::<_, UserResponse>(
        "SELECT id, username, role, status, created_at, updated_at FROM users WHERE role = 'public' LIMIT 1"
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "code": 200,
        "msg": "success",
        "data": user
    })))
}

#[derive(Debug, Deserialize)]
pub struct SetPublicVisitUserReq {
    pub user_id: i64,
}

pub async fn users_set_public_visit_user(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Json(req): Json<SetPublicVisitUserReq>,
) -> Result<Json<Value>> {
    auth::require_admin(&claims)?;

    // 先清除旧的公开用户
    sqlx::query("UPDATE users SET role = 'user' WHERE role = 'public'")
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    // 设置新的公开用户
    sqlx::query("UPDATE users SET role = 'public' WHERE id = ?")
        .bind(req.user_id)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "code": 200,
        "msg": "success"
    })))
}

// ==================== 图标组管理 ====================

#[derive(Debug, Deserialize)]
pub struct IconGroupListReq {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct IconGroupListResp {
    pub code: i32,
    pub msg: String,
    pub data: Vec<IconGroup>,
    pub total: i64,
}

pub async fn item_icon_group_get_list(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
    Json(req): Json<IconGroupListReq>,
) -> Result<Json<IconGroupListResp>> {
    let page = req.page.unwrap_or(1);
    let page_size = req.page_size.unwrap_or(100);
    let offset = (page - 1) * page_size;

    let groups = sqlx::query_as::<_, IconGroup>(
        "SELECT * FROM icon_groups ORDER BY sort_order ASC, id ASC LIMIT ? OFFSET ?"
    )
    .bind(page_size)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM icon_groups")
        .fetch_one(&state.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(IconGroupListResp {
        code: 200,
        msg: "success".to_string(),
        data: groups,
        total,
    }))
}

pub async fn item_icon_group_edit(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
    Json(req): Json<IconGroupForm>,
) -> Result<Json<Value>> {
    if let Some(id) = req.id {
        sqlx::query(
            "UPDATE icon_groups SET name = ?, sort_order = ? WHERE id = ?"
        )
        .bind(&req.name)
        .bind(req.sort_order.unwrap_or(0))
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
    } else {
        sqlx::query(
            "INSERT INTO icon_groups (name, sort_order) VALUES (?, ?)"
        )
        .bind(&req.name)
        .bind(req.sort_order.unwrap_or(0))
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
    }

    Ok(Json(serde_json::json!({
        "code": 200,
        "msg": "success"
    })))
}

#[derive(Debug, Deserialize)]
pub struct IconGroupDeletesReq {
    pub ids: Vec<i64>,
}

pub async fn item_icon_group_deletes(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
    Json(req): Json<IconGroupDeletesReq>,
) -> Result<Json<Value>> {
    let placeholders: Vec<String> = req.ids.iter().map(|_| "?".to_string()).collect();
    let sql = format!("DELETE FROM icon_groups WHERE id IN ({})", placeholders.join(","));

    let mut query = sqlx::query(&sql);
    for id in &req.ids {
        query = query.bind(id);
    }

    query.execute(&state.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "code": 200,
        "msg": "success"
    })))
}

#[derive(Debug, Deserialize)]
pub struct IconGroupSortReq {
    pub sorts: Vec<(i64, i32)>, // (id, sort_order)
}

pub async fn item_icon_group_save_sort(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
    Json(req): Json<IconGroupSortReq>,
) -> Result<Json<Value>> {
    for (id, sort_order) in req.sorts {
        sqlx::query("UPDATE icon_groups SET sort_order = ? WHERE id = ?")
            .bind(sort_order)
            .bind(id)
            .execute(&state.db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
    }

    Ok(Json(serde_json::json!({
        "code": 200,
        "msg": "success"
    })))
}

// ==================== 图标管理 ====================

#[derive(Debug, Deserialize)]
pub struct IconListByGroupReq {
    pub group_id: i64,
}

#[derive(Debug, Serialize)]
pub struct IconListResp {
    pub code: i32,
    pub msg: String,
    pub data: Vec<IconItem>,
}

pub async fn item_icon_get_list_by_group_id(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
    Json(req): Json<IconListByGroupReq>,
) -> Result<Json<IconListResp>> {
    let icons = sqlx::query_as::<_, IconItem>(
        "SELECT * FROM icon_items WHERE group_id = ? ORDER BY sort_order ASC, id ASC"
    )
    .bind(req.group_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(IconListResp {
        code: 200,
        msg: "success".to_string(),
        data: icons,
    }))
}

pub async fn item_icon_edit(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
    Json(req): Json<IconItemForm>,
) -> Result<Json<Value>> {
    if let Some(id) = req.id {
        sqlx::query(
            "UPDATE icon_items SET name = ?, url = ?, icon_url = ?, group_id = ?, sort_order = ? WHERE id = ?"
        )
        .bind(&req.name)
        .bind(&req.url)
        .bind(&req.icon_url)
        .bind(req.group_id)
        .bind(req.sort_order.unwrap_or(0))
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
    } else {
        sqlx::query(
            "INSERT INTO icon_items (name, url, icon_url, group_id, sort_order) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&req.name)
        .bind(&req.url)
        .bind(&req.icon_url)
        .bind(req.group_id)
        .bind(req.sort_order.unwrap_or(0))
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
    }

    Ok(Json(serde_json::json!({
        "code": 200,
        "msg": "success"
    })))
}

#[derive(Debug, Deserialize)]
pub struct IconAddMultipleReq {
    pub group_id: i64,
    pub icons: Vec<IconItemForm>,
}

pub async fn item_icon_add_multiple(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
    Json(req): Json<IconAddMultipleReq>,
) -> Result<Json<Value>> {
    for icon in req.icons {
        sqlx::query(
            "INSERT INTO icon_items (name, url, icon_url, group_id, sort_order) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&icon.name)
        .bind(&icon.url)
        .bind(&icon.icon_url)
        .bind(req.group_id)
        .bind(icon.sort_order.unwrap_or(0))
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
    }

    Ok(Json(serde_json::json!({
        "code": 200,
        "msg": "success"
    })))
}

#[derive(Debug, Deserialize)]
pub struct IconDeletesReq {
    pub ids: Vec<i64>,
}

pub async fn item_icon_deletes(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
    Json(req): Json<IconDeletesReq>,
) -> Result<Json<Value>> {
    let placeholders: Vec<String> = req.ids.iter().map(|_| "?".to_string()).collect();
    let sql = format!("DELETE FROM icon_items WHERE id IN ({})", placeholders.join(","));

    let mut query = sqlx::query(&sql);
    for id in &req.ids {
        query = query.bind(id);
    }

    query.execute(&state.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "code": 200,
        "msg": "success"
    })))
}

#[derive(Debug, Deserialize)]
pub struct IconSortReq {
    pub sorts: Vec<(i64, i32)>,
}

pub async fn item_icon_save_sort(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
    Json(req): Json<IconSortReq>,
) -> Result<Json<Value>> {
    for (id, sort_order) in req.sorts {
        sqlx::query("UPDATE icon_items SET sort_order = ? WHERE id = ?")
            .bind(sort_order)
            .bind(id)
            .execute(&state.db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
    }

    Ok(Json(serde_json::json!({
        "code": 200,
        "msg": "success"
    })))
}

#[derive(Debug, Deserialize)]
pub struct FaviconReq {
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct FaviconResp {
    pub code: i32,
    pub msg: String,
    pub data: Option<String>,
}

pub async fn item_icon_get_site_favicon(
    State(state): State<Arc<AppState>>,
    Json(req): Json<FaviconReq>,
) -> Result<Json<FaviconResp>> {
    // 检查缓存
    if let Some(cached) = db::get_favicon_cache(&state.db, &req.url).await? {
        return Ok(Json(FaviconResp {
            code: 200,
            msg: "success".to_string(),
            data: Some(cached),
        }));
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let resp = client.get(&req.url)
        .send()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let html = resp.text()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let document = Html::parse_document(&html);
    
    // 尝试多种方式获取favicon
    let selectors = [
        "link[rel=\"icon\"][type=\"image/x-icon\"]",
        "link[rel=\"icon\"][sizes=\"32x32\"]",
        "link[rel=\"icon\"]",
        "link[rel=\"shortcut icon\"]",
        "link[rel=\"apple-touch-icon\"]",
    ];

    let mut favicon_url = None;
    
    for selector_str in &selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            if let Some(element) = document.select(&selector).next() {
                if let Some(href) = element.value().attr("href") {
                    favicon_url = Some(href.to_string());
                    break;
                }
            }
        }
    }

    // 如果没有找到，尝试默认路径
    let favicon = if let Some(url) = favicon_url {
        if url.starts_with("http") {
            url
        } else if url.starts_with("//") {
            format!("https:{}", url)
        } else if url.starts_with("/") {
            let base_url = req.url.trim_end_matches('/');
            if let Some(domain_end) = base_url.find("/") {
                let base = &base_url[..domain_end + base_url[domain_end..].find("/").unwrap_or(base_url.len() - domain_end)];
                format!("{}{}", base, url)
            } else {
                format!("{}/{}", base_url, url.trim_start_matches('/'))
            }
        } else {
            format!("{}/{}", req.url.trim_end_matches('/'), url)
        }
    } else {
        // 默认favicon路径
        let domain = req.url
            .trim_start_matches("http://")
            .trim_start_matches("https://")
            .split('/')
            .next()
            .unwrap_or(&req.url);
        format!("https://{}/favicon.ico", domain)
    };

    // 缓存结果
    db::set_favicon_cache(&state.db, &req.url, &favicon).await?;

    Ok(Json(FaviconResp {
        code: 200,
        msg: "success".to_string(),
        data: Some(favicon),
    }))
}

// ==================== 书签管理 ====================

#[derive(Debug, Deserialize)]
pub struct BookmarkListReq {
    pub user_id: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct BookmarkListResp {
    pub code: i32,
    pub msg: String,
    pub data: Vec<BookmarkTreeNode>,
}

fn build_bookmark_tree(bookmarks: Vec<Bookmark>) -> Vec<BookmarkTreeNode> {
    let mut map: HashMap<i64, BookmarkTreeNode> = HashMap::new();
    let mut roots: Vec<BookmarkTreeNode> = Vec::new();

    // 第一遍：创建所有节点
    for bookmark in &bookmarks {
        let node = BookmarkTreeNode {
            id: bookmark.id,
            user_id: bookmark.user_id,
            title: bookmark.title.clone(),
            url: bookmark.url.clone(),
            icon: bookmark.icon.clone(),
            parent_id: bookmark.parent_id,
            sort_order: bookmark.sort_order,
            is_folder: bookmark.is_folder,
            children: Vec::new(),
            created_at: bookmark.created_at,
            updated_at: bookmark.updated_at,
        };
        map.insert(bookmark.id, node);
    }

    // 第二遍：建立父子关系
    for bookmark in &bookmarks {
        if let Some(parent_id) = bookmark.parent_id {
            if let Some(parent) = map.get_mut(&parent_id) {
                if let Some(child) = map.remove(&bookmark.id) {
                    parent.children.push(child);
                }
            }
        }
    }

    // 剩余的节点是根节点
    for (_, node) in map {
        if node.parent_id.is_none() {
            roots.push(node);
        }
    }

    // 排序
    roots.sort_by_key(|n| (n.sort_order, n.id));
    for root in &mut roots {
        sort_tree_node(root);
    }

    roots
}

fn sort_tree_node(node: &mut BookmarkTreeNode) {
    node.children.sort_by_key(|c| (c.sort_order, c.id));
    for child in &mut node.children {
        sort_tree_node(child);
    }
}

pub async fn bookmark_get_list(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Json(req): Json<BookmarkListReq>,
) -> Result<Json<BookmarkListResp>> {
    let user_id = req.user_id.unwrap_or(claims.user_id);
    
    // 如果不是自己的书签，检查权限
    if user_id != claims.user_id {
        auth::require_admin(&claims)?;
    }

    let bookmarks = sqlx::query_as::<_, Bookmark>(
        "SELECT * FROM bookmarks WHERE user_id = ? ORDER BY sort_order ASC, id ASC"
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let tree = build_bookmark_tree(bookmarks);

    Ok(Json(BookmarkListResp {
        code: 200,
        msg: "success".to_string(),
        data: tree,
    }))
}

pub async fn bookmark_add(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Json(req): Json<BookmarkForm>,
) -> Result<Json<Value>> {
    sqlx::query(
        "INSERT INTO bookmarks (user_id, title, url, icon, parent_id, sort_order, is_folder) 
         VALUES (?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(claims.user_id)
    .bind(&req.title)
    .bind(&req.url)
    .bind(&req.icon)
    .bind(req.parent_id)
    .bind(req.sort_order.unwrap_or(0))
    .bind(req.is_folder.unwrap_or(false))
    .execute(&state.db)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "code": 200,
        "msg": "success"
    })))
}

#[derive(Debug, Deserialize)]
pub struct BookmarkAddMultipleReq {
    pub bookmarks: Vec<BookmarkForm>,
}

pub async fn bookmark_add_multiple(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Json(req): Json<BookmarkAddMultipleReq>,
) -> Result<Json<Value>> {
    for bookmark in req.bookmarks {
        sqlx::query(
            "INSERT INTO bookmarks (user_id, title, url, icon, parent_id, sort_order, is_folder) 
             VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(claims.user_id)
        .bind(&bookmark.title)
        .bind(&bookmark.url)
        .bind(&bookmark.icon)
        .bind(bookmark.parent_id)
        .bind(bookmark.sort_order.unwrap_or(0))
        .bind(bookmark.is_folder.unwrap_or(false))
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
    }

    Ok(Json(serde_json::json!({
        "code": 200,
        "msg": "success"
    })))
}

#[derive(Debug, Deserialize)]
pub struct BookmarkUpdateReq {
    pub id: i64,
    pub title: Option<String>,
    pub url: Option<String>,
    pub icon: Option<String>,
    pub parent_id: Option<Option<i64>>,
    pub sort_order: Option<i32>,
}

pub async fn bookmark_update(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Json(req): Json<BookmarkUpdateReq>,
) -> Result<Json<Value>> {
    let mut query_parts = Vec::new();
    
    if let Some(ref title) = req.title {
        query_parts.push(format!("title = '{}'", title));
    }
    if let Some(ref url) = req.url {
        query_parts.push(format!("url = '{}'", url));
    }
    if let Some(ref icon) = req.icon {
        query_parts.push(format!("icon = '{}'", icon));
    }
    if let Some(parent_id) = req.parent_id {
        query_parts.push(format!("parent_id = {}", parent_id.map(|id| id.to_string()).unwrap_or_else(|| "NULL".to_string())));
    }
    if let Some(sort_order) = req.sort_order {
        query_parts.push(format!("sort_order = {}", sort_order));
    }

    if query_parts.is_empty() {
        return Err(AppError::Validation("No fields to update".to_string()));
    }

    query_parts.push("updated_at = CURRENT_TIMESTAMP".to_string());

    let sql = format!("UPDATE bookmarks SET {} WHERE id = ? AND user_id = ?", query_parts.join(", "));

    sqlx::query(&sql)
        .bind(req.id)
        .bind(claims.user_id)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "code": 200,
        "msg": "success"
    })))
}

#[derive(Debug, Deserialize)]
pub struct BookmarkDeletesReq {
    pub ids: Vec<i64>,
}

pub async fn bookmark_deletes(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Json(req): Json<BookmarkDeletesReq>,
) -> Result<Json<Value>> {
    // 递归删除需要处理父子关系
    let placeholders: Vec<String> = req.ids.iter().map(|_| "?".to_string()).collect();
    let sql = format!(
        "DELETE FROM bookmarks WHERE id IN ({})",
        placeholders.join(",")
    );

    let mut query = sqlx::query(&sql);
    for id in &req.ids {
        query = query.bind(id);
    }

    query.execute(&state.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "code": 200,
        "msg": "success"
    })))
}

// ==================== 记事本管理 ====================

#[derive(Debug, Deserialize)]
pub struct NotepadGetReq {
    pub id: i64,
}

#[derive(Debug, Serialize)]
pub struct NotepadResp {
    pub code: i32,
    pub msg: String,
    pub data: Option<Notepad>,
}

pub async fn notepad_get(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Query(req): Query<NotepadGetReq>,
) -> Result<Json<NotepadResp>> {
    let notepad = sqlx::query_as::<_, Notepad>(
        "SELECT * FROM notepads WHERE id = ? AND user_id = ?"
    )
    .bind(req.id)
    .bind(claims.user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(NotepadResp {
        code: 200,
        msg: "success".to_string(),
        data: notepad,
    }))
}

#[derive(Debug, Deserialize)]
pub struct NotepadListReq {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct NotepadListResp {
    pub code: i32,
    pub msg: String,
    pub data: Vec<Notepad>,
    pub total: i64,
}

pub async fn notepad_get_list(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Query(req): Query<NotepadListReq>,
) -> Result<Json<NotepadListResp>> {
    let page = req.page.unwrap_or(1);
    let page_size = req.page_size.unwrap_or(20);
    let offset = (page - 1) * page_size;

    let notepads = sqlx::query_as::<_, Notepad>(
        "SELECT * FROM notepads WHERE user_id = ? ORDER BY updated_at DESC LIMIT ? OFFSET ?"
    )
    .bind(claims.user_id)
    .bind(page_size)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notepads WHERE user_id = ?")
        .bind(claims.user_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(NotepadListResp {
        code: 200,
        msg: "success".to_string(),
        data: notepads,
        total,
    }))
}

pub async fn notepad_save(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Json(req): Json<NotepadForm>,
) -> Result<Json<Value>> {
    if let Some(id) = req.id {
        sqlx::query(
            "UPDATE notepads SET title = ?, content = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND user_id = ?"
        )
        .bind(&req.title)
        .bind(&req.content)
        .bind(id)
        .bind(claims.user_id)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
    } else {
        sqlx::query(
            "INSERT INTO notepads (user_id, title, content) VALUES (?, ?, ?)"
        )
        .bind(claims.user_id)
        .bind(&req.title)
        .bind(&req.content)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
    }

    Ok(Json(serde_json::json!({
        "code": 200,
        "msg": "success"
    })))
}

#[derive(Debug, Deserialize)]
pub struct NotepadDeleteReq {
    pub id: i64,
}

pub async fn notepad_delete(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Json(req): Json<NotepadDeleteReq>,
) -> Result<Json<Value>> {
    sqlx::query("DELETE FROM notepads WHERE id = ? AND user_id = ?")
        .bind(req.id)
        .bind(claims.user_id)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "code": 200,
        "msg": "success"
    })))
}

pub async fn notepad_upload(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Json(req): Json<NotepadForm>,
) -> Result<Json<Value>> {
    // 文件上传处理，保存到存储
    // 这里简化处理，实际应该处理multipart form
    notepad_save(State(state), claims, Json(req)).await
}

// ==================== 搜索引擎管理 ====================

#[derive(Debug, Deserialize)]
pub struct SearchEngineListReq {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct SearchEngineListResp {
    pub code: i32,
    pub msg: String,
    pub data: Vec<SearchEngine>,
    pub total: i64,
}

pub async fn search_engine_get_list(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
    Json(req): Json<SearchEngineListReq>,
) -> Result<Json<SearchEngineListResp>> {
    let page = req.page.unwrap_or(1);
    let page_size = req.page_size.unwrap_or(100);
    let offset = (page - 1) * page_size;

    let engines = sqlx::query_as::<_, SearchEngine>(
        "SELECT * FROM search_engines ORDER BY sort_order ASC, id ASC LIMIT ? OFFSET ?"
    )
    .bind(page_size)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM search_engines")
        .fetch_one(&state.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(SearchEngineListResp {
        code: 200,
        msg: "success".to_string(),
        data: engines,
        total,
    }))
}

pub async fn search_engine_add(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
    Json(req): Json<SearchEngineForm>,
) -> Result<Json<Value>> {
    sqlx::query(
        "INSERT INTO search_engines (name, url, icon, sort_order) VALUES (?, ?, ?, ?)"
    )
    .bind(&req.name)
    .bind(&req.url)
    .bind(&req.icon)
    .bind(req.sort_order.unwrap_or(0))
    .execute(&state.db)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "code": 200,
        "msg": "success"
    })))
}

#[derive(Debug, Deserialize)]
pub struct SearchEngineUpdateReq {
    pub id: i64,
    pub name: Option<String>,
    pub url: Option<String>,
    pub icon: Option<String>,
    pub sort_order: Option<i32>,
}

pub async fn search_engine_update(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
    Json(req): Json<SearchEngineUpdateReq>,
) -> Result<Json<Value>> {
    let mut query_parts = Vec::new();
    
    if let Some(ref name) = req.name {
        query_parts.push(format!("name = '{}'", name));
    }
    if let Some(ref url) = req.url {
        query_parts.push(format!("url = '{}'", url));
    }
    if let Some(ref icon) = req.icon {
        query_parts.push(format!("icon = '{}'", icon));
    }
    if let Some(sort_order) = req.sort_order {
        query_parts.push(format!("sort_order = {}", sort_order));
    }

    if query_parts.is_empty() {
        return Err(AppError::Validation("No fields to update".to_string()));
    }

    let sql = format!("UPDATE search_engines SET {} WHERE id = ?", query_parts.join(", "));

    sqlx::query(&sql)
        .bind(req.id)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "code": 200,
        "msg": "success"
    })))
}

#[derive(Debug, Deserialize)]
pub struct SearchEngineDeleteReq {
    pub id: i64,
}

pub async fn search_engine_delete(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
    Json(req): Json<SearchEngineDeleteReq>,
) -> Result<Json<Value>> {
    sqlx::query("DELETE FROM search_engines WHERE id = ?")
        .bind(req.id)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "code": 200,
        "msg": "success"
    })))
}

#[derive(Debug, Deserialize)]
pub struct SearchEngineSortReq {
    pub sorts: Vec<(i64, i32)>,
}

pub async fn search_engine_update_sort(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
    Json(req): Json<SearchEngineSortReq>,
) -> Result<Json<Value>> {
    for (id, sort_order) in req.sorts {
        sqlx::query("UPDATE search_engines SET sort_order = ? WHERE id = ?")
            .bind(sort_order)
            .bind(id)
            .execute(&state.db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
    }

    Ok(Json(serde_json::json!({
        "code": 200,
        "msg": "success"
    })))
}

// ==================== 公告管理 ====================

#[derive(Debug, Deserialize)]
pub struct NoticeListByDisplayTypeReq {
    pub display_type: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct NoticeListResp {
    pub code: i32,
    pub msg: String,
    pub data: Vec<Notice>,
    pub total: i64,
}

pub async fn notice_get_list_by_display_type(
    State(state): State<Arc<AppState>>,
    Json(req): Json<NoticeListByDisplayTypeReq>,
) -> Result<Json<NoticeListResp>> {
    let page = req.page.unwrap_or(1);
    let page_size = req.page_size.unwrap_or(20);
    let offset = (page - 1) * page_size;

    let mut query = String::from("SELECT * FROM notices WHERE status = 1");
    
    if let Some(ref display_type) = req.display_type {
        query.push_str(&format!(" AND display_type = '{}'", display_type));
    }
    
    query.push_str(&format!(" ORDER BY sort_order ASC, id ASC LIMIT {} OFFSET {}", page_size, offset));

    let notices = sqlx::query_as::<_, Notice>(&query)
        .fetch_all(&state.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let count_sql = if let Some(ref display_type) = req.display_type {
        format!("SELECT COUNT(*) FROM notices WHERE status = 1 AND display_type = '{}'", display_type)
    } else {
        "SELECT COUNT(*) FROM notices WHERE status = 1".to_string()
    };

    let total: i64 = sqlx::query_scalar(&count_sql)
        .fetch_one(&state.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(NoticeListResp {
        code: 200,
        msg: "success".to_string(),
        data: notices,
        total,
    }))
}
