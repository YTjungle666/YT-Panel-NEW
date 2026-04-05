//! 共享类型定义

use serde::{Deserialize, Serialize};

/// 用户查询字段枚举 - 防止SQL注入
#[derive(Debug, Clone, Copy)]
pub enum UserField {
    Username,
    Email,
    Id,
    Token,
}

impl UserField {
    pub fn as_str(&self) -> &'static str {
        match self {
            UserField::Username => "username",
            UserField::Email => "mail",
            UserField::Id => "id",
            UserField::Token => "token",
        }
    }
}

/// bcrypt成本因子 - 增强安全性
pub const BCRYPT_COST: u32 = 12;
