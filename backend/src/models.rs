use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ============== 用户相关 ==============
#[derive(Debug, Clone, Deserialize, Serialize, FromRow)]
pub struct User {
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
    pub create_time: Option<String>,
    pub update_time: Option<String>,
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
}

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user: CurrentUser,
    pub visit_mode: i32,
}

#[derive(Debug, Clone, Copy)]
pub enum AccessMode {
    LoginRequired,
    PublicAllowed,
}

// ============== 配置相关 ==============
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub uploads_dir: String,
    pub frontend_dist: String,
    pub max_upload_mb: u64,
    pub public_user_id: Option<i64>,
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

// ============== 书签相关 ==============
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

// ============== 用户配置相关 ==============
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserPayload {
    pub id: i64,
    pub user_id: i64,
    pub username: String,
    pub name: String,
    pub head_image: Option<String>,
    pub status: i64,
    pub role: i64,
    pub mail: Option<String>,
    pub referral_code: Option<String>,
    pub token: Option<String>,
    pub create_time: Option<String>,
    pub update_time: Option<String>,
}

// ============== 文件相关 ==============
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub size: i64,
    pub create_time: String,
}

// ============== 系统监控相关 ==============
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemState {
    pub cpu: CpuState,
    pub memory: MemoryState,
    pub disk: DiskState,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuState {
    pub percent: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryState {
    pub total: u64,
    pub used: u64,
    pub available: u64,
    pub percent: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskState {
    pub total: u64,
    pub used: u64,
    pub free: u64,
    pub percent: f64,
}
