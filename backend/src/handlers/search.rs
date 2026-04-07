use axum::{extract::{Query, State}, http::HeaderMap};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::{
    auth::authenticate,
    error::{ok, ApiError, ApiResult},
    models::{AccessMode, AppState},
};

#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    query: String,
    #[serde(default = "search_default_limit")]
    limit: i64,
    #[serde(default)]
    search_url: bool,
}

#[derive(Debug, Serialize)]
pub struct BookmarkSearchItem {
    id: i64,
    title: String,
    url: String,
    lan_url: Option<String>,
    icon: Option<String>,
    sort: i64,
    is_folder: i64,
    parent_id: i64,
    score: f64,
}

fn search_default_limit() -> i64 {
    20
}

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

    let limit = req.limit.clamp(1, 100);
    let score_pattern = format!("%{}%", query);
    let patterns: Vec<String> = query
        .split_whitespace()
        .map(|s| format!("%{}%", s))
        .collect();

    let mut sql = String::from(
        "SELECT id, title, url, lan_url, icon, sort, is_folder, parent_id, \
         CASE WHEN LOWER(title) LIKE LOWER(?) THEN 1.0 \
              WHEN LOWER(url) LIKE LOWER(?) THEN 0.8 \
              ELSE 0.5 END as score \
         FROM bookmark WHERE user_id = ?",
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
        sql.push(')');
    }
    sql.push_str(" ORDER BY score DESC, sort ASC LIMIT ?");

    let mut query_builder = sqlx::query(&sql)
        .bind(&score_pattern)
        .bind(&score_pattern)
        .bind(user_id);

    for pattern in &patterns {
        if req.search_url {
            query_builder = query_builder.bind(pattern).bind(pattern);
        } else {
            query_builder = query_builder.bind(pattern);
        }
    }
    query_builder = query_builder.bind(limit);

    let rows = query_builder
        .fetch_all(&state.db)
        .await
        .map_err(|_| ApiError::new(1200, "Database error"))?;

    let results: Vec<BookmarkSearchItem> = rows
        .into_iter()
        .map(|row| BookmarkSearchItem {
            id: row.get::<i64, _>("id"),
            title: row.get::<String, _>("title"),
            url: row.get::<String, _>("url"),
            lan_url: row.try_get::<Option<String>, _>("lan_url").unwrap_or(None),
            icon: row.try_get::<Option<String>, _>("icon").unwrap_or(None),
            sort: row.try_get::<Option<i64>, _>("sort").unwrap_or(Some(0)).unwrap_or(0),
            is_folder: row
                .try_get::<Option<i64>, _>("is_folder")
                .unwrap_or(Some(0))
                .unwrap_or(0),
            parent_id: row
                .try_get::<Option<i64>, _>("parent_id")
                .unwrap_or(Some(0))
                .unwrap_or(0),
            score: row.try_get::<f64, _>("score").unwrap_or(0.0),
        })
        .collect();

    Ok(ok(results))
}

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
        "SELECT DISTINCT title FROM bookmark WHERE user_id = ? AND LOWER(title) LIKE LOWER(?) ORDER BY sort ASC LIMIT 10",
    )
    .bind(user_id)
    .bind(&pattern)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::new(1200, format!("Database error[{}]", e)))?;

    Ok(ok(suggestions))
}
