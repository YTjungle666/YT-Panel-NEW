//! Handlers 模块 - 按功能分组的 API 处理器

pub mod auth;
pub mod common;
pub mod file;
pub mod panel;
pub mod search;
pub mod system;
pub mod user;

use axum::Router;
use crate::AppState;

/// 组合所有 handlers
pub fn handlers_router() -> Router<AppState> {
    Router::new()
        .merge(common::router())
        .merge(auth::router())
        .merge(user::router())
        .merge(search::router())
        .merge(system::router())
        .merge(file::router())
        .merge(panel::router())
}
