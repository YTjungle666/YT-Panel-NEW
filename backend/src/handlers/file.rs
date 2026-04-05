//! File handlers - 安全文件上传管理

use axum::{
    extract::{Multipart, State},
    http::HeaderMap,
    routing::post,
    Json, Router,
};
use chrono::Utc;
use serde_json::{json, Value};
use std::path::Path;

use crate::{
    AppState, ApiResult, ApiError,
    authenticate, AccessMode,
    ok, list_ok, ok_empty,
    random_token,
};

/// 允许的文件类型
const ALLOWED_TYPES: &[(&str, &[&str])] = &[
    ("image/jpeg", &["jpg", "jpeg"]),
    ("image/png", &["png"]),
    ("image/gif", &["gif"]),
    ("image/webp", &["webp"]),
];

const MAX_UPLOAD_SIZE: usize = 10 * 1024 * 1024; // 10MB

/// 生成安全文件名
fn generate_safe_filename(user_id: i64, ext: &str) -> String {
    let timestamp = Utc::now().timestamp_millis();
    let random = random_token(8);
    format!("{}_{}_{}.{}", user_id, timestamp, random, ext)
}

/// 上传图片
async fn file_upload_img(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    
    while let Some(field) = multipart.next_field().await.map_err(|e| ApiError::new(1400, e.to_string()))? {
        let name = field.name().unwrap_or("").to_string();
        if name != "file" {
            continue;
        }
        
        // 获取信息
        let filename = field.file_name().unwrap_or("unknown").to_string();
        let content_type = field.content_type().unwrap_or("application/octet-stream").to_string();
        
        // 验证 MIME
        let mime_ok = ALLOWED_TYPES.iter().any(|(mime, _)| *mime == content_type);
        if !mime_ok {
            return Err(ApiError::new(1400, "Invalid file type"));
        }
        
        // 验证扩展名
        let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();
        let ext_ok = ALLOWED_TYPES.iter().any(|(_, exts)| exts.contains(&ext.as_str()));
        if !ext_ok {
            return Err(ApiError::new(1400, "Invalid file extension"));
        }
        
        // 读取数据
        let data = field.bytes().await.map_err(|e| ApiError::new(1400, e.to_string()))?;
        
        if data.is_empty() {
            return Err(ApiError::new(1400, "Empty file"));
        }
        
        if data.len() > MAX_UPLOAD_SIZE {
            return Err(ApiError::new(1400, "File too large"));
        }
        
        // 生成安全文件名
        let safe_name = generate_safe_filename(auth.user.id, &ext);
        let filepath = Path::new(&state.config.uploads_dir).join(&safe_name);
        
        // 确保目录存在
        if let Some(parent) = filepath.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        
        // 写入文件
        tokio::fs::write(&filepath, &data).await
            .map_err(|e| ApiError::new(1500, format!("Failed to save file: {}", e)))?;
        
        return Ok(ok(json!({
            "url": format!("/uploads/{}", safe_name),
            "name": safe_name,
            "size": data.len(),
            "type": content_type,
        })));
    }
    
    Err(ApiError::new(1400, "No file uploaded"))
}

/// 上传多个文件
async fn file_upload_files(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let mut files: Vec<Value> = vec![];
    
    while let Some(field) = multipart.next_field().await.map_err(|e| ApiError::new(1400, e.to_string()))? {
        let name = field.name().unwrap_or("").to_string();
        if name != "files" {
            continue;
        }
        
        let filename = field.file_name().unwrap_or("unknown").to_string();
        let content_type = field.content_type().unwrap_or("application/octet-stream").to_string();
        
        // 验证
        let mime_ok = ALLOWED_TYPES.iter().any(|(mime, _)| *mime == content_type);
        let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();
        let ext_ok = ALLOWED_TYPES.iter().any(|(_, exts)| exts.contains(&ext.as_str()));
        
        if !mime_ok || !ext_ok {
            continue; // 跳过无效文件
        }
        
        // 读取
        let data = match field.bytes().await {
            Ok(d) if d.len() <= MAX_UPLOAD_SIZE => d,
            _ => continue,
        };
        
        // 保存
        let safe_name = generate_safe_filename(auth.user.id, &ext);
        let filepath = Path::new(&state.config.uploads_dir).join(&safe_name);
        
        if let Some(parent) = filepath.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        
        if tokio::fs::write(&filepath, &data).await.is_ok() {
            files.push(json!({
                "url": format!("/uploads/{}", safe_name),
                "name": safe_name,
                "size": data.len(),
                "type": content_type,
            }));
        }
    }
    
    let count = files.len() as i64;
    Ok(list_ok(files, count))
}

/// 获取文件列表
async fn file_get_list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let mut files: Vec<Value> = vec![];
    let uploads_path = Path::new(&state.config.uploads_dir);
    
    if let Ok(mut entries) = tokio::fs::read_dir(uploads_path).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let filename = entry.file_name().to_string_lossy().to_string();
            // 只返回当前用户的文件
            if !filename.starts_with(&format!("{}_", auth.user.id)) {
                continue;
            }
            
            if let Ok(metadata) = entry.metadata().await {
                files.push(json!({
                    "name": filename,
                    "url": format!("/uploads/{}", filename),
                    "size": metadata.len() as i64,
                }));
            }
        }
    }
    
    let count = files.len() as i64;
    Ok(list_ok(files, count))
}

/// 删除文件
async fn file_deletes(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<Value>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    
    let names: Vec<String> = req.get("names")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();
    
    let mut deleted = 0;
    for name in names {
        // 安全检查
        if !name.starts_with(&format!("{}_", auth.user.id)) {
            continue;
        }
        if name.contains('/') || name.contains("\\") || name.contains("..") {
            continue;
        }
        
        let filepath = Path::new(&state.config.uploads_dir).join(&name);
        if tokio::fs::remove_file(&filepath).await.is_ok() {
            deleted += 1;
        }
    }
    
    Ok(ok(json!({ "deleted": deleted })))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/file/uploadImg", post(file_upload_img))
        .route("/api/file/uploadFiles", post(file_upload_files))
        .route("/api/file/getList", post(file_get_list))
        .route("/api/file/deletes", post(file_deletes))
}
