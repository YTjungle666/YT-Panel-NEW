//! Search handlers - 搜索功能

use axum::{
    extract::{Query, State},
    http::HeaderMap,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::sync::Arc;

use crate::{
    AppState, ApiError, ApiResult,
    authenticate, AccessMode,
    ok,
};

#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub search_url: bool,
}

fn default_limit() -> i64 {
    20
}

#[derive(Debug, Serialize, FromRow)]
pub struct BookmarkSearchItem {
    pub id: i64,
    pub title: String,
    pub url: String,
    pub lan_url: Option<String>,
    pub icon: Option<String>,
    pub sort: i64,
    pub is_folder: i64,
    pub parent_id: i64,
    pub score: f64,
}

/// 搜索书签
pub async fn search_bookmarks(
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

    let patterns: Vec<String> = query
        .split_whitespace()
        .map(|s| format!("%{}%", s))
        .collect();

    let mut sql = String::from(
        "SELECT id, title, url, lan_url, icon, sort, is_folder, parent_id, \\
         CASE WHEN LOWER(title) LIKE LOWER(?) THEN 1.0 \\
              WHEN LOWER(url) LIKE LOWER(?) THEN 0.8 \\
              ELSE 0.5 END as score \\
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

/// 搜索建议
pub async fn search_suggestions(
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

/// 搜索路由
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/search/bookmarks", get(search_bookmarks))
        .route("/api/search/suggestions", get(search_suggestions))
}
