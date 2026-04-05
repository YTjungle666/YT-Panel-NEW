# 输入验证与安全限制

## 📋 概述
本文档描述 API 输入的长度限制和安全验证规则。

**目标**: 防止资源耗尽攻击、DoS、数据库溢位
**策略**: 统一输入校验，失败快速拒绝

---

## 📏 长度限制表

| 字段 | 类型 | 最大长度 | 说明 |
|------|------|----------|------|
| username | 字符串 | 32 | 用户名 |
| password | 字符串 | 128 | 密码（哈希后存储） |
| email | 字符串 | 255 | 邮箱（RFC 5321） |
| bookmark_title | 字符串 | 255 | 书签标题 |
| bookmark_url | 字符串 | 2048 | URL（浏览器限制） |
| bookmark_lan_url | 字符串 | 2048 | 内网URL |
| file_name | 字符串 | 256 | 原始文件名 |
| search_query | 字符串 | 100 | 搜索关键词 |
| notepad_title | 字符串 | 100 | 便签标题 |
| notepad_content | 文本 | 100KB | 便签内容 |
| group_title | 字符串 | 100 | 分组标题 |
| icon_title | 字符串 | 100 | 图标标题 |

---

## 🔧 实现方式

### 方式 1: 结构体验证（推荐）

```rust
use validator::Validate;

#[derive(Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(length(min = 3, max = 32))]
    pub username: String,
    
    #[validate(length(min = 8, max = 128))]
    pub password: String,
    
    #[validate(email, length(max = 255))]
    pub email: String,
}

// Handler
async fn create_user(Json(req): Json<CreateUserRequest>) -> ApiResult {
    req.validate()?; // 自动验证
    // ...
}
```

### 方式 2: 手动验证（当前使用）

```rust
const MAX_USERNAME_LEN: usize = 32;
const MAX_PASSWORD_LEN: usize = 128;

fn validate_username(username: &str) -> Result<(), ApiError> {
    if username.len() > MAX_USERNAME_LEN {
        return Err(ApiError::bad_param(
            format!("Username too long: {} > {}", username.len(), MAX_USERNAME_LEN)
        ));
    }
    Ok(())
}
```

### 方式 3: 数据库层限制（兜底）

```sql
CREATE TABLE user (
    username VARCHAR(32) NOT NULL,  -- 数据库层硬限制
    password VARCHAR(128) NOT NULL,
    email VARCHAR(255),
    -- ...
);
```

---

## ⚡ 性能优化

### 提前截断 vs 拒绝

| 策略 | 行为 | 适用场景 |
|------|------|----------|
| **拒绝** | 返回错误 | API 输入（推荐）|
| **截断** | 静默截断 | 日志、显示文本 |
| **滚动** | 删除旧数据 | 历史记录 |

**推荐**: API 统一使用拒绝策略，让客户端处理错误。

### 快速失败

```rust
// ❌ 先处理再验证（浪费资源）
let data = expensive_parse(&input)?;
if data.len() > MAX_LEN { return Err(...); }

// ✅ 先验证再处理（快速失败）
if input.len() > MAX_LEN { return Err(...); }
let data = expensive_parse(&input)?;
```

---

## 📝 错误消息规范

```rust
// 统一的错误格式
{
    "code": 1400,
    "msg": "Input too long: username (35) > max (32)"
}

// 不暴露内部细节（防信息泄露）
// ❌ 不要: "SQL error: value too long for column"
// ✅ 要: "Input validation failed"
```

---

## 🔍 审计日志

记录超出限制的请求（用于检测攻击）：

```rust
if input.len() > MAX_LEN {
    tracing::warn!(
        "Input validation failed: field={}, len={}, max={}, client_ip={}",
        field_name, input.len(), max_len, client_ip
    );
    return Err(...);
}
```

---

## 📁 相关文件

- `backend/src/utils/validate.rs` - 验证函数
- `backend/src/models.rs` - 结构体定义
- `backend/src/handlers/*.rs` - Handler 实现
