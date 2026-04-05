use crate::error::ApiError;
use sqlx::SqlitePool;

pub async fn init_db(db: &SqlitePool) -> anyhow::Result<()> {
    // User table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS user (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT UNIQUE NOT NULL,
            password TEXT NOT NULL,
            name TEXT NOT NULL,
            head_image TEXT,
            status INTEGER DEFAULT 1,
            role INTEGER DEFAULT 0,
            mail TEXT,
            referral_code TEXT,
            token TEXT,
            create_time TEXT DEFAULT CURRENT_TIMESTAMP,
            update_time TEXT DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(db)
    .await?;

    // Settings table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS settings (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            key TEXT UNIQUE NOT NULL,
            value TEXT,
            create_time TEXT DEFAULT CURRENT_TIMESTAMP,
            update_time TEXT DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(db)
    .await?;

    // Module config table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS module_config (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT UNIQUE NOT NULL,
            value TEXT,
            create_time TEXT DEFAULT CURRENT_TIMESTAMP,
            update_time TEXT DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(db)
    .await?;

    // User config table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS user_config (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            key TEXT NOT NULL,
            value TEXT,
            UNIQUE(user_id, key)
        )
        "#,
    )
    .execute(db)
    .await?;

    // Item icon group table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS item_icon_group (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            name TEXT NOT NULL,
            sort INTEGER DEFAULT 0,
            create_time TEXT DEFAULT CURRENT_TIMESTAMP,
            update_time TEXT DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(db)
    .await?;

    // Item icon table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS item_icon (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            group_id INTEGER NOT NULL,
            user_id INTEGER NOT NULL,
            title TEXT NOT NULL,
            url TEXT NOT NULL,
            lan_url TEXT,
            icon_json TEXT,
            sort INTEGER DEFAULT 0,
            create_time TEXT DEFAULT CURRENT_TIMESTAMP,
            update_time TEXT DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(db)
    .await?;

    // Bookmark table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS bookmark (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            title TEXT NOT NULL,
            url TEXT,
            lan_url TEXT,
            icon_json TEXT,
            parent_id INTEGER DEFAULT 0,
            is_folder INTEGER DEFAULT 0,
            sort INTEGER DEFAULT 0,
            create_time TEXT DEFAULT CURRENT_TIMESTAMP,
            update_time TEXT DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(db)
    .await?;

    // Notepad table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS notepad (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            title TEXT NOT NULL,
            content TEXT,
            sort INTEGER DEFAULT 0,
            create_time TEXT DEFAULT CURRENT_TIMESTAMP,
            update_time TEXT DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(db)
    .await?;

    // Search engine table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS search_engine (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            name TEXT NOT NULL,
            url TEXT NOT NULL,
            icon TEXT,
            sort INTEGER DEFAULT 0,
            create_time TEXT DEFAULT CURRENT_TIMESTAMP,
            update_time TEXT DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(db)
    .await?;

    // Notice table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS notice (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            display_type TEXT NOT NULL,
            content TEXT NOT NULL,
            enabled INTEGER DEFAULT 1,
            create_time TEXT DEFAULT CURRENT_TIMESTAMP,
            update_time TEXT DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(db)
    .await?;

    // Files table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS files (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            file_name TEXT NOT NULL,
            file_path TEXT NOT NULL,
            file_size INTEGER DEFAULT 0,
            create_time TEXT DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(db)
    .await?;

    // Favicon cache table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS favicon_cache (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            url TEXT UNIQUE NOT NULL,
            icon_data TEXT,
            create_time TEXT DEFAULT CURRENT_TIMESTAMP,
            update_time TEXT DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(db)
    .await?;

    Ok(())
}

pub async fn seed_defaults(db: &SqlitePool, config: &crate::config::AppConfig) -> anyhow::Result<()> {
    let _ = config;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user WHERE role = 0")
        .fetch_one(db)
        .await?;

    if count == 0 {
        let hashed = bcrypt::hash("admin", bcrypt::DEFAULT_COST)?;
        sqlx::query(
            "INSERT INTO user (username, password, name, role, status) VALUES (?, ?, ?, 0, 1)",
        )
        .bind("admin")
        .bind(&hashed)
        .bind("Administrator")
        .execute(db)
        .await?;
    }

    let notice_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notice")
        .fetch_one(db)
        .await?;

    if notice_count == 0 {
        sqlx::query(
            "INSERT INTO notice (display_type, content, enabled) VALUES (?, ?, 1)",
        )
        .bind("login")
        .bind("Welcome to YT-Panel")
        .execute(db)
        .await?;
    }

    Ok(())
}

pub async fn get_setting(db: &SqlitePool, key: &str) -> Result<Option<String>, ApiError> {
    let value = sqlx::query_scalar("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;
    Ok(value)
}

pub async fn set_setting(db: &SqlitePool, key: &str, value: &str) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        INSERT INTO settings (key, value, update_time) 
        VALUES (?, ?, CURRENT_TIMESTAMP)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value, update_time = CURRENT_TIMESTAMP
        "#,
    )
    .bind(key)
    .bind(value)
    .execute(db)
    .await
    .map_err(|e| ApiError::db(e.to_string()))?;
    Ok(())
}

pub async fn get_module_config(db: &SqlitePool, name: &str) -> Result<Option<String>, ApiError> {
    let value = sqlx::query_scalar("SELECT value FROM module_config WHERE name = ?")
        .bind(name)
        .fetch_optional(db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;
    Ok(value)
}

pub async fn save_module_config(
    db: &SqlitePool,
    name: &str,
    value: &str,
) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        INSERT INTO module_config (name, value, update_time) 
        VALUES (?, ?, CURRENT_TIMESTAMP)
        ON CONFLICT(name) DO UPDATE SET value = excluded.value, update_time = CURRENT_TIMESTAMP
        "#,
    )
    .bind(name)
    .bind(value)
    .execute(db)
    .await
    .map_err(|e| ApiError::db(e.to_string()))?;
    Ok(())
}

pub async fn get_user_config(db: &SqlitePool, user_id: i64, key: &str) -> Result<Option<String>, ApiError> {
    let value = sqlx::query_scalar("SELECT value FROM user_config WHERE user_id = ? AND key = ?")
        .bind(user_id)
        .bind(key)
        .fetch_optional(db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;
    Ok(value)
}

pub async fn set_user_config(
    db: &SqlitePool,
    user_id: i64,
    key: &str,
    value: &str,
) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        INSERT INTO user_config (user_id, key, value) 
        VALUES (?, ?, ?)
        ON CONFLICT(user_id, key) DO UPDATE SET value = excluded.value
        "#,
    )
    .bind(user_id)
    .bind(key)
    .bind(value)
    .execute(db)
    .await
    .map_err(|e| ApiError::db(e.to_string()))?;
    Ok(())
}

pub async fn get_favicon_cache(db: &SqlitePool, url: &str) -> Result<Option<String>, ApiError> {
    let value = sqlx::query_scalar("SELECT icon_data FROM favicon_cache WHERE url = ?")
        .bind(url)
        .fetch_optional(db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?;
    Ok(value)
}

pub async fn set_favicon_cache(db: &SqlitePool, url: &str, icon_data: &str) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        INSERT INTO favicon_cache (url, icon_data, update_time) 
        VALUES (?, ?, CURRENT_TIMESTAMP)
        ON CONFLICT(url) DO UPDATE SET icon_data = excluded.icon_data, update_time = CURRENT_TIMESTAMP
        "#,
    )
    .bind(url)
    .bind(icon_data)
    .execute(db)
    .await
    .map_err(|e| ApiError::db(e.to_string()))?;
    Ok(())
}
