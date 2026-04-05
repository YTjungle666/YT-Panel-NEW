//! 通用工具函数

use rand::{distributions::Alphanumeric, Rng};
use std::path::PathBuf;

/// 生成随机token
pub fn random_token(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

/// 从SQLite URL提取数据库文件路径
pub fn sqlite_file_path(database_url: &str) -> Option<PathBuf> {
    let raw = database_url.trim();
    if raw.is_empty() || raw.eq_ignore_ascii_case("sqlite::memory:") {
        return None;
    }
    let path_part = raw
        .strip_prefix("sqlite://")
        .or_else(|| raw.strip_prefix("sqlite:"))
        .unwrap_or(raw)
        .split('?')
        .next()
        .unwrap_or("")
        .trim();
    if path_part.is_empty() || path_part.eq_ignore_ascii_case(":memory:") {
        return None;
    }
    Some(PathBuf::from(path_part))
}

/// 解析i64
pub fn parse_i64(v: Option<&serde_json::Value>) -> i64 {
    v.and_then(|x| x.as_i64()).unwrap_or(0)
}

/// 解析字符串
pub fn parse_string(v: Option<&serde_json::Value>) -> String {
    v.and_then(|x| x.as_str()).map(|s| s.to_string()).unwrap_or_default()
}

/// 解析可选字符串
pub fn parse_opt_string(v: Option<&serde_json::Value>) -> Option<String> {
    v.and_then(|x| x.as_str()).map(|s| s.to_string())
}
