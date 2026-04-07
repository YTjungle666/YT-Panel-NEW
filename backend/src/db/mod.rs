use bcrypt::hash;
use serde_json::json;
use sqlx::{Row, SqlitePool};
use std::path::PathBuf;
use tokio::fs;

use crate::auth::random_token;
use crate::error::ApiError;
use crate::models::{AppConfig, CurrentUser};

/// 从数据库 URL 提取文件路径
pub fn sqlite_file_path(database_url: &str) -> Option<PathBuf> {
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

/// 确保必要的父目录存在
pub async fn ensure_parent_dirs(config: &AppConfig) -> anyhow::Result<()> {
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

/// 初始化数据库表结构
pub async fn init_db(db: &SqlitePool) -> anyhow::Result<()> {
    let statements = [
        // user 表
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
            token TEXT,
            must_change_password INTEGER DEFAULT 0
        )"#,
        // item_icon_group 表
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
        // item_icon 表
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
        // bookmark 表
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
        // notepad 表
        r#"CREATE TABLE IF NOT EXISTS notepad (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
            user_id INTEGER,
            title TEXT,
            content TEXT
        )"#,
        // search_engine 表
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
        // system_setting 表
        r#"CREATE TABLE IF NOT EXISTS system_setting (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            config_name TEXT UNIQUE,
            config_value TEXT
        )"#,
        // module_config 表
        r#"CREATE TABLE IF NOT EXISTS module_config (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
            user_id INTEGER,
            name TEXT,
            value_json TEXT
        )"#,
        // user_config 表
        r#"CREATE TABLE IF NOT EXISTS user_config (
            user_id INTEGER PRIMARY KEY,
            panel_json TEXT,
            search_engine_json TEXT
        )"#,
        // favicon_cache 表
        r#"CREATE TABLE IF NOT EXISTS favicon_cache (
            cache_key TEXT PRIMARY KEY,
            source_url TEXT,
            icon_data_url TEXT NOT NULL,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP
        )"#,
        // file 表
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
        // notice 表
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
        // 索引
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_user_username ON user(username)",
        "CREATE INDEX IF NOT EXISTS idx_user_mail ON user(mail)",
        "CREATE INDEX IF NOT EXISTS idx_user_token ON user(token)",
        "CREATE INDEX IF NOT EXISTS idx_item_icon_group_user_sort_created ON item_icon_group(user_id, sort, created_at)",
        "CREATE INDEX IF NOT EXISTS idx_item_icon_user_group_sort_created ON item_icon(item_icon_group_id, user_id, sort, created_at)",
        "CREATE INDEX IF NOT EXISTS idx_bookmark_user_parent_sort_created ON bookmark(user_id, parent_id, sort, created_at)",
        "CREATE INDEX IF NOT EXISTS idx_notepad_user_updated ON notepad(user_id, updated_at)",
        "CREATE INDEX IF NOT EXISTS idx_search_engine_user_deleted_sort ON search_engine(user_id, deleted_at, sort)",
        "CREATE INDEX IF NOT EXISTS idx_module_config_user_name ON module_config(user_id, name)",
        "CREATE INDEX IF NOT EXISTS idx_file_user_created ON file(user_id, created_at)",
        "CREATE INDEX IF NOT EXISTS idx_notice_display_type ON notice(display_type)",
    ];

    for sql in statements {
        sqlx::query(sql).execute(db).await?;
    }

    // 兼容旧表结构：添加 must_change_password 列
    sqlx::query("ALTER TABLE user ADD COLUMN must_change_password INTEGER DEFAULT 0")
        .execute(db)
        .await
        .ok();

    // SQLite 性能优化
    sqlx::query("PRAGMA journal_mode=WAL").execute(db).await.ok();
    sqlx::query("PRAGMA synchronous=NORMAL").execute(db).await.ok();
    sqlx::query("PRAGMA busy_timeout=3000").execute(db).await.ok();
    sqlx::query("PRAGMA foreign_keys=ON").execute(db).await.ok();
    sqlx::query("PRAGMA temp_store=MEMORY").execute(db).await.ok();

    Ok(())
}

pub async fn seed_defaults(db: &SqlitePool, config: &AppConfig) -> anyhow::Result<()> {
    sqlx::query("UPDATE user SET token = '' WHERE token != ''")
        .execute(db)
        .await
        .ok();

    let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user")
        .fetch_one(db)
        .await
        .unwrap_or(0);

    if user_count == 0 {
        let password = hash("123456", 12)?;
        let token = random_token(48);
        sqlx::query(
            "INSERT INTO user (username, password, name, status, role, token, must_change_password, created_at, updated_at) VALUES (?, ?, ?, 1, 1, ?, 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
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
    ensure_setting(
        db,
        "security_password_policy",
        json!({ "allowWeakPassword": false }).to_string(),
    )
    .await?;
    ensure_setting(db, "public_crypto_key", random_token(64)).await?;
    if let Some(public_user_id) = config.public_user_id {
        ensure_setting(db, "panel_public_user_id", public_user_id.to_string()).await?;
    }

    Ok(())
}

/// 加载用户数据（通用）
pub async fn load_user_by(
    db: &SqlitePool,
    field: &str,
    value: &str,
) -> Result<Option<CurrentUser>, ApiError> {
    let sql = match field {
        "id" => "SELECT id, username, password, name, head_image, status, role, mail, referral_code, token, must_change_password FROM user WHERE id = ? LIMIT 1",
        "username" => "SELECT id, username, password, name, head_image, status, role, mail, referral_code, token, must_change_password FROM user WHERE username = ? LIMIT 1",
        "mail" => "SELECT id, username, password, name, head_image, status, role, mail, referral_code, token, must_change_password FROM user WHERE mail = ? LIMIT 1",
        "token" => "SELECT id, username, password, name, head_image, status, role, mail, referral_code, token, must_change_password FROM user WHERE token = ? LIMIT 1",
        _ => return Err(ApiError::bad_param("invalid field")),
    };

    let row = sqlx::query(sql)
        .bind(value)
        .fetch_optional(db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;

    Ok(row.map(row_to_user))
}

pub async fn load_user_by_username(
    db: &SqlitePool,
    username: &str,
) -> Result<Option<CurrentUser>, ApiError> {
    load_user_by(db, "username", username).await
}

pub async fn load_user_by_mail(
    db: &SqlitePool,
    mail: &str,
) -> Result<Option<CurrentUser>, ApiError> {
    load_user_by(db, "mail", mail).await
}

pub async fn load_user_by_id(
    db: &SqlitePool,
    id: i64,
) -> Result<Option<CurrentUser>, ApiError> {
    load_user_by(db, "id", &id.to_string()).await
}

pub async fn load_user_by_persistent_token(
    db: &SqlitePool,
    token: &str,
) -> Result<Option<CurrentUser>, ApiError> {
    load_user_by(db, "token", token).await
}

fn row_to_user(row: sqlx::sqlite::SqliteRow) -> CurrentUser {
    CurrentUser {
        id: row.get::<i64, _>("id"),
        username: row.get::<String, _>("username"),
        password: row.get::<String, _>("password"),
        name: row.get::<Option<String>, _>("name").unwrap_or_default(),
        head_image: row.get::<Option<String>, _>("head_image"),
        status: row.get::<i64, _>("status"),
        role: row.get::<i64, _>("role"),
        mail: row.get::<Option<String>, _>("mail"),
        referral_code: row.get::<Option<String>, _>("referral_code"),
        token: row.get::<Option<String>, _>("token"),
        must_change_password: row.get::<Option<i64>, _>("must_change_password").unwrap_or(0),
    }
}

/// 设置配置项
pub async fn set_setting(db: &SqlitePool, key: &str, value: &str) -> Result<(), ApiError> {
    sqlx::query(
        "INSERT INTO system_setting (config_name, config_value) VALUES (?, ?)
         ON CONFLICT(config_name) DO UPDATE SET config_value = excluded.config_value"
    )
    .bind(key)
    .bind(value)
    .execute(db)
    .await
    .map_err(|e| ApiError::db(e.to_string()))?;
    Ok(())
}

/// 获取配置项
pub async fn get_setting(db: &SqlitePool, key: &str) -> Result<Option<String>, ApiError> {
    let row = sqlx::query("SELECT config_value FROM system_setting WHERE config_name = ?")
        .bind(key)
        .fetch_optional(db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;
    Ok(row.map(|r| r.get::<String, _>("config_value")))
}

pub fn parse_public_user_id_setting(raw: &str) -> Option<i64> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("null") {
        return None;
    }

    trimmed.parse::<i64>().ok().or_else(|| {
        serde_json::from_str::<serde_json::Value>(trimmed)
            .ok()
            .and_then(|value| match value {
                serde_json::Value::Null => None,
                serde_json::Value::Number(number) => number.as_i64(),
                serde_json::Value::String(value) => value.parse::<i64>().ok(),
                _ => None,
            })
    })
}

async fn ensure_setting(db: &SqlitePool, key: &str, value: String) -> anyhow::Result<()> {
    let existing: Option<i64> =
        sqlx::query_scalar("SELECT id FROM system_setting WHERE config_name = ?")
            .bind(key)
            .fetch_optional(db)
            .await?;
    if existing.is_none() {
        sqlx::query("INSERT INTO system_setting (config_name, config_value) VALUES (?, ?)")
            .bind(key)
            .bind(value)
            .execute(db)
            .await?;
    }
    Ok(())
}

fn default_system_application_value() -> serde_json::Value {
    json!({
        "loginCaptcha": false,
        "register": {
            "openRegister": false,
            "emailSuffix": "",
        },
    })
}
