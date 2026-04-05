//! Common handlers - 无需认证的公共接口
use axum::{
    extract::Query,
    http::HeaderMap,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use crate::{AppState, ApiResult, ok, AccessMode};

/// 提取客户端 IP
pub fn extract_client_ip(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.trim().to_string())
        })
        .or_else(|| {
            headers
                .get("cf-connecting-ip")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.trim().to_string())
        })
}

/// 检查是否为内网 IP
pub fn is_private_ip(ip: std::net::IpAddr) -> bool {
    use std::net::IpAddr;
    match ip {
        IpAddr::V4(addr) => {
            let octets = addr.octets();
            // 10.0.0.0/8
            octets[0] == 10
            // 172.16.0.0/12
            || (octets[0] == 172 && (16..=31).contains(&octets[1]))
            // 192.168.0.0/16
            || (octets[0] == 192 && octets[1] == 168)
            // 127.0.0.0/8
            || octets[0] == 127
        }
        IpAddr::V6(addr) => {
            // ::1
            addr.is_loopback()
            // fc00::/7
            || addr.segments()[0] & 0xfe00 == 0xfc00
        }
    }
}

/// 关于信息
pub async fn about() -> ApiResult {
    Ok(ok(json!({
        "versionName": env!("CARGO_PKG_VERSION"),
        "versionCode": 1,
    })))
}

/// 健康检查
pub async fn ping() -> &'static str {
    "pong"
}

/// 检查是否为内网访问
pub async fn is_lan(headers: HeaderMap) -> ApiResult {
    let ip = extract_client_ip(&headers);
    let is_lan = ip
        .as_deref()
        .and_then(|s| s.parse::<std::net::IpAddr>().ok())
        .map(is_private_ip)
        .unwrap_or(false);
    Ok(ok(json!({
        "isLan": is_lan,
        "clientIp": ip
    })))
}

/// 路由
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/about", get(about))
        .route("/api/isLan", get(is_lan))
        .route("/ping", get(ping))
}
