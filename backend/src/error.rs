use axum::{
    http::{header::SET_COOKIE, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct ApiError {
    pub code: i32,
    pub msg: String,
}

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
        Self::new(1200, format!("Database error[{}]", err.into()))
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::new(1500, msg)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let envelope = ApiEnvelope::<Value> {
            code: self.code,
            msg: self.msg,
            data: None,
        };
        (StatusCode::OK, Json(envelope)).into_response()
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
