use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone)]
pub struct ApiError {
    pub code: i32,
    pub msg: String,
}

pub type ApiResult = Result<Response, ApiError>;

impl ApiError {
    pub fn new(code: i32, msg: impl Into<String>) -> Self {
        Self {
            code,
            msg: msg.into(),
        }
    }

    pub fn bad_param(msg: impl Into<String>) -> Self {
        Self::new(1400, msg)
    }

    pub fn unauthorized(msg: impl Into<String>) -> Self {
        Self::new(1001, msg)
    }

    pub fn db(err: impl Into<String>) -> Self {
        Self::new(1200, format!("Database error[{}]", err.into()))
    }

    pub fn not_found() -> Self {
        Self::new(1404, "Resource not found")
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::new(1500, msg)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let envelope = ApiEnvelope {
            code: self.code,
            msg: self.msg,
            data: None::<Value>,
        };
        Json(envelope).into_response()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    ok(json!({
        "list": list,
        "count": count
    }))
}

pub fn with_cookie(response: Response, cookie: &str) -> Response {
    let mut resp = response;
    if let Ok(value) = axum::http::HeaderValue::from_str(cookie) {
        resp.headers_mut().insert(
            axum::http::header::SET_COOKIE,
            value,
        );
    }
    resp
}
