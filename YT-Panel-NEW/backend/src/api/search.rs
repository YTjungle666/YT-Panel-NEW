//! 搜索接口 - 高性能全文搜索
use axum::{
    extract::{Query, State},
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::sync::Arc;

use crate::{
    auth::{authenticate, AccessMode},
    error::{ApiError, ApiResult},
    state::AppState,
    error::ok,
};

/// 搜索请求
#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub search_url: bool, // 是否搜索 URL
}

fn default_limit() -> i64 {
    20
}

/// 搜索结果项
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
    pub score: f64, // 匹配分数
}

/// 创建搜索路由
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/search/bookmarks", get(search_bookmarks))
        .route("/api/search/suggestions", get(search_suggestions))
}

/// 书签搜索接口
async fn search_bookmarks(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Query(req): Query<SearchRequest>,
) -> ApiResult {
    let auth = authenticate(&headers, &state, AccessMode::PublicAllowed).await?;
    let user_id = auth.user.id;

    let query = req.query.trim();
    if query.is_empty() {
        return Ok(ok::<Vec<BookmarkSearchItem>>(vec![]));
    }

    // 构建搜索模式：支持多个关键词
    let patterns: Vec<String> = query
        .split_whitespace()
        .map(|s| format!("%{}%", s))
        .collect();

    // 基础查询
    let mut sql = String::from(
        "SELECT id, title, url, lan_url, icon, sort, is_folder, parent_id, \
         CASE WHEN LOWER(title) LIKE LOWER(?) THEN 1.0 \
              WHEN LOWER(url) LIKE LOWER(?) THEN 0.8 \
              ELSE 0.5 END as score \
         FROM bookmark WHERE user_id = ?"
    );

    // 动态添加条件
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

    // 构建查询
    let mut query_builder = sqlx::query_as::<_, BookmarkSearchItem>(&sql);

    // 绑定参数
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
        .map_err(|e| ApiError::db(e.to_string()))?;

    Ok(ok(results))
}

/// 快速搜索建议（自动补全）
async fn search_suggestions(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
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
    .map_err(|e| ApiError::db(e.to_string()))?;

    Ok(ok(suggestions))
}
