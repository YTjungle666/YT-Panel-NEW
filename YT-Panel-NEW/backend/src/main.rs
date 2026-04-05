use std::{collections::{HashMap, HashSet}, env, net::IpAddr, path::{Component, Path, PathBuf}, sync::Arc};
mod handlers;

use axum::{
    extract::{Multipart, Query, State},
    http::{header::SET_COOKIE, HeaderMap, HeaderValue},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use bcrypt::{hash, verify};
use chrono::{Datelike, Utc};
use mime_guess::MimeGuess;
use rand::{distributions::Alphanumeric, Rng};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{FromRow, Row, SqlitePool};
use sysinfo::{Disks, System};
use tokio::{fs, io::AsyncWriteExt};
use tower_http::{services::{ServeDir, ServeFile}, trace::TraceLayer};
use tracing::info;
use url::Url;
use once_cell::sync::Lazy;

#[derive(Debug, Clone, Deserialize, Serialize)]
struct AppConfig {
    host: String,
    port: u16,
    database_url: String,
    uploads_dir: String,
    frontend_dist: String,
    max_upload_mb: u64,
    public_user_id: Option<i64>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".into(),
            port: 80,
            database_url: "sqlite://./database/database.db".into(),
            uploads_dir: "./uploads".into(),
            frontend_dist: "../frontend-dist".into(),
            max_upload_mb: 10,
            public_user_id: Some(1),
        }
    }
}

#[derive(Clone)]
struct AppState {
    db: SqlitePool,
    config: Arc<AppConfig>,
}

const SESSION_COOKIE_NAME: &str = "yt_panel_session";
const SESSION_COOKIE_MAX_AGE: i64 = 60 * 60 * 24 * 30;

#[derive(Debug, Clone, Serialize)]
struct ApiEnvelope<T: Serialize> {
    code: i32,
    msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
}

#[derive(Debug, Clone)]
struct ApiError {
    code: i32,
    msg: String,
}

type ApiResult = Result<Response, ApiError>;

impl ApiError {
    fn new(code: i32, msg: impl Into<String>) -> Self {
        Self { code, msg: msg.into() }
    }

    fn bad_param(msg: impl Into<String>) -> Self {
        Self::new(1400, msg)
    }

    fn db(err: impl Into<String>) -> Self {
        Self::new(1200, format!("Database error[{}]", err.into()))
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        Json(ApiEnvelope::<Value> {
            code: self.code,
            msg: self.msg,
            data: None,
        })
        .into_response()
    }
}

fn ok<T: Serialize>(data: T) -> Response {
    Json(ApiEnvelope {
        code: 0,
        msg: "OK".into(),
        data: Some(data),
    })
    .into_response()
}

fn ok_empty() -> Response {
    Json(ApiEnvelope::<Value> {
        code: 0,
        msg: "OK".into(),
        data: None,
    })
    .into_response()
}

fn list_ok<T: Serialize>(list: T, count: i64) -> Response {
    ok(json!({ "list": list, "count": count }))
}

#[derive(Debug, Clone)]
struct CurrentUser {
    id: i64,
    username: String,
    password: String,
    name: String,
    head_image: Option<String>,
    status: i64,
    role: i64,
    mail: Option<String>,
    referral_code: Option<String>,
    token: Option<String>,
}

#[derive(Debug, Clone, Copy)]
enum AccessMode {
    LoginRequired,
    PublicAllowed,
}

#[derive(Debug, Clone)]
struct AuthContext {
    user: CurrentUser,
    visit_mode: i32,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct BookmarkNode {
    id: i64,
    create_time: Option<String>,
    icon_json: Option<String>,
    title: String,
    url: String,
    lan_url: Option<String>,
    sort: i64,
    is_folder: i64,
    parent_url: Option<String>,
    parent_id: i64,
    children: Vec<BookmarkNode>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = Arc::new(load_config().await?);
    ensure_parent_dirs(&config).await?;

    let db = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await?;

    init_db(&db).await?;
    seed_defaults(&db, &config).await?;

    let state = AppState {
        db,
        config: config.clone(),
    };

    let frontend_dir = PathBuf::from(&config.frontend_dist);
    let frontend_index = frontend_dir.join("index.html");

    let api = Router::new()
        .route("/api/login", post(login))
        .route("/api/crypto-key", get(get_crypto_key))
        .route("/api/logout", post(logout))
        .route("/api/login/sendResetPasswordVCode", post(login_send_reset_password_vcode))
        .route("/api/login/resetPasswordByVCode", post(login_reset_password_by_vcode))
        .route("/api/register/commit", post(register_commit))
        .route("/api/about", post(about))
        .route("/api/isLan", get(is_lan))
        .route("/api/user/getInfo", post(user_get_info))
        .route("/api/user/getAuthInfo", post(user_get_auth_info))
        .route("/api/user/updateInfo", post(user_update_info))
        .route("/api/user/updatePassword", post(user_update_password))
        .route("/api/user/getReferralCode", post(user_get_referral_code))
        .route("/api/notice/getListByDisplayType", post(notice_get_list))
        .route("/api/system/moduleConfig/getByName", post(module_config_get))
        .route("/api/system/moduleConfig/save", post(module_config_save))
        .route("/api/system/setting/set", post(system_setting_set))
        .route("/api/system/setting/get", post(system_setting_get))
        .route("/api/system/setting/getSingle", post(system_setting_get_single))
        .route("/api/system/monitor/getAll", post(system_monitor_get_all))
        .route("/api/system/monitor/getCpuState", post(system_monitor_get_cpu))
        .route("/api/system/monitor/getMemonyState", post(system_monitor_get_memory))
        .route("/api/system/monitor/getDiskStateByPath", post(system_monitor_get_disk))
        .route("/api/system/monitor/getDiskMountpoints", post(system_monitor_get_mountpoints))
        .route("/api/openness/loginConfig", get(openness_login_config))
        .route("/api/openness/getDisclaimer", get(openness_get_disclaimer))
        .route("/api/openness/getAboutDescription", get(openness_get_about_description))
        .route("/api/file/uploadImg", post(file_upload_img))
        .route("/api/file/uploadFiles", post(file_upload_files))
        .route("/api/file/getList", post(file_get_list))
        .route("/api/file/deletes", post(file_deletes))
        .route("/api/panel/userConfig/get", post(panel_user_config_get))
        .route("/api/panel/userConfig/set", post(panel_user_config_set))
        .route("/api/panel/users/create", post(panel_users_create))
        .route("/api/panel/users/update", post(panel_users_update))
        .route("/api/panel/users/getList", post(panel_users_get_list))
        .route("/api/panel/users/deletes", post(panel_users_deletes))
        .route("/api/panel/users/getPublicVisitUser", post(panel_users_get_public_visit_user))
        .route("/api/panel/users/setPublicVisitUser", post(panel_users_set_public_visit_user))
        .route("/api/panel/itemIconGroup/getList", post(panel_item_icon_group_get_list))
        .route("/api/panel/itemIconGroup/edit", post(panel_item_icon_group_edit))
        .route("/api/panel/itemIconGroup/deletes", post(panel_item_icon_group_deletes))
        .route("/api/panel/itemIconGroup/saveSort", post(panel_item_icon_group_save_sort))
        .route("/api/panel/itemIcon/getListByGroupId", post(panel_item_icon_get_list_by_group_id))
        .route("/api/panel/itemIcon/edit", post(panel_item_icon_edit))
        .route("/api/panel/itemIcon/addMultiple", post(panel_item_icon_add_multiple))
        .route("/api/panel/itemIcon/deletes", post(panel_item_icon_deletes))
        .route("/api/panel/itemIcon/saveSort", post(panel_item_icon_save_sort))
        .route("/api/panel/itemIcon/getSiteFavicon", post(panel_item_icon_get_site_favicon))
        .route("/api/panel/bookmark/getList", post(panel_bookmark_get_list))
        .route("/api/panel/bookmark/add", post(panel_bookmark_add))
        .route("/api/panel/bookmark/addMultiple", post(panel_bookmark_add_multiple))
        .route("/api/panel/bookmark/update", post(panel_bookmark_update))
        .route("/api/panel/bookmark/deletes", post(panel_bookmark_deletes))
        .route("/api/panel/notepad/get", get(panel_notepad_get))
        .route("/api/panel/notepad/getList", get(panel_notepad_get_list))
        .route("/api/panel/notepad/save", post(panel_notepad_save))
        .route("/api/panel/notepad/delete", post(panel_notepad_delete))
        .route("/api/panel/notepad/upload", post(panel_notepad_upload))
        .route("/api/panel/searchEngine/getList", post(panel_search_engine_get_list))
        .route("/api/panel/searchEngine/add", post(panel_search_engine_add))
        .route("/api/panel/searchEngine/update", post(panel_search_engine_update))
        .route("/api/panel/searchEngine/delete", post(panel_search_engine_delete))
        .route("/api/panel/searchEngine/updateSort", post(panel_search_engine_update_sort))
        .route("/api/search/bookmarks", get(search_bookmarks))
        .route("/api/search/suggestions", get(search_suggestions))
        .route("/ping", get(ping))
        .with_state(state.clone());

    let app = api
        .nest_service("/uploads", ServeDir::new(config.uploads_dir.clone()))
        .fallback_service(ServeDir::new(frontend_dir).fallback(ServeFile::new(frontend_index)))
        .layer(TraceLayer::new_for_http());

    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("YT-panel-Rust backend listening on {}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}

async fn load_config() -> anyhow::Result<AppConfig> {
    let mut candidates = Vec::<PathBuf>::new();
    if let Some(path) = env::var("YT_PANEL_CONFIG").ok().filter(|value| !value.trim().is_empty()) {
        candidates.push(PathBuf::from(path));
    }
    candidates.push(PathBuf::from("config/app.toml"));
    candidates.push(PathBuf::from("config/example.toml"));

    for path in candidates {
        if path.exists() {
            let content = fs::read_to_string(path).await?;
            return Ok(toml::from_str(&content).unwrap_or_default());
        }
    }

    Ok(AppConfig::default())
}

async fn ensure_parent_dirs(config: &AppConfig) -> anyhow::Result<()> {
    fs::create_dir_all(&config.uploads_dir).await?;

    let Some(db_path) = sqlite_file_path(&config.database_url) else {
        return Ok(());
    };

    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent).await?;
    }

    if fs::metadata(&db_path).await.is_err() {
        fs::File::create(&db_path).await?;
    }

    Ok(())
}

fn sqlite_file_path(database_url: &str) -> Option<PathBuf> {
    let raw = database_url.trim();
    if raw.is_empty() || raw.eq_ignore_ascii_case("sqlite::memory:") {
        return None;
    }

    let path_part = raw
        .strip_prefix("sqlite://")
        .or_else(|| raw.strip_prefix("sqlite:"))
        .unwrap_or(raw)
        .split('?')
        .next()
        .unwrap_or("")
        .trim();

    if path_part.is_empty() || path_part.eq_ignore_ascii_case(":memory:") {
        return None;
    }

    Some(PathBuf::from(path_part))
}

#[cfg(test)]
mod tests {
    use super::{extract_icon_candidates_from_html, extract_manifest_icon_candidates, sqlite_file_path};
    use std::path::PathBuf;
    use url::Url;

    #[test]
    fn sqlite_file_path_handles_file_backed_urls() {
        assert_eq!(
            sqlite_file_path("sqlite://./database/database.db"),
            Some(PathBuf::from("./database/database.db"))
        );
        assert_eq!(
            sqlite_file_path("sqlite:///app/database/database.db"),
            Some(PathBuf::from("/app/database/database.db"))
        );
        assert_eq!(
            sqlite_file_path("sqlite:////tmp/test.db?mode=rwc"),
            Some(PathBuf::from("//tmp/test.db"))
        );
    }

    #[test]
    fn sqlite_file_path_ignores_memory_urls() {
        assert_eq!(sqlite_file_path("sqlite::memory:"), None);
        assert_eq!(sqlite_file_path("sqlite::memory:?cache=shared"), None);
        assert_eq!(sqlite_file_path(""), None);
    }

    #[test]
    fn extract_icon_candidates_prefers_html_links_and_manifest() {
        let base = Url::parse("https://example.com/app/index.html").unwrap();
        let html = r#"
            <html><head>
              <link rel="icon" sizes="32x32" href="/favicon-32.png">
              <link rel="apple-touch-icon" href="/apple-touch.png">
              <link rel="manifest" href="/site.webmanifest">
            </head></html>
        "#;

        let (icons, manifests) = extract_icon_candidates_from_html(&base, html);
        let icon_urls: Vec<String> = icons.into_iter().map(|url| url.to_string()).collect();
        assert!(icon_urls.contains(&"https://example.com/apple-touch.png".to_string()));
        assert!(icon_urls.contains(&"https://example.com/favicon-32.png".to_string()));
        assert_eq!(manifests[0].as_str(), "https://example.com/site.webmanifest");
    }

    #[test]
    fn extract_manifest_icon_candidates_resolves_relative_urls() {
        let manifest = Url::parse("https://example.com/site.webmanifest").unwrap();
        let text = r#"{
          "icons": [
            {"src": "/icon-192.png", "sizes": "192x192", "type": "image/png"},
            {"src": "./icon.svg", "sizes": "any", "type": "image/svg+xml"}
          ]
        }"#;

        let icons = extract_manifest_icon_candidates(&manifest, text);
        assert_eq!(icons[0].as_str(), "https://example.com/icon-192.png");
        assert_eq!(icons[1].as_str(), "https://example.com/icon.svg");
    }
}

async fn init_db(db: &SqlitePool) -> anyhow::Result<()> {
    let statements = [
        r#"CREATE TABLE IF NOT EXISTS user (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
            username TEXT NOT NULL UNIQUE,
            password TEXT NOT NULL,
            name TEXT,
            head_image TEXT,
            status INTEGER DEFAULT 1,
            role INTEGER DEFAULT 2,
            mail TEXT,
            referral_code TEXT,
            token TEXT
        )"#,
        r#"CREATE TABLE IF NOT EXISTS item_icon_group (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
            icon TEXT,
            title TEXT,
            description TEXT,
            sort INTEGER DEFAULT 0,
            user_id INTEGER
        )"#,
        r#"CREATE TABLE IF NOT EXISTS item_icon (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
            icon_json TEXT,
            title TEXT,
            url TEXT,
            lan_url TEXT,
            description TEXT,
            open_method INTEGER DEFAULT 1,
            sort INTEGER DEFAULT 0,
            item_icon_group_id INTEGER,
            lan_only INTEGER DEFAULT 0,
            user_id INTEGER
        )"#,
        r#"CREATE TABLE IF NOT EXISTS bookmark (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            icon_json TEXT,
            title TEXT,
            url TEXT,
            lan_url TEXT,
            sort INTEGER DEFAULT 0,
            is_folder INTEGER DEFAULT 0,
            parent_url TEXT,
            parent_id INTEGER DEFAULT 0,
            user_id INTEGER
        )"#,
        r#"CREATE TABLE IF NOT EXISTS notepad (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
            user_id INTEGER,
            title TEXT,
            content TEXT
        )"#,
        r#"CREATE TABLE IF NOT EXISTS search_engine (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
            icon_src TEXT,
            title TEXT,
            url TEXT,
            sort INTEGER DEFAULT 0,
            user_id INTEGER,
            deleted_at TEXT
        )"#,
        r#"CREATE TABLE IF NOT EXISTS system_setting (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            config_name TEXT UNIQUE,
            config_value TEXT
        )"#,
        r#"CREATE TABLE IF NOT EXISTS module_config (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
            user_id INTEGER,
            name TEXT,
            value_json TEXT
        )"#,
        r#"CREATE TABLE IF NOT EXISTS user_config (
            user_id INTEGER PRIMARY KEY,
            panel_json TEXT,
            search_engine_json TEXT
        )"#,
        r#"CREATE TABLE IF NOT EXISTS favicon_cache (
            cache_key TEXT PRIMARY KEY,
            source_url TEXT,
            icon_data_url TEXT NOT NULL,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP
        )"#,
        r#"CREATE TABLE IF NOT EXISTS file (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
            src TEXT,
            user_id INTEGER,
            file_name TEXT,
            method INTEGER DEFAULT 0,
            ext TEXT
        )"#,
        r#"CREATE TABLE IF NOT EXISTS notice (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
            title TEXT,
            content TEXT,
            display_type INTEGER,
            one_read INTEGER DEFAULT 0,
            url TEXT,
            is_login INTEGER DEFAULT 0,
            user_id INTEGER
        )"#,
        r#"CREATE UNIQUE INDEX IF NOT EXISTS idx_user_username ON user(username)"#,
        r#"CREATE INDEX IF NOT EXISTS idx_user_mail ON user(mail)"#,
        r#"CREATE INDEX IF NOT EXISTS idx_user_token ON user(token)"#,
        r#"CREATE INDEX IF NOT EXISTS idx_item_icon_group_user_sort_created ON item_icon_group(user_id, sort, created_at)"#,
        r#"CREATE INDEX IF NOT EXISTS idx_item_icon_user_group_sort_created ON item_icon(item_icon_group_id, user_id, sort, created_at)"#,
        r#"CREATE INDEX IF NOT EXISTS idx_bookmark_user_parent_sort_created ON bookmark(user_id, parent_id, sort, created_at)"#,
        r#"CREATE INDEX IF NOT EXISTS idx_notepad_user_updated ON notepad(user_id, updated_at)"#,
        r#"CREATE INDEX IF NOT EXISTS idx_search_engine_user_deleted_sort ON search_engine(user_id, deleted_at, sort)"#,
        r#"CREATE INDEX IF NOT EXISTS idx_module_config_user_name ON module_config(user_id, name)"#,
        r#"CREATE INDEX IF NOT EXISTS idx_file_user_created ON file(user_id, created_at)"#,
        r#"CREATE INDEX IF NOT EXISTS idx_notice_display_type ON notice(display_type)"#,
    ];

    for sql in statements {
        sqlx::query(sql).execute(db).await?;
    }

    Ok(())
}

async fn seed_defaults(db: &SqlitePool, config: &AppConfig) -> anyhow::Result<()> {
    let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user")
        .fetch_one(db)
        .await
        .unwrap_or(0);

    if user_count == 0 {
        let password = hash("123456", 12)?;
        let token = random_token(48);
        sqlx::query(
            "INSERT INTO user (username, password, name, status, role, token, created_at, updated_at) VALUES (?, ?, ?, 1, 1, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind("admin")
        .bind(password)
        .bind("admin")
        .bind(token)
        .execute(db)
        .await?;
    }

    ensure_setting(db, "system_application", default_system_application_value().to_string()).await?;
    ensure_setting(db, "disclaimer", "".to_string()).await?;
    ensure_setting(db, "web_about_description", "".to_string()).await?;
    if let Some(public_user_id) = config.public_user_id {
        ensure_setting(db, "panel_public_user_id", public_user_id.to_string()).await?;
    }
    Ok(())
}

async fn ensure_setting(db: &SqlitePool, key: &str, value: String) -> anyhow::Result<()> {
    let exists: Option<i64> = sqlx::query_scalar("SELECT id FROM system_setting WHERE config_name = ?")
        .bind(key)
        .fetch_optional(db)
        .await?;
    if exists.is_none() {
        sqlx::query("INSERT INTO system_setting (config_name, config_value) VALUES (?, ?)")
            .bind(key)
            .bind(value)
            .execute(db)
            .await?;
    }
    Ok(())
}

fn random_token(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

fn session_cookie_value(headers: &HeaderMap) -> Option<String> {
    headers
        .get("cookie")
        .and_then(|value| value.to_str().ok())
        .and_then(|cookie_header| {
            cookie_header.split(';').find_map(|part| {
                let (key, value) = part.trim().split_once('=')?;
                (key.trim() == SESSION_COOKIE_NAME).then(|| value.trim().to_string())
            })
        })
        .filter(|value| !value.is_empty())
}

fn build_session_cookie(token: &str) -> String {
    format!(
        "{}={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
        SESSION_COOKIE_NAME, token, SESSION_COOKIE_MAX_AGE
    )
}

fn build_cleared_session_cookie() -> String {
    format!(
        "{}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0; Expires=Thu, 01 Jan 1970 00:00:00 GMT",
        SESSION_COOKIE_NAME
    )
}

fn with_set_cookie(mut response: Response, cookie: &str) -> Result<Response, ApiError> {
    let value = HeaderValue::from_str(cookie).map_err(|e| ApiError::new(-1, e.to_string()))?;
    response.headers_mut().append(SET_COOKIE, value);
    Ok(response)
}

fn parse_i64(input: Option<&Value>) -> i64 {
    input
        .and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
        .unwrap_or(0)
}

fn parse_string(input: Option<&Value>) -> String {
    input.and_then(|v| v.as_str()).unwrap_or_default().to_string()
}

fn parse_opt_string(input: Option<&Value>) -> Option<String> {
    input.and_then(|v| v.as_str()).map(ToString::to_string)
}

async fn verify_password_compat(plain: &str, stored: &str) -> bool {
    if stored.starts_with("$2") {
        verify(plain, stored).unwrap_or(false)
    } else {
        let plain = plain.to_string();
        let stored = stored.to_string();
        tokio::task::spawn_blocking(move || {
            format!("{:x}", md5::compute(plain)) == stored
        }).await.unwrap_or(false)
    }
}
// 统一的用户加载函数（安全版本）
async fn load_user_by(db: &SqlitePool, field: &str, value: &str) -> Result<Option<CurrentUser>, ApiError> {
    // 白名单校验字段名，防止SQL注入
    let column = match field {
        "username" | "mail" | "id" | "token" => field,
        _ => return Err(ApiError::bad_param("Invalid field")),
    };
    let query = format!(
        "SELECT id, username, password, name, head_image, status, role, mail, referral_code, token FROM user WHERE {} = ?",
        column
    );
    let row = sqlx::query(&query)
        .bind(value)
        .fetch_optional(db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;
    Ok(row.map(row_to_user))
}

// 向后兼容的包装函数
async fn load_user_by_username(db: &SqlitePool, username: &str) -> Result<Option<CurrentUser>, ApiError> {
    load_user_by(db, "username", username).await
}

async fn load_user_by_mail(db: &SqlitePool, mail: &str) -> Result<Option<CurrentUser>, ApiError> {
    load_user_by(db, "mail", mail).await
}

async fn load_user_by_id(db: &SqlitePool, id: i64) -> Result<Option<CurrentUser>, ApiError> {
    load_user_by(db, "id", &id.to_string()).await
}

async fn load_user_by_persistent_token(db: &SqlitePool, token: &str) -> Result<Option<CurrentUser>, ApiError> {
    load_user_by(db, "token", token).await
}

fn row_to_user(row: sqlx::sqlite::SqliteRow) -> CurrentUser {
    CurrentUser {
        id: row.get("id"),
        username: row.get("username"),
        password: row.get("password"),
        name: row.try_get::<Option<String>, _>("name").unwrap_or(None).unwrap_or_default(),
        head_image: row.try_get("head_image").unwrap_or(None),
        status: row.get::<i64, _>("status"),
        role: row.get::<i64, _>("role"),
        mail: row.try_get("mail").unwrap_or(None),
        referral_code: row.try_get("referral_code").unwrap_or(None),
        token: row.try_get("token").unwrap_or(None),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct RegisterConfig {
    open_register: bool,
    #[serde(default)]
    email_suffix: String,
}

fn default_system_application_value() -> Value {
    json!({
        "loginCaptcha": false,
        "register": {
            "openRegister": false,
            "emailSuffix": "",
        },
    })
}

fn parse_register_config(input: Option<&Value>) -> RegisterConfig {
    match input {
        Some(Value::Bool(enabled)) => RegisterConfig {
            open_register: *enabled,
            email_suffix: String::new(),
        },
        Some(Value::Object(_)) => serde_json::from_value(input.cloned().unwrap_or(Value::Null)).unwrap_or_default(),
        _ => RegisterConfig::default(),
    }
}

fn validate_register_username(username: &str) -> Result<(), ApiError> {
    if !(3..=80).contains(&username.len()) {
        return Err(ApiError::bad_param("Username length must be between 3 and 80 characters"));
    }
    if !username.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '@')) {
        return Err(ApiError::bad_param("Username can only contain letters, numbers, _, . and @"));
    }
    Ok(())
}

fn validate_register_password(password: &str) -> Result<(), ApiError> {
    if !(6..=64).contains(&password.len()) {
        return Err(ApiError::bad_param("Password length must be between 6 and 64 characters"));
    }
    if password.chars().any(char::is_whitespace) {
        return Err(ApiError::bad_param("Password cannot contain whitespace"));
    }
    Ok(())
}

fn validate_register_email(email: &str) -> Result<(), ApiError> {
    let email_regex = Regex::new(r"^\w+([-.+]\w+)*@\w+([-.]\w+)*\.\w+([-.]\w+)*$").unwrap();
    if !email_regex.is_match(email) {
        return Err(ApiError::bad_param("Invalid email address"));
    }
    Ok(())
}

fn password_reset_not_configured() -> ApiError {
    ApiError::new(
        1503,
        "Password reset email is not configured yet. Please contact the administrator to reset your password.",
    )
}

async fn get_setting(db: &SqlitePool, key: &str) -> Result<Option<String>, ApiError> {
    sqlx::query_scalar("SELECT config_value FROM system_setting WHERE config_name = ?")
        .bind(key)
        .fetch_optional(db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))
}

async fn set_setting(db: &SqlitePool, key: &str, value: &str) -> Result<(), ApiError> {
    let exists: Option<i64> = sqlx::query_scalar("SELECT id FROM system_setting WHERE config_name = ?")
        .bind(key)
        .fetch_optional(db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;
    if let Some(id) = exists {
        sqlx::query("UPDATE system_setting SET config_value = ? WHERE id = ?")
            .bind(value)
            .bind(id)
            .execute(db)
            .await
            .map_err(|e| ApiError::db(e.to_string()))?;
    } else {
        sqlx::query("INSERT INTO system_setting (config_name, config_value) VALUES (?, ?)")
            .bind(key)
            .bind(value)
            .execute(db)
            .await
            .map_err(|e| ApiError::db(e.to_string()))?;
    }
    Ok(())
}

async fn authenticate(headers: &HeaderMap, state: &AppState, mode: AccessMode) -> Result<AuthContext, ApiError> {
    let incoming_token = session_cookie_value(headers)
        .or_else(|| {
            headers
                .get(axum::http::header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
                .and_then(|value| value.strip_prefix("Bearer ").or_else(|| value.strip_prefix("bearer ")))
                .map(|value| value.trim().to_string())
        })
        .or_else(|| headers.get("token").and_then(|v| v.to_str().ok()).map(|v| v.to_string()))
        .unwrap_or_default();

    if !incoming_token.is_empty() {
        if let Some(user) = load_user_by_persistent_token(&state.db, &incoming_token).await? {
            return Ok(AuthContext { user, visit_mode: 0 });
        }

        if matches!(mode, AccessMode::LoginRequired) {
            return Err(ApiError::new(1001, "Not logged in yet"));
        }
    }

    if matches!(mode, AccessMode::LoginRequired) {
        return Err(ApiError::new(1000, "Not logged in yet"));
    }

    let public_user_id = match get_setting(&state.db, "panel_public_user_id").await? {
        Some(value) => parse_public_user_id_setting(&value),
        None => state.config.public_user_id,
    }
    .ok_or_else(|| ApiError::new(1001, "Not logged in yet"))?;

    let user = load_user_by_id(&state.db, public_user_id)
        .await?
        .ok_or_else(|| ApiError::new(1001, "Not logged in yet"))?;

    Ok(AuthContext { user, visit_mode: 1 })
}

fn ensure_admin(auth: &AuthContext) -> Result<(), ApiError> {
    if auth.user.role != 1 {
        Err(ApiError::new(1005, "No current permission for operation"))
    } else {
        Ok(())
    }
}

fn parse_public_user_id_setting(raw: &str) -> Option<i64> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("null") {
        return None;
    }

    trimmed.parse::<i64>().ok().or_else(|| {
        serde_json::from_str::<Value>(trimmed).ok().and_then(|value| match value {
            Value::Null => None,
            Value::Number(number) => number.as_i64(),
            Value::String(value) => value.parse::<i64>().ok(),
            _ => None,
        })
    })
}

fn build_user_payload(
    id: i64,
    username: String,
    name: String,
    head_image: Option<String>,
    status: i64,
    role: i64,
    mail: Option<String>,
    referral_code: Option<String>,
    token: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
) -> Value {
    json!({
        "id": id,
        "userId": id,
        "username": username,
        "name": name,
        "headImage": head_image,
        "status": status,
        "role": role,
        "mail": mail,
        "referralCode": referral_code,
        "token": token,
        "createTime": created_at,
        "updateTime": updated_at,
    })
}

fn row_to_user_payload(row: sqlx::sqlite::SqliteRow) -> Value {
    build_user_payload(
        row.get::<i64, _>("id"),
        row.get::<String, _>("username"),
        row.try_get::<Option<String>, _>("name").unwrap_or(None).unwrap_or_default(),
        row.try_get("head_image").unwrap_or(None),
        row.try_get::<Option<i64>, _>("status").unwrap_or(Some(1)).unwrap_or(1),
        row.try_get::<Option<i64>, _>("role").unwrap_or(Some(2)).unwrap_or(2),
        row.try_get("mail").unwrap_or(None),
        row.try_get("referral_code").unwrap_or(None),
        row.try_get("token").unwrap_or(None),
        row.try_get::<Option<String>, _>("created_at").unwrap_or(None),
        row.try_get::<Option<String>, _>("updated_at").unwrap_or(None),
    )
}


#[derive(Serialize)]
struct CryptoKeyResponse {
    code: i32,
    msg: String,
    data: Option<String>,
}

/// GET /api/crypto-key - 获取前端加密密钥
/// 每天轮换一次密钥，增加安全性
async fn get_crypto_key() -> Json<CryptoKeyResponse> {
    // 生成基于日期的密钥（每天变化）
    // 实际应用中可以使用更复杂的密钥生成策略
    let today = chrono::Local::now().format("%Y%m%d").to_string();
    let base_key = format!("yt-panel-key-{}", today);
    
    // 使用简单的哈希让密钥更复杂
    let key = format!("{:x}", md5::compute(&base_key));
    
    Json(CryptoKeyResponse {
        code: 200,
        msg: "success".to_string(),
        data: Some(key),
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegisterCommitRequest {
    username: String,
    password: String,
    email: String,
    #[allow(dead_code)]
    email_vcode: Option<String>,
    #[allow(dead_code)]
    vcode: Option<String>,
    #[allow(dead_code)]
    referral_code: Option<String>,
}

async fn register_commit(State(state): State<AppState>, Json(req): Json<RegisterCommitRequest>) -> ApiResult {
    let raw = get_setting(&state.db, "system_application")
        .await?
        .unwrap_or_else(|| default_system_application_value().to_string());
    let value = serde_json::from_str::<Value>(&raw).unwrap_or_else(|_| default_system_application_value());
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

    let password_hash = hash(password, 12).map_err(|e| ApiError::new(-1, e.to_string()))?;
    let token = random_token(48);
    let result = sqlx::query(
        "INSERT INTO user (username, password, name, status, role, mail, token, created_at, updated_at) VALUES (?, ?, ?, 1, 2, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SendResetPasswordVCodeRequest {
    email: String,
    #[allow(dead_code)]
    verification: Option<Value>,
}

async fn login_send_reset_password_vcode(Json(req): Json<SendResetPasswordVCodeRequest>) -> ApiResult {
    let email = req.email.trim().to_lowercase();
    if email.is_empty() {
        return Err(ApiError::bad_param("Email is required"));
    }
    validate_register_email(&email)?;
    Err(password_reset_not_configured())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResetPasswordByVCodeRequest {
    email: String,
    password: String,
    #[serde(rename = "emailVCode", alias = "emailVcode")]
    email_vcode: Option<String>,
    #[allow(dead_code)]
    verification: Option<Value>,
}

async fn login_reset_password_by_vcode(Json(req): Json<ResetPasswordByVCodeRequest>) -> ApiResult {
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

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
    #[allow(dead_code)]
    vcode: Option<String>,
}

async fn login(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<LoginRequest>) -> ApiResult {
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

    if !user.password.starts_with("$2") {
        let new_hash = hash(&req.password, 12).map_err(|e| ApiError::new(-1, e.to_string()))?;
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

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> ApiResult {
    let _ = state;
    let _ = headers;
    with_set_cookie(ok_empty(), &build_cleared_session_cookie())
}

async fn about() -> ApiResult {
    Ok(ok(json!({
        "versionName": env!("CARGO_PKG_VERSION"),
        "versionCode": 1,
    })))
}

async fn is_lan(headers: HeaderMap) -> ApiResult {
    let ip = extract_client_ip(&headers);
    let is_lan = ip.as_deref().and_then(|s| s.parse::<IpAddr>().ok()).map(is_private_ip).unwrap_or(false);
    Ok(ok(json!({ "isLan": is_lan, "clientIp": ip })))
}

async fn ping() -> &'static str {
    "pong"
}

#[derive(Debug, Deserialize)]
struct SearchRequest {
    query: String,
    #[serde(default = "search_default_limit")]
    limit: i64,
    #[serde(default)]
    search_url: bool,
}

fn search_default_limit() -> i64 {
    20
}

#[derive(Debug, Serialize, FromRow)]
struct BookmarkSearchItem {
    id: i64,
    title: String,
    url: String,
    lan_url: Option<String>,
    icon: Option<String>,
    sort: i64,
    is_folder: i64,
    parent_id: i64,
    score: f64,
}

async fn search_bookmarks(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(req): Query<SearchRequest>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::PublicAllowed).await?;
    let user_id = auth.user.id;

    let query = req.query.trim();
    if query.is_empty() {
        return Ok(ok::<Vec<BookmarkSearchItem>>(vec![]));
    }

    let patterns: Vec<String> = query.split_whitespace().map(|s| format!("%{}%", s)).collect();

    let mut sql = String::from(
        "SELECT id, title, url, lan_url, icon, sort, is_folder, parent_id, \
         CASE WHEN LOWER(title) LIKE LOWER(?) THEN 1.0 \
              WHEN LOWER(url) LIKE LOWER(?) THEN 0.8 \
              ELSE 0.5 END as score \
         FROM bookmark WHERE user_id = ?"
    );

    let mut conditions = vec![];
    for _ in 0..patterns.len() {
        if req.search_url {
            conditions.push("(LOWER(title) LIKE LOWER(?) OR LOWER(url) LIKE LOWER(?))".to_string());
        } else {
            conditions.push("LOWER(title) LIKE LOWER(?)".to_string());
        }
    }

    if !conditions.is_empty() {
        sql.push_str(" AND (");
        sql.push_str(&conditions.join(" OR "));
        sql.push_str(")");
    }
    sql.push_str(" ORDER BY score DESC, sort ASC LIMIT ?");

    let mut query_builder = sqlx::query_as::<_, BookmarkSearchItem>(&sql);

    for pattern in &patterns {
        query_builder = query_builder.bind(pattern).bind(pattern);
    }
    query_builder = query_builder.bind(user_id);

    for pattern in &patterns {
        if req.search_url {
            query_builder = query_builder.bind(pattern).bind(pattern);
        } else {
            query_builder = query_builder.bind(pattern);
        }
    }
    query_builder = query_builder.bind(req.limit);

    let results = query_builder
        .fetch_all(&state.db)
        .await
        .map_err(|e| ApiError::new(1200, format!("Database error[{}]", e)))?;

    Ok(ok(results))
}

async fn search_suggestions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(req): Query<SearchRequest>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::PublicAllowed).await?;
    let user_id = auth.user.id;

    let query = req.query.trim();
    if query.is_empty() || query.len() < 2 {
        return Ok(ok::<Vec<String>>(vec![]));
    }

    let pattern = format!("{}%", query);
    let suggestions: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT title FROM bookmark WHERE user_id = ? AND LOWER(title) LIKE LOWER(?) ORDER BY sort ASC LIMIT 10"
    )
    .bind(user_id)
    .bind(&pattern)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::new(1200, format!("Database error[{}]", e)))?;

    Ok(ok(suggestions))
}


async fn user_get_info(State(state): State<AppState>, headers: HeaderMap) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    Ok(ok(json!({
        "userId": auth.user.id,
        "id": auth.user.id,
        "headImage": auth.user.head_image,
        "name": auth.user.name,
        "role": auth.user.role,
    })))
}

async fn user_get_auth_info(State(state): State<AppState>, headers: HeaderMap) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::PublicAllowed).await?;
    Ok(ok(json!({
        "user": {
            "id": auth.user.id,
            "username": auth.user.username,
            "name": auth.user.name,
            "headImage": auth.user.head_image,
            "role": auth.user.role,
        },
        "visitMode": auth.visit_mode,
    })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateInfoRequest {
    head_image: Option<String>,
    name: String,
}

async fn user_update_info(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<UpdateInfoRequest>) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    if req.name.trim().len() < 2 || req.name.trim().len() > 15 {
        return Err(ApiError::bad_param("name length invalid"));
    }
    sqlx::query("UPDATE user SET head_image = ?, name = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(req.head_image)
        .bind(req.name.trim())
        .bind(auth.user.id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;
    with_set_cookie(ok_empty(), &build_cleared_session_cookie())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdatePasswordRequest {
    old_password: String,
    new_password: String,
}

async fn user_update_password(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<UpdatePasswordRequest>) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let Some(fresh) = load_user_by_id(&state.db, auth.user.id).await? else {
        return Err(ApiError::new(1006, "Account does not exist"));
    };
    if !verify_password_compat(&req.old_password, &fresh.password).await {
        return Err(ApiError::new(1007, "Old password error"));
    }
    let new_hash = hash(req.new_password, 12).map_err(|e| ApiError::new(-1, e.to_string()))?;
    sqlx::query("UPDATE user SET password = ?, token = '', updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(new_hash)
        .bind(auth.user.id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;
    Ok(ok_empty())
}

async fn user_get_referral_code(State(state): State<AppState>, headers: HeaderMap) -> ApiResult {
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NoticeRequest {
    display_type: Vec<i64>,
}

async fn notice_get_list(State(state): State<AppState>, Json(req): Json<NoticeRequest>) -> ApiResult {
    if req.display_type.is_empty() {
        return Ok(list_ok(Vec::<Value>::new(), 0));
    }
    let placeholders = vec!["?"; req.display_type.len()].join(",");
    let sql = format!("SELECT id, title, content, display_type, one_read, url, is_login, user_id, created_at, updated_at FROM notice WHERE display_type IN ({})", placeholders);
    let mut query = sqlx::query(&sql);
    for item in &req.display_type {
        query = query.bind(item);
    }
    let rows = query.fetch_all(&state.db).await.map_err(|e| ApiError::db(e.to_string()))?;
    let list: Vec<Value> = rows.into_iter().map(|row| json!({
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
    })).collect();
    Ok(list_ok(list.clone(), list.len() as i64))
}

#[derive(Deserialize)]
struct NameRequest { name: String }

async fn module_config_get(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<NameRequest>) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::PublicAllowed).await?;
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

#[derive(Deserialize)]
struct ModuleConfigSaveRequest {
    name: String,
    value: Value,
}

async fn module_config_save(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<ModuleConfigSaveRequest>) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let value_json = serde_json::to_string(&req.value).unwrap_or_else(|_| "{}".into());
    let existing: Option<i64> = sqlx::query_scalar("SELECT id FROM module_config WHERE user_id = ? AND name = ?")
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

#[derive(Deserialize)]
struct SystemSettingSetRequest {
    settings: HashMap<String, Value>,
}

async fn system_setting_set(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<SystemSettingSetRequest>) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    ensure_admin(&auth)?;
    for (k, v) in req.settings {
        let value = if v.is_string() { v.as_str().unwrap_or_default().to_string() } else { serde_json::to_string(&v).unwrap_or_else(|_| "{}".into()) };
        set_setting(&state.db, &k, &value).await?;
    }
    Ok(ok_empty())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SystemSettingGetRequest {
    config_names: Option<Vec<String>>,
}

async fn system_setting_get(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<SystemSettingGetRequest>) -> ApiResult {
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
            result.insert(row.get::<String, _>("config_name"), Value::String(row.get::<String, _>("config_value")));
        }
    }
    Ok(ok(Value::Object(result)))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SystemSettingSingleRequest { config_name: String }

async fn system_setting_get_single(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<SystemSettingSingleRequest>) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    ensure_admin(&auth)?;
    let value = get_setting(&state.db, &req.config_name).await?.unwrap_or_default();
    Ok(ok(json!({ "configName": req.config_name, "configValue": value })))
}

async fn system_monitor_get_all(State(state): State<AppState>, headers: HeaderMap) -> ApiResult {
    let _auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    Ok(ok(build_monitor_payload(None)))
}

async fn system_monitor_get_cpu(State(state): State<AppState>, headers: HeaderMap) -> ApiResult {
    let _auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    Ok(ok(build_cpu_payload()))
}

async fn system_monitor_get_memory(State(state): State<AppState>, headers: HeaderMap) -> ApiResult {
    let _auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    Ok(ok(build_memory_payload()))
}

#[derive(Deserialize)]
struct DiskPathRequest { path: String }

async fn system_monitor_get_disk(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<DiskPathRequest>) -> ApiResult {
    let _auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    Ok(ok(build_disk_payload(Some(req.path))))
}

async fn system_monitor_get_mountpoints(State(state): State<AppState>, headers: HeaderMap) -> ApiResult {
    let _auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let disks = Disks::new_with_refreshed_list();
    let list: Vec<Value> = disks.list().iter().map(|d| json!({
        "device": d.name().to_string_lossy(),
        "mountpoint": d.mount_point().to_string_lossy(),
        "fstype": d.file_system().to_string_lossy(),
    })).collect();
    Ok(ok(list))
}

async fn openness_login_config(State(state): State<AppState>) -> ApiResult {
    let raw = get_setting(&state.db, "system_application")
        .await?
        .unwrap_or_else(|| default_system_application_value().to_string());
    let value = serde_json::from_str::<Value>(&raw).unwrap_or_else(|_| default_system_application_value());
    let register = parse_register_config(value.get("register"));
    Ok(ok(json!({
        "loginCaptcha": value.get("loginCaptcha").and_then(|v| v.as_bool()).unwrap_or(false),
        "register": register,
    })))
}

async fn openness_get_disclaimer(State(state): State<AppState>) -> ApiResult {
    Ok(ok(get_setting(&state.db, "disclaimer").await?.unwrap_or_default()))
}

async fn openness_get_about_description(State(state): State<AppState>) -> ApiResult {
    Ok(ok(get_setting(&state.db, "web_about_description").await?.unwrap_or_default()))
}

async fn file_upload_img(State(state): State<AppState>, headers: HeaderMap, mut multipart: Multipart) -> ApiResult {
    let _auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    if let Some(field) = multipart.next_field().await.map_err(|e| ApiError::new(1300, e.to_string()))? {
        let file_name = field.file_name().unwrap_or("image.png").to_string();
        let ext = Path::new(&file_name).extension().and_then(|s| s.to_str()).unwrap_or("png").to_lowercase();
        let allowed = ["png", "jpg", "jpeg", "gif", "webp", "ico"];
        if !allowed.contains(&ext.as_str()) {
            return Err(ApiError::new(1301, "Unsupported file format"));
        }
        let bytes = field.bytes().await.map_err(|e| ApiError::new(1300, e.to_string()))?;
        let mime = MimeGuess::from_ext(&ext).first_or_octet_stream();
        let data_url = format!("data:{};base64,{}", mime, B64.encode(bytes));
        return Ok(ok(json!({ "imageUrl": data_url })));
    }
    Err(ApiError::new(1300, "Upload failed"))
}

async fn file_upload_files(State(state): State<AppState>, headers: HeaderMap, mut multipart: Multipart) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let mut succ_map = serde_json::Map::new();
    let mut err_files = Vec::<String>::new();
    while let Some(field) = multipart.next_field().await.map_err(|e| ApiError::new(1300, e.to_string()))? {
        let file_name = field.file_name().unwrap_or("upload.bin").to_string();
        match save_upload_field(&state, auth.user.id, field, None).await {
            Ok((relative_db_path, public_url, ext)) => {
                sqlx::query("INSERT INTO file (src, user_id, file_name, method, ext, created_at, updated_at) VALUES (?, ?, ?, 0, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
                    .bind(relative_db_path)
                    .bind(auth.user.id)
                    .bind(file_name.clone())
                    .bind(ext)
                    .execute(&state.db)
                    .await
                    .map_err(|e| ApiError::db(e.to_string()))?;
                succ_map.insert(file_name, Value::String(public_url));
            }
            Err(_) => err_files.push(file_name),
        }
    }
    Ok(ok(json!({ "succMap": succ_map, "errFiles": err_files })))
}

async fn file_get_list(State(state): State<AppState>, headers: HeaderMap) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let rows = sqlx::query("SELECT id, src, file_name, created_at, updated_at FROM file WHERE user_id = ? ORDER BY created_at DESC")
        .bind(auth.user.id)
        .fetch_all(&state.db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;
    let list: Vec<Value> = rows.into_iter().map(|row| {
        let src: String = row.get("src");
        json!({
            "id": row.get::<i64, _>("id"),
            "src": src.trim_start_matches('.'),
            "fileName": row.try_get::<Option<String>, _>("file_name").unwrap_or(None),
            "createTime": row.try_get::<Option<String>, _>("created_at").unwrap_or(None),
            "updateTime": row.try_get::<Option<String>, _>("updated_at").unwrap_or(None),
            "path": src,
        })
    }).collect();
    Ok(list_ok(list.clone(), list.len() as i64))
}

#[derive(Deserialize)]
struct IdsRequest { ids: Vec<i64> }

async fn file_deletes(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<IdsRequest>) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    for id in req.ids {
        if let Some(src) = sqlx::query_scalar::<_, String>("SELECT src FROM file WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(auth.user.id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| ApiError::db(e.to_string()))? {
            if let Some(path) = resolve_uploaded_file_path(&state.config, &src) {
                let _ = fs::remove_file(path).await;
            }
            sqlx::query("DELETE FROM file WHERE id = ? AND user_id = ?")
                .bind(id)
                .bind(auth.user.id)
                .execute(&state.db)
                .await
                .map_err(|e| ApiError::db(e.to_string()))?;
        }
    }
    Ok(ok_empty())
}

async fn panel_user_config_get(State(state): State<AppState>, headers: HeaderMap) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::PublicAllowed).await?;
    let row = sqlx::query("SELECT panel_json, search_engine_json FROM user_config WHERE user_id = ?")
        .bind(auth.user.id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;
    let Some(row) = row else {
        return Err(ApiError::new(-1, "No data record found"));
    };
    let panel = row.try_get::<Option<String>, _>("panel_json").unwrap_or(None).and_then(|s| serde_json::from_str::<Value>(&s).ok()).unwrap_or(Value::Null);
    let search_engine = row.try_get::<Option<String>, _>("search_engine_json").unwrap_or(None).and_then(|s| serde_json::from_str::<Value>(&s).ok()).unwrap_or(Value::Null);
    Ok(ok(json!({ "userId": auth.user.id, "panel": panel, "searchEngine": search_engine })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserConfigSetRequest {
    panel: Value,
    search_engine: Option<Value>,
}

async fn panel_user_config_set(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<UserConfigSetRequest>) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let panel_json = serde_json::to_string(&req.panel).unwrap_or_else(|_| "{}".into());
    let search_engine_json = serde_json::to_string(&req.search_engine.unwrap_or(Value::Null)).unwrap_or_else(|_| "{}".into());
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

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct AdminUserUpsertRequest {
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
struct AdminUsersListRequest {
    page: Option<i64>,
    limit: Option<i64>,
    #[serde(rename = "keyword", alias = "keyWord")]
    keyword: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AdminUsersDeleteRequest {
    user_ids: Vec<i64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PublicVisitUserRequest {
    user_id: Option<i64>,
}

async fn panel_users_create(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<AdminUserUpsertRequest>) -> ApiResult {
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
    validate_register_password(password)?;

    if load_user_by_username(&state.db, username).await?.is_some() {
        return Err(ApiError::new(1401, "The username already exists"));
    }

    let mail = req.mail.as_deref().map(str::trim).filter(|value| !value.is_empty()).map(str::to_lowercase);
    if let Some(mail_value) = mail.as_deref() {
        validate_register_email(mail_value)?;
        if load_user_by_mail(&state.db, mail_value).await?.is_some() {
            return Err(ApiError::new(1401, "The email already exists"));
        }
    }

    let role = req.role.unwrap_or(2).clamp(1, 2);
    let status = req.status.unwrap_or(1);
    let name = req.name.as_deref().map(str::trim).filter(|value| !value.is_empty()).unwrap_or(username).to_string();
    let password_hash = hash(password, 12).map_err(|e| ApiError::new(-1, e.to_string()))?;

    let result = sqlx::query(
        "INSERT INTO user (username, password, name, head_image, status, role, mail, token, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, '', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
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
        Some(String::new()),
        None,
        None,
    )))
}

async fn panel_users_update(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<AdminUserUpsertRequest>) -> ApiResult {
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

    let mail = req.mail.as_deref().map(str::trim).filter(|value| !value.is_empty()).map(str::to_lowercase);
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
    let name = req.name.as_deref().map(str::trim).filter(|value| !value.is_empty()).unwrap_or(existing.name.as_str()).to_string();
    let password = req.password.as_deref().unwrap_or("").trim();

    if existing.role == 1 && role != 1 {
        let admin_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user WHERE role = 1 AND id != ?")
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
        validate_register_password(password)?;
        let password_hash = hash(password, 12).map_err(|e| ApiError::new(-1, e.to_string()))?;
        sqlx::query("UPDATE user SET username = ?, password = ?, name = ?, head_image = ?, status = ?, role = ?, mail = ?, token = '', updated_at = CURRENT_TIMESTAMP WHERE id = ?")
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

    Ok(ok(build_user_payload(
        id,
        username.to_string(),
        name,
        req.head_image,
        status,
        role,
        mail,
        existing.referral_code,
        Some(String::new()),
        None,
        None,
    )))
}

async fn panel_users_get_list(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<AdminUsersListRequest>) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    ensure_admin(&auth)?;

    let page = req.page.unwrap_or(1).max(1);
    let limit = req.limit.unwrap_or(10).clamp(1, 200);
    let keyword = req.keyword.unwrap_or_default().trim().to_string();
    let like = format!("%{}%", keyword);
    let offset = (page - 1) * limit;

    let rows = sqlx::query(
        "SELECT id, username, name, head_image, status, role, mail, referral_code, token, created_at, updated_at FROM user WHERE (? = '' OR name LIKE ? OR username LIKE ?) ORDER BY id ASC LIMIT ? OFFSET ?",
    )
    .bind(&keyword)
    .bind(&like)
    .bind(&like)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::db(e.to_string()))?;

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user WHERE (? = '' OR name LIKE ? OR username LIKE ?)")
        .bind(&keyword)
        .bind(&like)
        .bind(&like)
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;

    let list: Vec<Value> = rows.into_iter().map(row_to_user_payload).collect();
    Ok(list_ok(list, count))
}

async fn panel_users_deletes(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<AdminUsersDeleteRequest>) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    ensure_admin(&auth)?;

    if req.user_ids.is_empty() {
        return Ok(ok_empty());
    }

    let mut removed_tokens = Vec::<String>::new();
    let mut tx = state.db.begin().await.map_err(|e| ApiError::db(e.to_string()))?;
    for user_id in &req.user_ids {
        if let Some(token) = sqlx::query_scalar::<_, Option<String>>("SELECT token FROM user WHERE id = ?")
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| ApiError::db(e.to_string()))?
            .flatten()
            .filter(|value| !value.is_empty()) {
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

    let _ = removed_tokens;

    if let Some(raw_value) = get_setting(&state.db, "panel_public_user_id").await? {
        if let Some(public_user_id) = parse_public_user_id_setting(&raw_value) {
            if req.user_ids.contains(&public_user_id) {
                let replacement: Option<i64> = sqlx::query_scalar("SELECT id FROM user ORDER BY CASE WHEN role = 1 THEN 0 ELSE 1 END, id ASC LIMIT 1")
                    .fetch_optional(&state.db)
                    .await
                    .map_err(|e| ApiError::db(e.to_string()))?;
                let new_value = replacement.map(|value| value.to_string()).unwrap_or_else(|| "null".to_string());
                set_setting(&state.db, "panel_public_user_id", &new_value).await?;
            }
        }
    }

    Ok(ok_empty())
}

async fn panel_users_get_public_visit_user(State(state): State<AppState>, headers: HeaderMap) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    ensure_admin(&auth)?;

    let Some(raw_value) = get_setting(&state.db, "panel_public_user_id").await? else {
        return Ok(ok(json!({})));
    };
    let Some(user_id) = parse_public_user_id_setting(&raw_value) else {
        return Ok(ok(json!({})));
    };
    let Some(row) = sqlx::query("SELECT id, username, name, head_image, status, role, mail, referral_code, token, created_at, updated_at FROM user WHERE id = ? LIMIT 1")
        .bind(user_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))? else {
        return Ok(ok(json!({})));
    };

    Ok(ok(row_to_user_payload(row)))
}

async fn panel_users_set_public_visit_user(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<PublicVisitUserRequest>) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    ensure_admin(&auth)?;

    let value = if let Some(user_id) = req.user_id.filter(|value| *value > 0) {
        let exists: Option<i64> = sqlx::query_scalar("SELECT id FROM user WHERE id = ?")
            .bind(user_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| ApiError::db(e.to_string()))?;
        if exists.is_none() {
            return Err(ApiError::new(-1, "No data record found"));
        }
        user_id.to_string()
    } else {
        "null".to_string()
    };

    set_setting(&state.db, "panel_public_user_id", &value).await?;
    Ok(ok_empty())
}

async fn panel_item_icon_group_get_list(State(state): State<AppState>, headers: HeaderMap) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::PublicAllowed).await?;
    let mut rows = sqlx::query("SELECT id, icon, title, description, sort, created_at, updated_at FROM item_icon_group WHERE user_id = ? ORDER BY sort ASC, created_at ASC")
        .bind(auth.user.id)
        .fetch_all(&state.db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;
    if rows.is_empty() {
        sqlx::query("INSERT INTO item_icon_group (icon, title, description, sort, user_id, created_at, updated_at) VALUES (?, ?, '', 0, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
            .bind("material-symbols:ad-group-outline")
            .bind("APP")
            .bind(auth.user.id)
            .execute(&state.db)
            .await
            .map_err(|e| ApiError::db(e.to_string()))?;
        rows = sqlx::query("SELECT id, icon, title, description, sort, created_at, updated_at FROM item_icon_group WHERE user_id = ? ORDER BY sort ASC, created_at ASC")
            .bind(auth.user.id)
            .fetch_all(&state.db)
            .await
            .map_err(|e| ApiError::db(e.to_string()))?;
    }
    let list: Vec<Value> = rows.into_iter().map(|row| json!({
        "id": row.get::<i64, _>("id"),
        "icon": row.try_get::<Option<String>, _>("icon").unwrap_or(None),
        "title": row.try_get::<Option<String>, _>("title").unwrap_or(None),
        "description": row.try_get::<Option<String>, _>("description").unwrap_or(None),
        "sort": row.try_get::<Option<i64>, _>("sort").unwrap_or(Some(0)).unwrap_or(0),
        "createTime": row.try_get::<Option<String>, _>("created_at").unwrap_or(None),
        "updateTime": row.try_get::<Option<String>, _>("updated_at").unwrap_or(None),
    })).collect();
    Ok(list_ok(list.clone(), list.len() as i64))
}

async fn panel_item_icon_group_edit(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<Value>) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let id = parse_i64(req.get("id"));
    let icon = parse_opt_string(req.get("icon"));
    let title = parse_string(req.get("title"));
    let description = parse_string(req.get("description"));
    let sort = parse_i64(req.get("sort"));
    if id > 0 {
        sqlx::query("UPDATE item_icon_group SET icon = ?, title = ?, description = ?, sort = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND user_id = ?")
            .bind(icon.clone())
            .bind(title.clone())
            .bind(description.clone())
            .bind(sort)
            .bind(id)
            .bind(auth.user.id)
            .execute(&state.db)
            .await
            .map_err(|e| ApiError::db(e.to_string()))?;
        Ok(ok(json!({ "id": id, "icon": icon, "title": title, "description": description, "sort": sort, "userId": auth.user.id })))
    } else {
        let res = sqlx::query("INSERT INTO item_icon_group (icon, title, description, sort, user_id, created_at, updated_at) VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
            .bind(icon.clone())
            .bind(title.clone())
            .bind(description.clone())
            .bind(sort)
            .bind(auth.user.id)
            .execute(&state.db)
            .await
            .map_err(|e| ApiError::db(e.to_string()))?;
        Ok(ok(json!({ "id": res.last_insert_rowid(), "icon": icon, "title": title, "description": description, "sort": sort, "userId": auth.user.id })))
    }
}

async fn panel_item_icon_group_deletes(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<IdsRequest>) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM item_icon_group WHERE user_id = ?")
        .bind(auth.user.id)
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);
    if req.ids.len() as i64 >= count {
        return Err(ApiError::new(1201, "Please keep at least one"));
    }
    for id in req.ids {
        sqlx::query("DELETE FROM item_icon WHERE item_icon_group_id = ? AND user_id = ?")
            .bind(id)
            .bind(auth.user.id)
            .execute(&state.db)
            .await
            .map_err(|e| ApiError::db(e.to_string()))?;
        sqlx::query("DELETE FROM item_icon_group WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(auth.user.id)
            .execute(&state.db)
            .await
            .map_err(|e| ApiError::db(e.to_string()))?;
    }
    Ok(ok_empty())
}

async fn panel_item_icon_group_save_sort(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<Value>) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let sort_items = req.get("sortItems").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    for item in sort_items {
        let id = parse_i64(item.get("id"));
        let sort = parse_i64(item.get("sort"));
        sqlx::query("UPDATE item_icon_group SET sort = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND user_id = ?")
            .bind(sort)
            .bind(id)
            .bind(auth.user.id)
            .execute(&state.db)
            .await
            .map_err(|e| ApiError::db(e.to_string()))?;
    }
    Ok(ok_empty())
}

async fn panel_item_icon_get_list_by_group_id(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<Value>) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::PublicAllowed).await?;
    let group_id = parse_i64(req.get("itemIconGroupId"));
    let rows = sqlx::query("SELECT id, created_at, updated_at, icon_json, title, url, lan_url, description, open_method, sort, item_icon_group_id, lan_only FROM item_icon WHERE item_icon_group_id = ? AND user_id = ? ORDER BY sort ASC, created_at ASC")
        .bind(group_id)
        .bind(auth.user.id)
        .fetch_all(&state.db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;
    let list: Vec<Value> = rows.into_iter().map(|row| {
        let icon_json = row.try_get::<Option<String>, _>("icon_json").unwrap_or(None).unwrap_or_else(|| "{}".into());
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
    }).collect();
    Ok(list_ok(list.clone(), list.len() as i64))
}

async fn panel_item_icon_edit(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<Value>) -> ApiResult {
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
    if group_id == 0 { return Err(ApiError::bad_param("Group is mandatory")); }
    let icon_json = serde_json::to_string(req.get("icon").unwrap_or(&Value::Null)).unwrap_or_else(|_| "{}".into());
    if id > 0 {
        sqlx::query("UPDATE item_icon SET icon_json = ?, title = ?, url = ?, lan_url = ?, description = ?, open_method = ?, sort = ?, item_icon_group_id = ?, lan_only = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND user_id = ?")
            .bind(icon_json.clone()).bind(title.clone()).bind(url.clone()).bind(lan_url.clone()).bind(description.clone())
            .bind(open_method).bind(sort).bind(group_id).bind(lan_only).bind(id).bind(auth.user.id)
            .execute(&state.db).await.map_err(|e| ApiError::db(e.to_string()))?;
        Ok(ok(json!({ "id": id, "title": title, "url": url, "lanUrl": lan_url, "description": description, "openMethod": open_method, "sort": sort, "itemIconGroupId": group_id, "lanOnly": lan_only, "icon": req.get("icon").cloned().unwrap_or(Value::Null) })))
    } else {
        let res = sqlx::query("INSERT INTO item_icon (icon_json, title, url, lan_url, description, open_method, sort, item_icon_group_id, lan_only, user_id, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, 9999, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
            .bind(icon_json).bind(title.clone()).bind(url.clone()).bind(lan_url.clone()).bind(description.clone())
            .bind(open_method).bind(group_id).bind(lan_only).bind(auth.user.id)
            .execute(&state.db).await.map_err(|e| ApiError::db(e.to_string()))?;
        Ok(ok(json!({ "id": res.last_insert_rowid(), "title": title, "url": url, "lanUrl": lan_url, "description": description, "openMethod": open_method, "sort": 9999, "itemIconGroupId": group_id, "lanOnly": lan_only, "icon": req.get("icon").cloned().unwrap_or(Value::Null) })))
    }
}

async fn panel_item_icon_add_multiple(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<Value>) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let items = req.as_array().cloned().unwrap_or_default();
    let mut created = Vec::<Value>::new();
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
        if group_id == 0 { continue; }
        let icon_json = serde_json::to_string(item.get("icon").unwrap_or(&Value::Null)).unwrap_or_else(|_| "{}".into());
        let res = sqlx::query("INSERT INTO item_icon (icon_json, title, url, lan_url, description, open_method, sort, item_icon_group_id, lan_only, user_id, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
            .bind(icon_json).bind(title.clone()).bind(url.clone()).bind(lan_url.clone()).bind(description.clone()).bind(open_method).bind(sort).bind(group_id).bind(lan_only).bind(auth.user.id)
            .execute(&state.db).await.map_err(|e| ApiError::db(e.to_string()))?;
        created.push(json!({ "id": res.last_insert_rowid(), "title": title, "url": url, "lanUrl": lan_url, "description": description, "openMethod": open_method, "sort": sort, "itemIconGroupId": group_id, "lanOnly": lan_only, "icon": item.get("icon").cloned().unwrap_or(Value::Null) }));
    }
    Ok(ok(created))
}

async fn panel_item_icon_deletes(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<IdsRequest>) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    for id in req.ids {
        sqlx::query("DELETE FROM item_icon WHERE id = ? AND user_id = ?")
            .bind(id).bind(auth.user.id)
            .execute(&state.db).await.map_err(|e| ApiError::db(e.to_string()))?;
    }
    Ok(ok_empty())
}

async fn panel_item_icon_save_sort(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<Value>) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let group_id = parse_i64(req.get("itemIconGroupId"));
    let items = req.get("sortItems").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    for item in items {
        let id = parse_i64(item.get("id"));
        let sort = parse_i64(item.get("sort"));
        sqlx::query("UPDATE item_icon SET sort = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND item_icon_group_id = ? AND user_id = ?")
            .bind(sort).bind(id).bind(group_id).bind(auth.user.id)
            .execute(&state.db).await.map_err(|e| ApiError::db(e.to_string()))?;
    }
    Ok(ok_empty())
}

#[derive(Deserialize)]
struct FaviconRequest { url: String }

fn build_favicon_client() -> Result<reqwest::Client, ApiError> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(5))
        .connect_timeout(std::time::Duration::from_secs(3))
        .timeout(std::time::Duration::from_secs(6))
        .build()
        .map_err(|e| ApiError::new(-1, e.to_string()))
}


// Pre-compiled regex patterns for favicon extraction (compiled once)
static ATTR_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"([A-Za-z_:][-A-Za-z0-9_:.]*)\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'=<>`]+))"#)
        .expect("attribute regex must compile")
});

static LINK_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?is)<link\b[^>]*>"#)
        .expect("link regex must compile")
});

fn normalize_attr_value(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string()
}

fn parse_html_attributes(tag: &str) -> HashMap<String, String> {
    let attr_re = Regex::new(r#"([A-Za-z_:][-A-Za-z0-9_:.]*)\s*=\s*(?:\"([^\"]*)\"|'([^']*)'|([^\s\"'=<>`]+))"#)
        .expect("attribute regex must compile");
    let mut attrs = HashMap::new();

    for cap in attr_re.captures_iter(tag) {
        let key = cap.get(1).map(|m| m.as_str().to_ascii_lowercase()).unwrap_or_default();
        let value = cap.get(2)
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
    let link_re = Regex::new(r#"(?is)<link\b[^>]*>"#).expect("link regex must compile");
    let mut icon_candidates: Vec<(i32, Url)> = Vec::new();
    let mut manifest_candidates: Vec<Url> = Vec::new();

    for link_tag in link_re.find_iter(html) {
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
                attrs.get("sizes").map(|s| s.as_str()).unwrap_or(""),
                attrs.get("type").map(|s| s.as_str()).unwrap_or(""),
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

    if let Some(icons) = value.get("icons").and_then(|v| v.as_array()) {
        for icon in icons {
            let Some(src) = icon.get("src").and_then(|v| v.as_str()) else { continue; };
            let Ok(url) = manifest_url.join(src) else { continue; };
            let score = score_icon_candidate(
                "icon",
                icon.get("sizes").and_then(|v| v.as_str()).unwrap_or(""),
                icon.get("type").and_then(|v| v.as_str()).unwrap_or(""),
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
        "INSERT INTO favicon_cache (cache_key, source_url, icon_data_url, created_at, updated_at) VALUES (?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) \
         ON CONFLICT(cache_key) DO UPDATE SET source_url = excluded.source_url, icon_data_url = excluded.icon_data_url, updated_at = CURRENT_TIMESTAMP",
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
    let resp = match client
        .get(url.clone())
        .header(reqwest::header::USER_AGENT, "YT-Panel/1.0")
        .header(reqwest::header::ACCEPT, "text/html,application/xhtml+xml")
        .send()
        .await {
        Ok(resp) => resp,
        Err(_) => return Ok(None),
    };

    if !resp.status().is_success() {
        return Ok(None);
    }

    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();

    if !content_type.contains("text/html") && !content_type.contains("application/xhtml") {
        return Ok(None);
    }

    let text = resp.text().await.map_err(|e| ApiError::new(-1, e.to_string()))?;
    if text.trim().is_empty() {
        return Ok(None);
    }

    Ok(Some(text))
}

async fn fetch_manifest_document(client: &reqwest::Client, url: &Url) -> Result<Option<String>, ApiError> {
    let resp = match client
        .get(url.clone())
        .header(reqwest::header::USER_AGENT, "YT-Panel/1.0")
        .header(reqwest::header::ACCEPT, "application/manifest+json,application/json,text/plain")
        .send()
        .await {
        Ok(resp) => resp,
        Err(_) => return Ok(None),
    };

    if !resp.status().is_success() {
        return Ok(None);
    }

    let text = resp.text().await.map_err(|e| ApiError::new(-1, e.to_string()))?;
    if text.trim().is_empty() {
        return Ok(None);
    }

    Ok(Some(text))
}

async fn fetch_favicon_data_url(client: &reqwest::Client, url: &Url) -> Result<Option<String>, ApiError> {
    let resp = match client
        .get(url.clone())
        .header(reqwest::header::USER_AGENT, "YT-Panel/1.0")
        .header(reqwest::header::ACCEPT, "image/*,*/*;q=0.8")
        .send()
        .await {
        Ok(resp) => resp,
        Err(_) => return Ok(None),
    };

    if !resp.status().is_success() {
        return Ok(None);
    }

    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
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

    let bytes = resp.bytes().await.map_err(|e| ApiError::new(-1, e.to_string()))?;
    if bytes.is_empty() {
        return Ok(None);
    }

    Ok(Some(format!("data:{};base64,{}", mime, B64.encode(bytes))))
}

async fn resolve_site_favicon_data_url(url: &str) -> Result<String, ApiError> {
    let parsed = Url::parse(url).map_err(|_| ApiError::new(-1, "invalid or unsafe URL"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(ApiError::new(-1, "invalid or unsafe URL"));
    }

    let host = parsed.host_str().ok_or_else(|| ApiError::new(-1, "invalid or unsafe URL"))?;
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
            if seen.insert(icon_url.to_string()) {
                candidates.push(icon_url);
            }
        }

        for manifest_url in manifest_urls {
            if let Some(manifest_text) = fetch_manifest_document(&client, &manifest_url).await? {
                for icon_url in extract_manifest_icon_candidates(&manifest_url, &manifest_text) {
                    if seen.insert(icon_url.to_string()) {
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
            if seen.insert(url.to_string()) {
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
                if seen.insert(url.to_string()) {
                    candidates.push(url);
                }
            }
        }
    }

    if let Ok(google_s2) = Url::parse(&format!("https://www.google.com/s2/favicons?domain={}&sz=64", host)) {
        if seen.insert(google_s2.to_string()) {
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

async fn panel_item_icon_get_site_favicon(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<FaviconRequest>) -> ApiResult {
    let _auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let icon_url = resolve_site_favicon_with_cache(&state, &req.url).await?;
    Ok(ok(json!({ "iconUrl": icon_url })))
}

async fn panel_bookmark_get_list(State(state): State<AppState>, headers: HeaderMap) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::PublicAllowed).await?;
    let rows = sqlx::query("SELECT id, created_at, icon_json, title, url, lan_url, sort, is_folder, parent_url, parent_id FROM bookmark WHERE user_id = ? ORDER BY sort ASC, created_at ASC")
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
            title: row.try_get::<Option<String>, _>("title").unwrap_or(None).unwrap_or_default(),
            url: row.try_get::<Option<String>, _>("url").unwrap_or(None).unwrap_or_default(),
            lan_url: row.try_get::<Option<String>, _>("lan_url").unwrap_or(None),
            sort: row.try_get::<Option<i64>, _>("sort").unwrap_or(Some(0)).unwrap_or(0),
            is_folder: row.try_get::<Option<i64>, _>("is_folder").unwrap_or(Some(0)).unwrap_or(0),
            parent_url: row.try_get::<Option<String>, _>("parent_url").unwrap_or(None),
            parent_id: row.try_get::<Option<i64>, _>("parent_id").unwrap_or(Some(0)).unwrap_or(0),
            children: Vec::new(),
        };
        grouped.entry(node.parent_id).or_default().push(node);
    }
    fn build_tree(parent_id: i64, grouped: &HashMap<i64, Vec<BookmarkNode>>) -> Vec<BookmarkNode> {
        let mut nodes = grouped.get(&parent_id).cloned().unwrap_or_default();
        nodes.sort_by_key(|n| (n.sort, n.title.clone()));
        for node in &mut nodes {
            node.children = build_tree(node.id, grouped);
        }
        nodes
    }
    let tree = build_tree(0, &grouped);
    Ok(list_ok(tree.clone(), tree.len() as i64))
}

async fn panel_bookmark_add(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<Value>) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let title = parse_string(req.get("title"));
    let url = parse_string(req.get("url"));
    let lan_url = parse_opt_string(req.get("lanUrl"));
    let parent_url = parse_opt_string(req.get("parentUrl"));
    let parent_id = parse_i64(req.get("parentId"));
    let is_folder = parse_i64(req.get("isFolder"));
    let icon_json = req.get("iconJson").and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_default();
    let max_sort: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(sort), 0) FROM bookmark WHERE user_id = ? AND parent_id = ?")
        .bind(auth.user.id)
        .bind(parent_id)
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);
    let res = sqlx::query("INSERT INTO bookmark (title, url, lan_url, sort, is_folder, parent_url, parent_id, icon_json, user_id, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)")
        .bind(title.clone()).bind(url.clone()).bind(lan_url.clone()).bind(max_sort + 1).bind(is_folder).bind(parent_url.clone()).bind(parent_id).bind(icon_json.clone()).bind(auth.user.id)
        .execute(&state.db).await.map_err(|e| ApiError::db(e.to_string()))?;
    Ok(ok(json!({ "id": res.last_insert_rowid(), "title": title, "url": url, "lanUrl": lan_url, "sort": max_sort + 1, "isFolder": is_folder, "parentUrl": parent_url, "parentId": parent_id, "iconJson": icon_json })))
}

async fn panel_bookmark_add_multiple(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<Value>) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let items = if let Some(arr) = req.as_array() {
        arr.clone()
    } else if let Some(arr) = req.get("Bookmarks").and_then(|v| v.as_array()) {
        arr.clone()
    } else if let Some(arr) = req.get("bookmarks").and_then(|v| v.as_array()) {
        arr.clone()
    } else {
        Vec::new()
    };
    let mut inserted = Vec::new();
    let mut temp_id_map = std::collections::HashMap::new();
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
            if icon.is_string() { icon.as_str().unwrap_or_default().to_string() } else { serde_json::to_string(icon).unwrap_or_default() }
        } else { String::new() };
        let res = sqlx::query("INSERT INTO bookmark (title, url, lan_url, sort, is_folder, parent_url, parent_id, icon_json, user_id, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)")
            .bind(title.clone()).bind(url.clone()).bind(lan_url.clone()).bind(sort).bind(is_folder).bind(parent_url.clone()).bind(parent_id).bind(icon_json.clone()).bind(auth.user.id)
            .execute(&state.db).await.map_err(|e| ApiError::db(e.to_string()))?;
        let new_id = res.last_insert_rowid();
        let temp_id = parse_i64(item.get("tempId"));
        if temp_id > 0 {
            temp_id_map.insert(temp_id, new_id);
        }
        inserted.push(json!({ "id": new_id, "title": title, "url": url, "lanUrl": lan_url, "sort": sort, "isFolder": is_folder, "parentUrl": parent_url, "parentId": parent_id, "iconJson": icon_json }));
    }
    Ok(ok(json!({ "count": inserted.len(), "list": inserted })))
}

async fn panel_bookmark_update(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<Value>) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let id = parse_i64(req.get("id"));
    let title = parse_string(req.get("title"));
    let url = parse_string(req.get("url"));
    let lan_url = parse_opt_string(req.get("lanUrl"));
    let parent_url = parse_opt_string(req.get("parentUrl"));
    let parent_id = parse_i64(req.get("parentId"));
    let sort = parse_i64(req.get("sort"));
    let icon_json = req.get("iconJson").and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_default();
    sqlx::query("UPDATE bookmark SET title = ?, url = ?, lan_url = ?, parent_url = ?, parent_id = ?, sort = ?, icon_json = ? WHERE id = ? AND user_id = ?")
        .bind(title.clone()).bind(url.clone()).bind(lan_url.clone()).bind(parent_url.clone()).bind(parent_id).bind(sort).bind(icon_json.clone()).bind(id).bind(auth.user.id)
        .execute(&state.db).await.map_err(|e| ApiError::db(e.to_string()))?;
    Ok(ok(json!({ "id": id, "title": title, "url": url, "lanUrl": lan_url, "parentUrl": parent_url, "parentId": parent_id, "sort": sort, "iconJson": icon_json })))
}

async fn panel_bookmark_deletes(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<IdsRequest>) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let mut to_delete = req.ids.clone();
    let mut idx = 0usize;
    while idx < to_delete.len() {
        let current = to_delete[idx];
        let child_rows = sqlx::query("SELECT id FROM bookmark WHERE user_id = ? AND parent_id = ?")
            .bind(auth.user.id).bind(current)
            .fetch_all(&state.db).await.map_err(|e| ApiError::db(e.to_string()))?;
        for row in child_rows {
            let child_id = row.get::<i64, _>("id");
            if !to_delete.contains(&child_id) { to_delete.push(child_id); }
        }
        idx += 1;
    }
    for id in to_delete {
        sqlx::query("DELETE FROM bookmark WHERE id = ? AND user_id = ?")
            .bind(id).bind(auth.user.id)
            .execute(&state.db).await.map_err(|e| ApiError::db(e.to_string()))?;
    }
    Ok(ok_empty())
}

#[derive(Deserialize)]
struct NotepadQuery { id: Option<i64> }

async fn panel_notepad_get(State(state): State<AppState>, headers: HeaderMap, Query(query): Query<NotepadQuery>) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let row = if let Some(id) = query.id {
        sqlx::query("SELECT id, user_id, title, content, created_at, updated_at FROM notepad WHERE user_id = ? AND id = ? LIMIT 1")
            .bind(auth.user.id).bind(id)
            .fetch_optional(&state.db).await.map_err(|e| ApiError::db(e.to_string()))?
    } else {
        sqlx::query("SELECT id, user_id, title, content, created_at, updated_at FROM notepad WHERE user_id = ? ORDER BY updated_at DESC LIMIT 1")
            .bind(auth.user.id)
            .fetch_optional(&state.db).await.map_err(|e| ApiError::db(e.to_string()))?
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

async fn panel_notepad_get_list(State(state): State<AppState>, headers: HeaderMap) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let rows = sqlx::query("SELECT id, user_id, title, content, created_at, updated_at FROM notepad WHERE user_id = ? ORDER BY updated_at DESC")
        .bind(auth.user.id)
        .fetch_all(&state.db).await.map_err(|e| ApiError::db(e.to_string()))?;
    let list: Vec<Value> = rows.into_iter().map(|row| json!({
        "id": row.get::<i64, _>("id"),
        "userId": row.get::<i64, _>("user_id"),
        "title": row.try_get::<Option<String>, _>("title").unwrap_or(None),
        "content": row.try_get::<Option<String>, _>("content").unwrap_or(None),
        "createdAt": row.try_get::<Option<String>, _>("created_at").unwrap_or(None),
        "updatedAt": row.try_get::<Option<String>, _>("updated_at").unwrap_or(None),
    })).collect();
    Ok(ok(list))
}

async fn panel_notepad_save(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<Value>) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let id = parse_i64(req.get("id"));
    let title = parse_string(req.get("title"));
    let content = parse_string(req.get("content"));
    if id > 0 {
        sqlx::query("UPDATE notepad SET title = ?, content = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND user_id = ?")
            .bind(title.clone()).bind(content.clone()).bind(id).bind(auth.user.id)
            .execute(&state.db).await.map_err(|e| ApiError::db(e.to_string()))?;
        Ok(ok(json!({ "id": id, "userId": auth.user.id, "title": title, "content": content })))
    } else {
        let res = sqlx::query("INSERT INTO notepad (user_id, title, content, created_at, updated_at) VALUES (?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
            .bind(auth.user.id).bind(title.clone()).bind(content.clone())
            .execute(&state.db).await.map_err(|e| ApiError::db(e.to_string()))?;
        Ok(ok(json!({ "id": res.last_insert_rowid(), "userId": auth.user.id, "title": title, "content": content })))
    }
}

async fn panel_notepad_delete(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<Value>) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let id = parse_i64(req.get("id"));
    sqlx::query("DELETE FROM notepad WHERE id = ? AND user_id = ?")
        .bind(id).bind(auth.user.id)
        .execute(&state.db).await.map_err(|e| ApiError::db(e.to_string()))?;
    Ok(ok_empty())
}

async fn panel_notepad_upload(State(state): State<AppState>, headers: HeaderMap, mut multipart: Multipart) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    if let Some(field) = multipart.next_field().await.map_err(|e| ApiError::new(1300, e.to_string()))? {
        let file_name = field.file_name().unwrap_or("notepad.bin").to_string();
        let ext = Path::new(&file_name).extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
        let allow = ["png", "jpg", "gif", "jpeg", "webp", "ico", "txt", "md", "json", "pdf", "doc", "docx", "xls", "xlsx"];
        if !allow.contains(&ext.as_str()) { return Err(ApiError::new(-1, "file type not allowed")); }
        let (relative_db_path, public_url, ext) = save_upload_field(&state, auth.user.id, field, Some("notepad")).await?;
        sqlx::query("INSERT INTO file (src, user_id, file_name, method, ext, created_at, updated_at) VALUES (?, ?, ?, 0, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
            .bind(relative_db_path)
            .bind(auth.user.id)
            .bind(file_name.clone())
            .bind(ext)
            .execute(&state.db)
            .await
            .map_err(|e| ApiError::db(e.to_string()))?;
        return Ok(ok(json!({ "url": public_url, "name": file_name, "type": MimeGuess::from_path(&file_name).first_or_octet_stream().to_string() })));
    }
    Err(ApiError::new(1300, "Upload failed"))
}

async fn panel_search_engine_get_list(State(state): State<AppState>, headers: HeaderMap) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::PublicAllowed).await?;
    let rows = sqlx::query("SELECT id, icon_src, title, url, sort, user_id, created_at, updated_at FROM search_engine WHERE user_id = ? AND deleted_at IS NULL ORDER BY sort ASC")
        .bind(auth.user.id)
        .fetch_all(&state.db).await.map_err(|e| ApiError::db(e.to_string()))?;
    let list: Vec<Value> = rows.into_iter().map(|row| json!({
        "id": row.get::<i64, _>("id"),
        "iconSrc": row.try_get::<Option<String>, _>("icon_src").unwrap_or(None),
        "title": row.try_get::<Option<String>, _>("title").unwrap_or(None),
        "url": row.try_get::<Option<String>, _>("url").unwrap_or(None),
        "sort": row.try_get::<Option<i64>, _>("sort").unwrap_or(Some(0)).unwrap_or(0),
        "userId": row.try_get::<Option<i64>, _>("user_id").unwrap_or(None),
        "createTime": row.try_get::<Option<String>, _>("created_at").unwrap_or(None),
        "updateTime": row.try_get::<Option<String>, _>("updated_at").unwrap_or(None),
    })).collect();
    Ok(list_ok(list.clone(), list.len() as i64))
}

async fn panel_search_engine_add(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<Value>) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let title = parse_string(req.get("title"));
    let url = parse_string(req.get("url"));
    let icon_src = parse_string(req.get("iconSrc"));
    let max_sort: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(sort), 0) FROM search_engine WHERE user_id = ? AND deleted_at IS NULL")
        .bind(auth.user.id).fetch_one(&state.db).await.unwrap_or(0);
    let res = sqlx::query("INSERT INTO search_engine (icon_src, title, url, sort, user_id, created_at, updated_at) VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
        .bind(icon_src.clone()).bind(title.clone()).bind(url.clone()).bind(max_sort + 1).bind(auth.user.id)
        .execute(&state.db).await.map_err(|e| ApiError::db(e.to_string()))?;
    Ok(ok(json!({ "id": res.last_insert_rowid(), "iconSrc": icon_src, "title": title, "url": url, "sort": max_sort + 1, "userId": auth.user.id })))
}

async fn panel_search_engine_update(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<Value>) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let id = parse_i64(req.get("id"));
    let title = parse_string(req.get("title"));
    let url = parse_string(req.get("url"));
    let icon_src = parse_string(req.get("iconSrc"));
    let sort = parse_i64(req.get("sort"));
    sqlx::query("UPDATE search_engine SET icon_src = ?, title = ?, url = ?, sort = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND user_id = ?")
        .bind(icon_src.clone()).bind(title.clone()).bind(url.clone()).bind(sort).bind(id).bind(auth.user.id)
        .execute(&state.db).await.map_err(|e| ApiError::db(e.to_string()))?;
    Ok(ok(json!({ "id": id, "iconSrc": icon_src, "title": title, "url": url, "sort": sort })))
}

async fn panel_search_engine_delete(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<Value>) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let id = parse_i64(req.get("id"));
    sqlx::query("DELETE FROM search_engine WHERE id = ? AND user_id = ?")
        .bind(id).bind(auth.user.id)
        .execute(&state.db).await.map_err(|e| ApiError::db(e.to_string()))?;
    Ok(ok_empty())
}

async fn panel_search_engine_update_sort(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<Value>) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let items = req.get("items").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    for item in items {
        let id = parse_i64(item.get("id"));
        let sort = parse_i64(item.get("sort"));
        sqlx::query("UPDATE search_engine SET sort = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND user_id = ?")
            .bind(sort).bind(id).bind(auth.user.id)
            .execute(&state.db).await.map_err(|e| ApiError::db(e.to_string()))?;
    }
    Ok(ok_empty())
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
    let used_percent = if total == 0 { 0.0 } else { (used as f64 / total as f64) * 100.0 };
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
            let used_percent = if total == 0 { 0.0 } else { (used as f64 / total as f64) * 100.0 };
            best = Some(json!({
                "mountpoint": mount,
                "total": total,
                "used": used,
                "free": free,
                "usedPercent": used_percent,
            }));
        }
    }
    best.unwrap_or_else(|| json!({
        "mountpoint": wanted,
        "total": 0,
        "used": 0,
        "free": 0,
        "usedPercent": 0,
    }))
}

fn build_monitor_payload(path: Option<String>) -> Value {
    json!({
        "cpuInfo": build_cpu_payload(),
        "diskInfo": [build_disk_payload(path)],
        "netIOCountersInfo": [],
        "memoryInfo": build_memory_payload(),
    })
}

fn uploads_public_prefix(subdir: Option<&str>) -> String {
    let mut parts = vec!["uploads".to_string()];
    if let Some(subdir) = subdir.filter(|value| !value.is_empty()) {
        parts.push(subdir.to_string());
    }
    parts.join("/")
}

fn resolve_uploaded_file_path(config: &AppConfig, stored_src: &str) -> Option<PathBuf> {
    let normalized = stored_src.trim();
    if normalized.is_empty() {
        return None;
    }

    let direct = Path::new(normalized);
    if direct.is_absolute() {
        return Some(direct.to_path_buf());
    }

    for prefix in ["./uploads/", "/uploads/", "uploads/"] {
        if let Some(suffix) = normalized.strip_prefix(prefix) {
            return Some(Path::new(&config.uploads_dir).join(suffix));
        }
    }

    let trimmed = normalized.trim_start_matches("./");
    if !trimmed.is_empty() {
        let as_absolute = Path::new("/").join(trimmed);
        if as_absolute.is_absolute() && as_absolute.components().next() == Some(Component::RootDir) {
            return Some(as_absolute);
        }

        let as_relative = PathBuf::from(trimmed);
        if as_relative.exists() {
            return Some(as_relative);
        }
    }

    None
}

async fn save_upload_field(
    state: &AppState,
    _user_id: i64,
    field: axum::extract::multipart::Field<'_>,
    subdir: Option<&str>,
) -> Result<(String, String, String), ApiError> {
    let file_name = field.file_name().unwrap_or("upload.bin").to_string();
    let bytes = field.bytes().await.map_err(|e| ApiError::new(1300, e.to_string()))?;
    let max = state.config.max_upload_mb * 1024 * 1024;
    if bytes.len() as u64 > max { return Err(ApiError::new(1300, "file too large")); }
    let ext = Path::new(&file_name).extension().and_then(|s| s.to_str()).unwrap_or("bin").to_lowercase();
    let now = Utc::now();
    let hash_input = format!("{}-{}-{}", file_name, now.timestamp_millis(), random_token(6));
    let safe_name = format!("{:x}", md5::compute(hash_input));
    let public_prefix = uploads_public_prefix(subdir);
    let relative_dir = format!("{}/{}/{}/{}", public_prefix, now.year(), now.month(), now.day());
    let absolute_dir = Path::new(&state.config.uploads_dir)
        .join(subdir.unwrap_or(""))
        .join(now.year().to_string())
        .join(now.month().to_string())
        .join(now.day().to_string());
    fs::create_dir_all(&absolute_dir).await.map_err(|e| ApiError::new(1300, e.to_string()))?;
    let absolute_path = absolute_dir.join(format!("{}.{}", safe_name, ext));
    let mut file = fs::File::create(&absolute_path).await.map_err(|e| ApiError::new(1300, e.to_string()))?;
    file.write_all(&bytes).await.map_err(|e| ApiError::new(1300, e.to_string()))?;
    let relative_db_path = format!("./{}/{}.{}", relative_dir, safe_name, ext);
    let public_url = format!("/{}/{}.{}", relative_dir, safe_name, ext);
    Ok((relative_db_path, public_url, ext))
}

fn extract_client_ip(headers: &HeaderMap) -> Option<String> {
    ["cf-connecting-ip", "x-forwarded-for", "x-real-ip", "x-original-forwarded-for"]
        .iter()
        .find_map(|key| headers.get(*key).and_then(|v| v.to_str().ok()).map(|v| v.split(',').next().unwrap_or(v).trim().to_string()))
        .or_else(|| {
            headers
                .get("forwarded")
                .and_then(|v| v.to_str().ok())
                .and_then(|value| value.split(';').find(|part| part.trim_start().starts_with("for=")))
                .map(|part| part.trim().trim_start_matches("for=").trim_matches('"').trim_matches('[').trim_matches(']').to_string())
        })
}

fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_private() || v4.is_loopback() || v4.is_link_local(),
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unique_local() || v6.is_unicast_link_local(),
    }
}

#[allow(dead_code)]
fn strip_html_image_refs(content: &str) -> Vec<String> {
    let img_regex = Regex::new(r#"<img[^>]+src=\"([^\"]+)\""#).unwrap();
    let file_regex = Regex::new(r#"<a[^>]+href=\"([^\"]+)\""#).unwrap();
    img_regex
        .captures_iter(content)
        .chain(file_regex.captures_iter(content))
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect()
}
