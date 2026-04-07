use axum::{
    http::{header::SET_COOKIE, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_json::Value;
use tracing::error;

#[derive(Debug, Clone)]
pub struct ApiError {
    pub code: i32,
    pub msg: String,
    pub status: StatusCode,
}

impl ApiError {
    pub fn new(code: i32, msg: impl Into<String>) -> Self {
        Self::new_with_status(code, msg, status_from_code(code))
    }

    pub fn new_with_status(code: i32, msg: impl Into<String>, status: StatusCode) -> Self {
        Self {
            code,
            msg: msg.into(),
            status,
        }
    }

    pub fn bad_param(msg: impl Into<String>) -> Self {
        Self::new(1400, msg)
    }

    #[allow(dead_code)]
    pub fn unauthorized() -> Self {
        Self::new(1100, "Unauthorized")
    }

    #[allow(dead_code)]
    pub fn forbidden() -> Self {
        Self::new(1103, "Forbidden")
    }

    #[allow(dead_code)]
    pub fn not_found() -> Self {
        Self::new(1104, "Not Found")
    }

    pub fn db(err: impl Into<String>) -> Self {
        let err = err.into();
        error!("database error: {}", err);
        Self::new_with_status(1200, "Database error", StatusCode::INTERNAL_SERVER_ERROR)
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        error!("internal error: {}", msg);
        Self::new_with_status(1500, "Internal server error", StatusCode::INTERNAL_SERVER_ERROR)
    }

    pub fn password_change_required() -> Self {
        Self::new_with_status(1108, "PASSWORD_CHANGE_REQUIRED", StatusCode::FORBIDDEN)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let envelope = ApiEnvelope::<Value> {
            code: self.code,
            msg: self.msg,
            data: None,
        };
        (self.status, Json(envelope)).into_response()
    }
}

fn status_from_code(code: i32) -> StatusCode {
    match code {
        1000 | 1001 | 1003 | 1100 => StatusCode::UNAUTHORIZED,
        1004 | 1005 | 1103 | 1108 | 1403 => StatusCode::FORBIDDEN,
        1006 | 1104 => StatusCode::NOT_FOUND,
        1200 | 1500 => StatusCode::INTERNAL_SERVER_ERROR,
        _ => StatusCode::BAD_REQUEST,
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiEnvelope<T: Serialize> {
    pub code: i32,
    pub msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

pub fn ok<T: Serialize>(data: T) -> Response {
    Json(ApiEnvelope {
        code: 0,
        msg: "OK".into(),
        data: Some(data),
    })
    .into_response()
}

pub fn ok_empty() -> Response {
    Json(ApiEnvelope::<Value> {
        code: 0,
        msg: "OK".into(),
        data: None,
    })
    .into_response()
}

pub fn list_ok<T: Serialize>(list: T, count: i64) -> Response {
    ok(serde_json::json!({
        "list": list,
        "count": count
    }))
}

pub fn with_set_cookie(mut response: Response, cookie: &str) -> Result<Response, ApiError> {
    let value = HeaderValue::from_str(cookie).map_err(|e| ApiError::internal(e.to_string()))?;
    response.headers_mut().append(SET_COOKIE, value);
    Ok(response)
}

pub type ApiResult = Result<Response, ApiError>;
