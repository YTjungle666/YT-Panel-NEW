# YT-Panel-NEW

> 🚀 面向自托管服务器的现代化个人导航面板

YT-Panel-NEW 是一款专为家庭服务器、NAS 和个人云设计的书签管理面板。采用 Rust 重写后端，提供更高的性能和安全性，同时保持与原有前端的无缝兼容。

![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/YTjungle666/YT-Panel-NEW/docker-publish.yml)
![License](https://img.shields.io/badge/license-PolyForm--Noncommercial--1.0.0-orange)

---

## ✨ 特性

- 🦀 **Rust 高性能后端** - 内存安全、并发高效、资源占用低
- 🔒 **内置安全认证** - JWT + Session 双模式，支持公开访问
- 📱 **响应式界面** - 基于 Vue3 + Tailwind，完美适配移动端
- 🎨 **丰富的模块** - 书签、应用图标、搜索引擎、系统监控、便签
- 📁 **文件管理** - 支持图片/文件上传，自动按日期归档
- 🔄 **数据迁移** - 一键从旧版迁移，零数据丢失
- 🐳 **容器化部署** - 支持 Docker / Docker Compose / LXC / CT

---

## 🚀 快速开始

### Docker Compose（推荐）

```bash
mkdir -p yt-panel && cd yt-panel

# 下载 compose 文件
curl -O https://raw.githubusercontent.com/YTjungle666/YT-Panel-NEW/main/docker-compose.yml

# 启动服务
docker compose up -d

# 访问 http://localhost
# 默认账号: admin / 123456
```

### Docker 单容器

以下镜像及其派生产物仅可按 `PolyForm-Noncommercial-1.0.0` 用于非商业用途：

```bash
docker run -d \
  --name yt-panel \
  -p 80:80 \
  -v $(pwd)/data/database:/app/database \
  -v $(pwd)/data/uploads:/app/uploads \
  --restart unless-stopped \
  ghcr.io/ytjungle666/yt-panel-new:latest
```

### LXC / Proxmox CT

适用于 PVE 等虚拟化平台，直接复用 Alpine Docker 镜像 rootfs 创建 CT。模板及镜像同样仅允许非商业用途：

```bash
docker pull ghcr.io/ytjungle666/yt-panel-new:latest
cid=$(docker create ghcr.io/ytjungle666/yt-panel-new:latest)
docker export "$cid" | zstd -19 -T0 -o /var/lib/vz/template/cache/yt-panel-alpine-ct-template.tar.zst -f
docker rm -f "$cid"

pct create 100 local:vztmpl/yt-panel-alpine-ct-template.tar.zst \
  --ostype unmanaged \
  --hostname yt-panel \
  --memory 1024 \
  --cores 2 \
  --rootfs local-lvm:4 \
  --net0 name=eth0,bridge=vmbr0,ip=dhcp \
  --unprivileged 0 \
  --start 1
```

---

## 📋 配置说明

### 环境变量

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `YT_PANEL_CONFIG` | 配置文件路径 | `/app/conf/app.toml` |
| `RUST_LOG` | 日志级别 | `info` |

### 配置文件示例

```toml
# config/app.toml
host = "0.0.0.0"          # 监听地址
port = 80                # 监听端口
database_url = "sqlite:///app/database/database.db"
uploads_dir = "/app/uploads"
frontend_dist = "/app/web"
max_upload_mb = 10       # 最大上传限制
public_user_id = 1       # 公开访问用户ID
```

### 目录说明

```
/app/
├── yt-panel          # 主程序
├── conf/
│   └── app.toml      # 配置文件
├── database/           # SQLite 数据库（需持久化）
├── uploads/            # 上传文件（需持久化）
└── web/                # 前端静态资源
```

---

## 🛡️ 安全特性

- ✅ **bcrypt 密码哈希** - 自适应成本因子 (12)
- ✅ **SQL 参数化查询** - 完全防止 SQL 注入
- ✅ **Session + Token 双认证** - 灵活的登录方式
- ✅ **字段白名单校验** - 防止非法查询
- ✅ **CORS 支持** - 安全的跨域配置

---

## 🔄 从旧版迁移

支持从 YT-Panel (Go 版) 无缝迁移：

```bash
# 下载迁移脚本
curl -O https://raw.githubusercontent.com/YTjungle666/YT-Panel-NEW/main/scripts/migrate_from_yt_panel.py

# 预览迁移
python3 migrate_from_yt_panel.py --dry-run --from /path/to/old-yt-panel

# 执行迁移
python3 migrate_from_yt_panel.py --force --from /path/to/old-yt-panel
```

迁移内容包括：
- 用户数据和密码
- 书签、图标分组
- 上传的文件
- 系统设置

---

## 🏗️ 自行构建

### 前置要求

- Node.js 22+
- Rust 1.85+
- npm / cargo
- Docker + zstd（导出 PVE CT 模板时需要）

### 构建步骤

```bash
# 克隆仓库
git clone https://github.com/YTjungle666/YT-Panel-NEW.git
cd YT-Panel-NEW

# 类型检查
npm ci
npm run type-check

# 构建前端和 musl 后端
npm run build-only
(cd backend && cargo build --release --target x86_64-unknown-linux-musl)

# 构建 Alpine Docker 镜像
docker build -t yt-panel:alpine .

# 导出 PVE CT 模板
./scripts/export-pve-template.sh yt-panel:alpine ./artifacts/YT-Panel-NEW/release/yt-panel-alpine-ct-template.tar.zst
```

生成的 `yt-panel-linux-amd64.tar.gz`、`yt-panel-alpine-ct-template.tar.zst` 和 Docker 镜像都会附带同一份 [LICENSE](LICENSE)。

---

## 📖 API 文档

主要 API 端点：

| 端点 | 说明 |
|------|------|
| `POST /api/auth/login` | 用户登录 |
| `POST /api/auth/register` | 用户注册 |
| `GET /api/user/info` | 获取用户信息 |
| `GET /api/system/monitor` | 系统监控数据 |
| `GET /api/panel/bookmarks` | 书签列表 |
| `POST /api/file/upload` | 文件上传 |

完整 API 路由与处理逻辑见项目源码 `backend/src/handlers/`。

---

## 🤝 贡献

欢迎提交 Issue 和 PR！

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送分支 (`git push origin feature/AmazingFeature`)
5. 创建 Pull Request

---

## 📄 许可证

PolyForm Noncommercial License 1.0.0 - 禁止商业用途，详见 [LICENSE](LICENSE) 文件

---

## 💬 交流群

- GitHub Issues: [提交问题](https://github.com/YTjungle666/YT-Panel-NEW/issues)

---

<p align="center">
  Made with ❤️ using Rust & Vue
</p>
