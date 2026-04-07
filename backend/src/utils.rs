use serde_json::Value;
use std::net::IpAddr;
use std::path::{Component, Path, PathBuf};
use tokio::{fs, io::AsyncWriteExt};

use chrono::{Datelike, Utc};

use crate::{
    auth::random_token,
    error::ApiError,
    models::{AppConfig, AppState},
};

/// 从 Option<Value> 解析 i64
pub fn parse_i64(input: Option<&Value>) -> i64 {
    input
        .and_then(|v| v.as_i64())
        .or_else(|| input.and_then(|v| v.as_str()).and_then(|s| s.parse().ok()))
        .unwrap_or(0)
}

/// 从 Option<Value> 解析 String
pub fn parse_string(input: Option<&Value>) -> String {
    input
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_default()
}

/// 从 Option<Value> 解析 Option<String>
pub fn parse_opt_string(input: Option<&Value>) -> Option<String> {
    input.and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// 检查是否为私有 IP
pub fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_private() || v4.is_loopback() || v4.is_link_local(),
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unique_local() || v6.is_unicast_link_local(),
    }
}

/// 从请求头提取客户端 IP
pub fn extract_client_ip(headers: &axum::http::HeaderMap) -> Option<String> {
    let keys = [
        "cf-connecting-ip",
        "x-forwarded-for",
        "x-real-ip",
        "x-original-forwarded-for",
    ];

    for key in keys {
        if let Some(value) = headers.get(key).and_then(|v| v.to_str().ok()) {
            return Some(value.split(',').next().unwrap_or(value).trim().to_string());
        }
    }

    // RFC 7239 Forwarded
    if let Some(value) = headers.get("forwarded").and_then(|v| v.to_str().ok()) {
        if let Some(part) = value.split(';').find(|p| p.trim_start().starts_with("for=")) {
            return Some(
                part.trim()
                    .trim_start_matches("for=")
                    .trim_matches('"')
                    .trim_matches('[')
                    .trim_matches(']')
                    .to_string(),
            );
        }
    }

    None
}

/// 上传路径前缀
pub fn uploads_public_prefix(subdir: Option<&str>) -> String {
    let mut parts = vec!["uploads".to_string()];
    if let Some(subdir) = subdir.filter(|v| !v.is_empty()) {
        parts.push(subdir.to_string());
    }
    parts.join("/")
}

/// 解析存储的文件路径为绝对路径
pub fn resolve_uploaded_file_path(uploads_dir: &str, stored_src: &str) -> Option<PathBuf> {
    let normalized = stored_src.trim();
    if normalized.is_empty() {
        return None;
    }

    let suffix = ["./uploads/", "/uploads/", "uploads/"]
        .iter()
        .find_map(|prefix| normalized.strip_prefix(prefix))?;
    let relative = Path::new(suffix);
    if relative.components().any(|component| {
        matches!(
            component,
            Component::RootDir | Component::ParentDir | Component::Prefix(_)
        )
    }) {
        return None;
    }

    Some(Path::new(uploads_dir).join(relative))
}

pub fn max_upload_bytes(config: &AppConfig) -> u64 {
    config.max_upload_mb.saturating_mul(1024 * 1024)
}

pub async fn save_upload_field(
    state: &AppState,
    _user_id: i64,
    field: axum::extract::multipart::Field<'_>,
    subdir: Option<&str>,
) -> Result<(String, String, String), ApiError> {
    let file_name = field.file_name().unwrap_or("upload.bin").to_string();
    let bytes = field
        .bytes()
        .await
        .map_err(|e| ApiError::new(1300, e.to_string()))?;
    let max = max_upload_bytes(&state.config);
    if bytes.len() as u64 > max {
        return Err(ApiError::new(
            1300,
            format!("file too large (max {}MB)", state.config.max_upload_mb),
        ));
    }

    let ext = Path::new(&file_name)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("bin")
        .to_lowercase();
    let now = Utc::now();
    let hash_input = format!(
        "{}-{}-{}",
        file_name,
        now.timestamp_millis(),
        random_token(6)
    );
    let safe_name = format!("{:x}", md5::compute(hash_input));
    let public_prefix = uploads_public_prefix(subdir);
    let relative_dir = format!(
        "{}/{}/{}/{}",
        public_prefix,
        now.year(),
        now.month(),
        now.day()
    );
    let absolute_dir = Path::new(&state.config.uploads_dir)
        .join(subdir.unwrap_or(""))
        .join(now.year().to_string())
        .join(now.month().to_string())
        .join(now.day().to_string());
    fs::create_dir_all(&absolute_dir)
        .await
        .map_err(|e| ApiError::new(1300, e.to_string()))?;
    let absolute_path = absolute_dir.join(format!("{}.{}", safe_name, ext));
    let mut file = fs::File::create(&absolute_path)
        .await
        .map_err(|e| ApiError::new(1300, e.to_string()))?;
    file.write_all(&bytes)
        .await
        .map_err(|e| ApiError::new(1300, e.to_string()))?;
    let relative_db_path = format!("./{}/{}.{}", relative_dir, safe_name, ext);
    let public_url = format!("/{}/{}.{}", relative_dir, safe_name, ext);
    Ok((relative_db_path, public_url, ext))
}
