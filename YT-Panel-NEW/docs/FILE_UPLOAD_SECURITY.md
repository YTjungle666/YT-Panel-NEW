# 文件上传安全加固指南

## 📋 概述
本文档描述 YT-Panel-NEW 文件上传功能的安全加固实现。

**风险等级**: ⭐⭐⭐⭐ 高
**攻击向量**: 恶意文件上传、路径遍历、资源耗尽
**防护目标**: 只允许上传安全的图片文件

---

## 🛡️ 安全机制

### 1. 文件类型白名单 (MIME + 扩展名双重验证)

```rust
// 允许的文件类型
const ALLOWED_TYPES: &[(&str, &[&str])] = &[
    ("image/jpeg", &["jpg", "jpeg"]),
    ("image/png", &["png"]),
    ("image/gif", &["gif"]),
    ("image/webp", &["webp"]),
];

// 双重验证流程
1. 检查 Content-Type MIME 类型
2. 检查文件扩展名
3. 两者必须匹配白名单
```

**为什么需要双重验证？**
- MIME 可被伪造（Burp Suite 修改请求头）
- 扩展名可被篡改（shell.php → shell.jpg.php）
- 两者结合提高安全性

### 2. 文件大小限制

```rust
const MAX_UPLOAD_SIZE: usize = 10 * 1024 * 1024; // 10MB
```

**防护**: 防止资源耗尽攻击（DoS）

### 3. 文件名安全处理

```rust
// 原始文件名: "../../../etc/passwd" 或 "shell.php.jpg"
// 处理后: "{user_id}_{timestamp}_{random}.ext"

let safe_filename = format!(
    "{}_{}_{}.{}",
    user_id,           // 隔离不同用户
    timestamp,         // 防止覆盖
    random_string(8),  // 防止猜测
    whitelist_ext      // 强制使用白名单扩展名
);
```

**防护措施**:
- 路径遍历 (`../../../`)
- 文件名注入 (`; rm -rf /`)
- 可执行文件伪装 (`.php`, `.jsp`)

### 4. 存储隔离

```
uploads/
├── 1_1699123456_a1b2c3d4.jpg   # 用户1的文件
├── 1_1699123457_e5f6g7h8.png   # 用户1的另一文件
├── 2_1699123460_i9j0k1l2.webp  # 用户2的文件
└── ...
```

**隔离策略**: 按用户ID前缀隔离，防止用户A访问用户B的文件

---

## 🔧 实现代码

### 核心验证函数

```rust
/// 验证文件类型
fn validate_file_type(content_type: &str, filename: &str) -> Result<String, ApiError> {
    // MIME 检查
    let mime_ok = ALLOWED_TYPES.iter().any(|(mime, _)| *mime == content_type);
    if !mime_ok {
        return Err(ApiError::new(1400, "Invalid file type"));
    }
    
    // 扩展名检查
    let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();
    let ext_ok = ALLOWED_TYPES.iter().any(|(_, exts)| exts.contains(&ext.as_str()));
    if !ext_ok {
        return Err(ApiError::new(1400, "Invalid file extension"));
    }
    
    Ok(ext)
}

/// 生成安全文件名
fn generate_safe_filename(user_id: i64, ext: &str) -> String {
    let timestamp = chrono::Utc::now().timestamp_millis();
    let random = random_token(8);
    format!("{}_{}_{}.{}", user_id, timestamp, random, ext)
}
```

### Handler 实现

```rust
async fn file_upload_img(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> ApiResult {
    // 1. 认证
    let auth = authenticate(&headers, &state, AccessMode::LoginRequired).await?;
    
    while let Some(field) = multipart.next_field().await? {
        // 2. 获取文件名和类型
        let filename = field.file_name().unwrap_or("unknown").to_string();
        let content_type = field.content_type().unwrap_or("application/octet-stream");
        
        // 3. 验证文件类型
        let ext = validate_file_type(content_type, &filename)?;
        
        // 4. 读取数据（限制大小）
        let data = field.bytes().await?;
        if data.len() > MAX_UPLOAD_SIZE {
            return Err(ApiError::new(1400, "File too large"));
        }
        
        // 5. 生成安全文件名
        let safe_name = generate_safe_filename(auth.user.id, &ext);
        let filepath = Path::new(&state.config.uploads_dir).join(&safe_name);
        
        // 6. 写入文件
        tokio::fs::write(&filepath, &data).await?;
        
        // 7. 返回URL（不是路径）
        return Ok(ok(json!({
            "url": format!("/uploads/{}", safe_name),
            "name": safe_name,
            "size": data.len(),
        })));
    }
    
    Err(ApiError::new(1400, "No file uploaded"))
}
```

---

## 🧪 测试用例

### 合法上传
```bash
curl -X POST -F "file=@photo.jpg" \
  -H "Content-Type: image/jpeg" \
  http://localhost:3000/api/file/uploadImg
# ✅ 成功
```

### 恶意上传尝试
```bash
# 伪造 MIME 类型
curl -X POST -F "file=@shell.php" \
  -H "Content-Type: image/jpeg" \
  http://localhost:3000/api/file/uploadImg
# ❌ 失败：扩展名不匹配

# 路径遍历
curl -X POST -F "file=@../../../etc/passwd" \
  http://localhost:3000/api/file/uploadImg
# ❌ 失败：文件名被重写

# 超大文件
curl -X POST -F "file=@huge.bin" \
  http://localhost:3000/api/file/uploadImg
# ❌ 失败：大小超过 10MB
```

---

## 📊 安全审计检查表

- [ ] MIME 白名单验证
- [ ] 扩展名白名单验证
- [ ] 文件大小限制
- [ ] 文件名安全处理（去除路径、重命名）
- [ ] 用户隔离存储
- [ ] 返回 URL 而非绝对路径
- [ ] 日志记录上传行为

---

## 🔗 相关文件

- `backend/src/handlers/file.rs`
- `backend/src/utils/validate.rs` (建议创建)
