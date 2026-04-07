use std::path::Path;

use axum::{extract::{Multipart, State}, http::HeaderMap, Json};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use mime_guess::MimeGuess;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use tokio::fs;

use crate::{
    auth::authenticate,
    error::{list_ok, ok, ok_empty, ApiError, ApiResult},
    models::{AccessMode, AppState},
    utils::{max_upload_bytes, resolve_uploaded_file_path, save_upload_field},
};

#[derive(Deserialize)]
pub struct IdsRequest {
    ids: Vec<i64>,
}

pub async fn file_upload_img(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> ApiResult {
    let _auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    if let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::new(1300, e.to_string()))?
    {
        let file_name = field.file_name().unwrap_or("image.png").to_string();
        let ext = Path::new(&file_name)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("png")
            .to_lowercase();
        let allowed = ["png", "jpg", "jpeg", "gif", "webp", "ico"];
        if !allowed.contains(&ext.as_str()) {
            return Err(ApiError::new(1301, "Unsupported file format"));
        }
        let bytes = field
            .bytes()
            .await
            .map_err(|e| ApiError::new(1300, e.to_string()))?;
        let max_bytes = max_upload_bytes(&state.config) as usize;
        if bytes.len() > max_bytes {
            return Err(ApiError::new(
                1300,
                format!("file too large (max {}MB)", state.config.max_upload_mb),
            ));
        }
        let mime = MimeGuess::from_ext(&ext).first_or_octet_stream();
        let data_url = format!("data:{};base64,{}", mime, B64.encode(bytes));
        return Ok(ok(json!({ "imageUrl": data_url })));
    }
    Err(ApiError::new(1300, "Upload failed"))
}

pub async fn file_upload_files(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let mut succ_map = serde_json::Map::new();
    let mut err_files = Vec::<String>::new();
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::new(1300, e.to_string()))?
    {
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

pub async fn file_get_list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    let rows = sqlx::query(
        "SELECT id, src, file_name, created_at, updated_at FROM file WHERE user_id = ? ORDER BY created_at DESC",
    )
    .bind(auth.user.id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::db(e.to_string()))?;
    let list: Vec<Value> = rows
        .into_iter()
        .map(|row| {
            let src: String = row.get("src");
            json!({
                "id": row.get::<i64, _>("id"),
                "src": src.trim_start_matches('.'),
                "fileName": row.try_get::<Option<String>, _>("file_name").unwrap_or(None),
                "createTime": row.try_get::<Option<String>, _>("created_at").unwrap_or(None),
                "updateTime": row.try_get::<Option<String>, _>("updated_at").unwrap_or(None),
                "path": src,
            })
        })
        .collect();
    let count = list.len() as i64;
    Ok(list_ok(list, count))
}

pub async fn file_deletes(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<IdsRequest>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    for id in req.ids {
        if let Some(src) = sqlx::query_scalar::<_, String>(
            "SELECT src FROM file WHERE id = ? AND user_id = ?",
        )
        .bind(id)
        .bind(auth.user.id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::db(e.to_string()))?
        {
            if let Some(path) = resolve_uploaded_file_path(&state.config.uploads_dir, &src) {
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
