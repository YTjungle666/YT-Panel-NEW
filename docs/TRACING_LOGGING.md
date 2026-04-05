# 结构化日志与可观测性

## 📋 概述
使用 `tracing` 实现结构化日志，提升问题排查效率。

---

## 🎯 日志级别

| 级别 | 用途 | 示例 |
|------|------|------|
| ERROR | 系统错误，需立即处理 | 数据库连接失败、文件写入失败 |
| WARN | 异常情况，可容忍 | 登录失败、文件上传过大 |
| INFO | 关键业务事件 | 用户登录、注册成功 |
| DEBUG | 调试信息 | SQL 查询、请求参数 |
| TRACE | 详细追踪 | 函数入参出参 |

---

## 🔧 实现示例

### 1. Handler 级别日志

```rust
use tracing::{info, warn, error};

#[tracing::instrument(skip(state, headers), fields(user_id, ip))]
pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<LoginRequest>,
) -> ApiResult {
    // 自动记录入参
    let username = req.username.trim();
    
    let ip = extract_client_ip(&headers).unwrap_or_default();
    tracing::Span::current().record("ip", &ip);
    
    info!(target: "auth", "Login attempt: username={}", username);
    
    // 验证
    let Some(user) = load_user_by_username(&state.db, username).await? else {
        warn!(target: "auth", "Login failed: user not found, username={}", username);
        return Err(ApiError::new(1003, "Incorrect username or password"));
    };
    
    // 验证密码
    if !verify_password_compat(&req.password, &user.password) {
        warn!(target: "auth", "Login failed: wrong password, username={}, ip={}", username, ip);
        return Err(ApiError::new(1003, "Incorrect username or password"));
    }
    
    tracing::Span::current().record("user_id", user.id);
    info!(target: "auth", "Login successful: user_id={}, username={}", user.id, username);
    
    Ok(...)
}
```

### 2. 中间件日志

```rust
// tower-http trace 层
.layer(TraceLayer::new_for_http()
    .make_span_with(|request: &Request<_>| {
        tracing::info_span!(
            "http_request",
            method = %request.method(),
            uri = %request.uri(),
            version = ?request.version(),
        )
    })
    .on_request(|request: &Request<_>, _span: &Span| {
        tracing::debug!("Request: {} {}", request.method(), request.uri());
    })
    .on_response(|response: &Response, latency: Duration, _span: &Span| {
        tracing::info!(
            "Response: {} in {:?}",
            response.status(),
            latency
        );
    })
)
```

### 3. 错误日志

```rust
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self.code {
            1000..=1999 => "auth",
            1200..=1299 => "db",
            1400..=1499 => "param",
            _ => "other",
        };
        
        error!(
            target: "api",
            code = self.code,
            status = status,
            msg = self.msg,
            "API error"
        );
        
        // ...
    }
}
```

---

## 📊 日志配置

```rust
// main.rs
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn init_logging() {
    let env_filter = tracing_subscriber::EnvFilter::new(
        std::env::var("RUST_LOG")
            .unwrap_or_else(|_| "yt_panel=info,tower_http=debug".into()),
    );
    
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_level(true)
        .pretty();
    
    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();
}
```

---

## 🔍 常用查询

```bash
# 查看所有登录失败
RUST_LOG=warn cargo run 2>&1 | grep "Login failed"

# 查看特定用户的操作
RUST_LOG=trace cargo run 2>&1 | grep "user_id=123"

# 查看慢请求（>100ms）
RUST_LOG=info cargo run 2>&1 | grep -E "Response:.*[0-9]{3}ms"
```

---

## 🚀 下一步：分布式追踪

使用 OpenTelemetry 集成 Jaeger：

```rust
use opentelemetry::trace::Tracer;

#[tracing::instrument]
async fn process_bookmark(user_id: i64, bookmark: Bookmark) {
    let tracer = global::tracer("bookmark");
    let mut span = tracer.start("save_bookmark");
    
    // ...
    
    span.end();
}
```
