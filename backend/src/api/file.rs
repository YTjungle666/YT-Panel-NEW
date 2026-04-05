use crate::{
    auth::{authenticate, AccessMode},
    error::{list_ok, ok, ApiError, ApiResult},
    models::FileInfo,
    state::AppState,
};
use axum::{
    extract::{Multipart, State},
    http::HeaderMap,
    routing::post,
    Router,
};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::fs;

// 允许的图片扩展名
const IMAGE_EXTENSIONS: [&str; 5] = ["jpg", "jpeg", "png", "gif", "webp"];

// 允许的文件扩展名（包含图片）
const FILE_EXTENSIONS: [&str; 10] = [
    "jpg", "jpeg", "png", "gif", "webp", "pdf", "doc", "docx", "txt", "zip",
];

#[derive(Debug, Deserialize)]
struct GetListRequest {
    page: Option<i64>,
    size: Option<i64>,
    #[serde(rename = "type")]
    file_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeletesRequest {
    paths: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UploadResponse {
    path: String,
    url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MultiUploadResponse {
    paths: Vec<String>,
    urls: Vec<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/file/uploadImg", post(upload_img))
        .route("/api/file/uploadFiles", post(upload_files))
        .route("/api/file/getList", post(get_list))
        .route("/api/file/deletes", post(deletes))
}

/// POST /api/file/uploadImg - 上传单张图片
async fn upload_img(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> ApiResult {
    let _auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;

    let max_size = (state.config.max_upload_mb as usize) * 1024 * 1024;

    // 读取第一个字段（图片）
    let field = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::bad_param(format!("Failed to read multipart: {}", e)))?;

    let Some(field) = field else {
        return Err(ApiError::bad_param("No file uploaded"));
    };

    let filename = field.file_name().unwrap_or("unknown").to_string();
    let data = field
        .bytes()
        .await
        .map_err(|e| ApiError::bad_param(format!("Failed to read file: {}", e)))?;

    if data.len() > max_size {
        return Err(ApiError::bad_param(format!(
            "File size exceeds {}MB limit",
            state.config.max_upload_mb
        )));
    }

    // 验证图片扩展名
    let ext = get_extension(&filename).to_lowercase();
    if !IMAGE_EXTENSIONS.contains(&ext.as_str()) {
        return Err(ApiError::bad_param(format!(
            "Invalid image format. Allowed: {}",
            IMAGE_EXTENSIONS.join(", ")
        )));
    }

    // 生成唯一文件名
    let unique_name = generate_unique_filename(&filename);
    let relative_path = format!("images/{}", unique_name);
    let save_path = Path::new(&state.config.uploads_dir).join(&relative_path);

    // 确保目录存在
    if let Some(parent) = save_path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|e| ApiError::internal(format!("Failed to create directory: {}", e)))?;
    }

    // 保存文件
    fs::write(&save_path, &data)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to save file: {}", e)))?;

    // 构建URL路径
    let url = format!("/uploads/{}", relative_path);

    ok(UploadResponse {
        path: relative_path,
        url,
    })
}

/// POST /api/file/uploadFiles - 上传多个文件
async fn upload_files(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> ApiResult {
    let _auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;

    let max_size = (state.config.max_upload_mb as usize) * 1024 * 1024;
    let mut paths = Vec::new();
    let mut urls = Vec::new();

    // 读取所有文件字段
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::bad_param(format!("Failed to read multipart: {}", e)))?
    {
        let filename = field.file_name().unwrap_or("unknown").to_string();
        let data = field
            .bytes()
            .await
            .map_err(|e| ApiError::bad_param(format!("Failed to read file: {}", e)))?;

        if data.len() > max_size {
            return Err(ApiError::bad_param(format!(
                "File '{}' exceeds {}MB limit",
                filename, state.config.max_upload_mb
            )));
        }

        // 验证文件扩展名
        let ext = get_extension(&filename).to_lowercase();
        if !FILE_EXTENSIONS.contains(&ext.as_str()) {
            return Err(ApiError::bad_param(format!(
                "Invalid file format '{}'. Allowed: {}",
                ext,
                FILE_EXTENSIONS.join(", ")
            )));
        }

        // 生成唯一文件名
        let unique_name = generate_unique_filename(&filename);
        let relative_path = format!("files/{}", unique_name);
        let save_path = Path::new(&state.config.uploads_dir).join(&relative_path);

        // 确保目录存在
        if let Some(parent) = save_path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| ApiError::internal(format!("Failed to create directory: {}", e)))?;
        }

        // 保存文件
        fs::write(&save_path, &data)
            .await
            .map_err(|e| ApiError::internal(format!("Failed to save file: {}", e)))?;

        // 构建URL路径
        let url = format!("/uploads/{}", relative_path);

        paths.push(relative_path);
        urls.push(url);
    }

    if paths.is_empty() {
        return Err(ApiError::bad_param("No files uploaded"));
    }

    ok(MultiUploadResponse { paths, urls })
}

/// POST /api/file/getList - 获取文件列表
async fn get_list(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Json(req): axum::extract::Json<GetListRequest>,
) -> ApiResult {
    let _auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;

    let uploads_dir = &state.config.uploads_dir;
    let page = req.page.unwrap_or(1).max(1);
    let size = req.size.unwrap_or(10).max(1).min(100);

    // 根据类型决定扫描哪个目录
    let target_dir = match req.file_type.as_deref() {
        Some("image") => Path::new(uploads_dir).join("images"),
        Some("file") => Path::new(uploads_dir).join("files"),
        _ => Path::new(uploads_dir).to_path_buf(),
    };

    let mut all_files: Vec<FileInfo> = Vec::new();

    // 递归扫描目录
    if let Ok(mut entries) = fs::read_dir(&target_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.is_file() {
                if let Ok(metadata) = entry.metadata().await {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let relative_path = path
                        .strip_prefix(uploads_dir)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .to_string()
                        .replace('\\', "/");
                    let size = metadata.len() as i64;
                    let create_time = metadata
                        .created()
                        .ok()
                        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                        .map(|d| {
                            let secs = d.as_secs() as i64;
                            let naive = chrono::NaiveDateTime::from_timestamp_opt(secs, 0)
                                .unwrap_or_else(|| chrono::NaiveDateTime::MIN);
                            let dt: chrono::DateTime<chrono::Local> =
                                chrono::DateTime::from_naive_utc_and_local(
                                    naive,
                                    *chrono::Local::now().offset(),
                                );
                            dt.format("%Y-%m-%d %H:%M:%S").to_string()
                        })
                        .unwrap_or_else(|| "Unknown".to_string());

                    all_files.push(FileInfo {
                        name,
                        path: relative_path,
                        size,
                        create_time,
                    });
                }
            }
        }
    }

    // 按创建时间排序（最新的在前）
    all_files.sort_by(|a, b| b.create_time.cmp(&a.create_time));

    let total = all_files.len() as i64;
    let offset = ((page - 1) * size) as usize;
    let paginated: Vec<FileInfo> = all_files
        .into_iter()
        .skip(offset)
        .take(size as usize)
        .collect();

    Ok(list_ok(paginated, total))
}

/// POST /api/file/deletes - 删除文件
async fn deletes(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Json(req): axum::extract::Json<DeletesRequest>,
) -> ApiResult {
    let _auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let uploads_dir = &state.config.uploads_dir;
    let mut deleted_count = 0;
    let mut failed = Vec::new();
    
    // Pre-canonicalize base directory to ensure it's valid
    let canonical_base = match tokio::fs::canonicalize(uploads_dir).await {
        Ok(p) => p,
        Err(e) => {
            return Err(ApiError::internal(format!("Invalid upload directory: {}", e)));
        }
    };
    
    for path_str in &req.paths {
        // Block path traversal attempts
        if path_str.contains("..") || path_str.starts_with('/') {
            failed.push(path_str.clone());
            continue;
        }
        
        let target_path = Path::new(uploads_dir).join(path_str);
        let canonical_target = match tokio::fs::canonicalize(&target_path).await {
            Ok(p) => p,
            Err(_) => {
                failed.push(path_str.clone());
                continue;
            }
        };
        
        if !canonical_target.starts_with(&canonical_base) {
            failed.push(path_str.clone());
            continue;
        }
        
        match tokio::fs::remove_file(&canonical_target).await {
            Ok(_) => deleted_count += 1,
            Err(_) => failed.push(path_str.clone()),
        }
    }
    
    ok(serde_json::json!({
        "deleted": deleted_count,
        "failed": failed,
    }))
}

// 辅助函数：获取文件扩展名
fn get_extension(filename: &str) -> String {
    Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_string()
}

// 辅助函数：生成唯一文件名
fn generate_unique_filename(filename: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let ext = get_extension(filename);
    let base = Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");

    if ext.is_empty() {
        format!("{}_{}", base, timestamp)
    } else {
        format!("{}_{}.{}", base, timestamp, ext)
    }
}
