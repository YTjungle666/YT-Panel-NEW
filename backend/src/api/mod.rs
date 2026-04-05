pub mod auth;
pub mod file;
pub mod panel;
pub mod system;
pub mod user;

use axum::Router;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(auth::router())
        .merge(user::router())
        .merge(system::router())
        .merge(file::router())
        .merge(panel::router())
}
