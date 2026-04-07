use std::{collections::HashMap, sync::Arc, time::Instant};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub uploads_dir: String,
    pub frontend_dist: String,
    pub max_upload_mb: u64,
    pub public_user_id: Option<i64>,
    pub crypto_key: Option<String>,
    #[serde(default)]
    pub cors_allowed_origins: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".into(),
            port: 80,
            database_url: "sqlite://./database/database.db".into(),
            uploads_dir: "./uploads".into(),
            frontend_dist: "../frontend-dist".into(),
            max_upload_mb: 20,
            public_user_id: Some(1),
            crypto_key: None,
            cors_allowed_origins: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub id: i64,
    pub username: String,
    pub password: String,
    pub name: String,
    pub head_image: Option<String>,
    pub status: i64,
    pub role: i64,
    pub mail: Option<String>,
    pub referral_code: Option<String>,
    pub token: Option<String>,
    pub must_change_password: i64,
}

#[derive(Debug, Clone)]
pub struct AuthCacheEntry {
    pub user: CurrentUser,
    pub expires_at: Instant,
}

#[derive(Debug, Clone, Copy)]
pub enum AccessMode {
    LoginRequired,
    PublicAllowed,
}

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user: CurrentUser,
    pub visit_mode: i32,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BookmarkNode {
    pub id: i64,
    pub create_time: Option<String>,
    pub icon_json: Option<String>,
    pub title: String,
    pub url: String,
    pub lan_url: Option<String>,
    pub sort: i64,
    pub is_folder: i64,
    pub parent_url: Option<String>,
    pub parent_id: i64,
    pub children: Vec<BookmarkNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RegisterConfig {
    pub open_register: bool,
    #[serde(default)]
    pub email_suffix: String,
}

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub config: Arc<AppConfig>,
    pub auth_cache: Arc<RwLock<HashMap<String, AuthCacheEntry>>>,
}

pub fn build_user_payload(
    id: i64,
    username: String,
    name: String,
    head_image: Option<String>,
    status: i64,
    role: i64,
    mail: Option<String>,
    referral_code: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
    must_change_password: i64,
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
        "createTime": created_at,
        "updateTime": updated_at,
        "mustChangePassword": must_change_password == 1,
    })
}

pub fn row_to_user_payload(row: SqliteRow) -> Value {
    build_user_payload(
        row.get::<i64, _>("id"),
        row.get::<String, _>("username"),
        row.try_get::<Option<String>, _>("name")
            .unwrap_or(None)
            .unwrap_or_default(),
        row.try_get("head_image").unwrap_or(None),
        row.try_get::<Option<i64>, _>("status")
            .unwrap_or(Some(1))
            .unwrap_or(1),
        row.try_get::<Option<i64>, _>("role")
            .unwrap_or(Some(2))
            .unwrap_or(2),
        row.try_get("mail").unwrap_or(None),
        row.try_get("referral_code").unwrap_or(None),
        row.try_get::<Option<String>, _>("created_at").unwrap_or(None),
        row.try_get::<Option<String>, _>("updated_at").unwrap_or(None),
        row.try_get::<Option<i64>, _>("must_change_password")
            .unwrap_or(Some(0))
            .unwrap_or(0),
    )
}

pub fn default_system_application_value() -> Value {
    json!({
        "loginCaptcha": false,
        "register": {
            "openRegister": false,
            "emailSuffix": "",
        },
    })
}

pub fn parse_register_config(input: Option<&Value>) -> RegisterConfig {
    match input {
        Some(Value::Bool(enabled)) => RegisterConfig {
            open_register: *enabled,
            email_suffix: String::new(),
        },
        Some(Value::Object(_)) => {
            serde_json::from_value(input.cloned().unwrap_or(Value::Null)).unwrap_or_default()
        }
        _ => RegisterConfig::default(),
    }
}
