# YT-Panel → YT-panel-Rust 迁移方案

这份方案按 **可执行、低风险、可回滚** 来写，默认目标是：

- 旧项目：`projects/YT-Panel/service`
- 新项目源码：`projects/YT-panel-Rust`
- 新项目运行时：`artifacts/YT-panel-Rust/runtime`

核心原则：**不直接改旧库，不在 repo 内落运行时数据，先复制、再切换、保留回滚面。**

---

## 1. 审核结论（基于当前仓库状态）

### 1.1 旧 YT-Panel 运行目录

旧服务根目录：`projects/YT-Panel/service`

当前运行相关路径：

- 配置：`service/conf/conf.ini`
- SQLite：`service/database/database.db`
- uploads：`service/uploads/`
- 运行日志：`service/runtime/runlog/running.log`
- 临时目录配置：`base.source_temp_path=./runtime/temp`

### 1.2 新 YT-panel-Rust 运行目录

新项目运行时已经按 YT 规则落到 `artifacts/`：

- 配置：`projects/YT-panel-Rust/backend/config/app.toml`
- SQLite：`artifacts/YT-panel-Rust/runtime/database/database.db`
- uploads：`artifacts/YT-panel-Rust/runtime/uploads/`
- 前端 dist：`artifacts/YT-panel-Rust/frontend-dist`
- Rust target：`artifacts/YT-panel-Rust/backend-target`
- 日志：`artifacts/YT-panel-Rust/logs`

### 1.3 SQLite 结构差异

实查结果：**旧库的核心业务表与 Rust 版当前运行库基本同构**。

旧库现有表：

- `bookmark`
- `file`
- `item_icon`
- `item_icon_group`
- `module_config`
- `notepad`
- `search_engine`
- `system_setting`
- `user`
- `user_config`

Rust 运行库额外多一个：

- `notice`

结论：

1. **数据库迁移不需要做复杂字段映射**，优先走“复制旧库 + 补 Rust 缺少的表/默认配置”。
2. 旧库里 `system_setting` 可能为空；Rust 端需要确保以下配置存在：
   - `system_application`
   - `disclaimer`
   - `web_about_description`
   - `panel_public_user_id`
3. Rust 端登录兼容旧密码格式：
   - 如果旧库是 bcrypt，直接可用
   - 如果旧库历史上有 md5 存量，Rust 端登录时会自动升级为 bcrypt

### 1.4 当前样本数据（本地现状）

这不是迁移规则本身，只是当前仓库里看到的样本，方便判断风险：

- `user`: 1
- `user_config`: 1
- `item_icon_group`: 1
- `search_engine`: 4（其中 1 条已软删）
- `bookmark/item_icon/file/notepad/module_config/system_setting`: 当前样本基本为空
- `service/uploads/`: 当前样本为空目录

这说明：

- 当前仓库里的演示数据量很小
- 但正式迁移方案仍按“有真实数据、有上传文件”来设计，避免只适配样本

### 1.5 路径/运行方式差异

| 项 | 旧 YT-Panel | 新 YT-panel-Rust |
| --- | --- | --- |
| 配置格式 | INI | TOML |
| 数据库位置 | `service/database/database.db` | `artifacts/YT-panel-Rust/runtime/database/database.db` |
| 上传目录 | `service/uploads` | `artifacts/YT-panel-Rust/runtime/uploads` |
| 日志 | `service/runtime/runlog` | `artifacts/YT-panel-Rust/logs` |
| 临时目录 | `service/runtime/temp` | 当前 Rust 暂无等价强依赖 |
| 运行时产物位置 | 混在 repo/service 内 | 明确放 `artifacts/` |

---

## 2. 推荐迁移策略

推荐采用：**复制旧库 / 复制 uploads / 显式切换配置 / 短停机切换**。

不推荐：

- 直接让 Rust 进程读旧项目目录下的 `service/database` 和 `service/uploads`
- 在 repo 内继续写运行时数据库/上传文件
- 一边旧服务写库、一边新服务写同一份 SQLite

原因很直接：

- SQLite 不适合双写/并发切换
- uploads 路径要和 Rust 运行目录一致，才能满足 YT 的 artifacts 规则
- 复制迁移天然有回滚面，风险最低

---

## 3. 数据库迁移方案

### 3.1 迁移原则

优先方案：

1. 停旧服务写入
2. 复制旧 `database.db` 到新 runtime
3. 在新库里补齐 Rust 需要但旧库没有的 `notice` 表
4. 补齐缺失的默认 `system_setting`
5. 再启动 Rust 后端

因为当前 schema 基本同构，这比逐表导入更稳，也更少踩隐藏字段/索引问题。

### 3.2 已提供的最小迁移脚本

已新增脚本：`scripts/migrate_from_yt_panel.py`

它会做这些事：

- 校验旧库是否存在、表是否齐全
- 默认从 `projects/YT-Panel/service` 读旧数据
- 复制旧 SQLite 到 `artifacts/YT-panel-Rust/runtime/database/database.db`
- 复制旧 uploads 到 `artifacts/YT-panel-Rust/runtime/uploads`
- 自动补 `notice` 表
- 自动补缺失的：
  - `system_application`
  - `disclaimer`
  - `web_about_description`
  - `panel_public_user_id`
- 默认拒绝覆盖已有 Rust runtime；如确实要覆盖，需显式加 `--force`
- `--force` 时会先备份旧的 Rust runtime DB / uploads

### 3.3 推荐执行命令

先 dry-run：

```bash
cd /home/ytjungle/.openclaw/workspace/projects/YT-panel-Rust
python3 scripts/migrate_from_yt_panel.py --dry-run
```

确认无误后正式执行：

```bash
cd /home/ytjungle/.openclaw/workspace/projects/YT-panel-Rust
python3 scripts/migrate_from_yt_panel.py --force
```

如果你的实际路径不是当前 workspace 默认值，再显式传参：

```bash
python3 scripts/migrate_from_yt_panel.py \
  --old-service-root /path/to/YT-Panel/service \
  --new-runtime-root /path/to/artifacts/YT-panel-Rust/runtime \
  --force
```

### 3.4 数据库迁移后的预期

迁移完成后，新库至少应满足：

- `user`、`user_config`、`item_icon_group`、`item_icon`、`bookmark`、`search_engine`、`notepad`、`file`、`module_config` 数据已复制
- `notice` 表存在
- `system_setting` 至少包含：
  - `system_application`
  - `disclaimer`
  - `web_about_description`
  - `panel_public_user_id`

### 3.5 用户登录与会话影响

需要明确：

- **旧浏览器登录态不要指望无缝继承**
- 切换到 Rust 后，建议按“用户重新登录”处理

原因：

- 旧前端/旧服务的内存态 session 不会迁移
- Rust 端会沿用数据库中的持久 token，但当前真正对外返回的是新的 client token/session 映射

结论：**密码保留，登录态重建。**

---

## 4. uploads / 文件路径迁移方案

### 4.1 迁移原则

旧 uploads：

- 实际文件在 `service/uploads/...`
- 数据库 `file.src` 采用相对路径风格：`./uploads/...`

新 Rust 运行时：

- 实际文件必须放 `artifacts/YT-panel-Rust/runtime/uploads/...`
- 数据库存储仍保持 `./uploads/...` 这种**对前端兼容**的路径语义
- 实际磁盘路径由 Rust 配置 `uploads_dir` 决定

### 4.2 为什么要这样做

这样能同时满足：

1. 前端兼容旧接口返回值（`/uploads/...`）
2. 运行时文件不落回 repo
3. 切换后文件管理/删除逻辑可以稳定解析到 `artifacts/.../runtime/uploads`

### 4.3 执行方式

迁移脚本会直接把旧目录复制到新目录：

- `projects/YT-Panel/service/uploads` → `artifacts/YT-panel-Rust/runtime/uploads`

如果旧 uploads 为空，脚本也会保留目标目录结构。

### 4.4 风险点

如果生产旧实例里有人手改过 `source_path`，而不是默认的 `./uploads`，需要先确认：

- 旧数据库 `file.src` 是否仍然是 `./uploads/...`
- 真实文件是否也都在旧 `source_path` 下

若不是默认结构，迁移前要做一次路径抽样核对，不建议直接盲迁。

---

## 5. 配置迁移方案

### 5.1 旧配置来源

旧配置文件：`projects/YT-Panel/service/conf/conf.ini`

关键项：

```ini
[base]
http_port=3002
database_drive=sqlite
source_path=./uploads
source_temp_path=./runtime/temp

[sqlite]
file_path=./database/database.db
```

### 5.2 新配置目标

新配置文件：`projects/YT-panel-Rust/backend/config/app.toml`

示例：

```toml
host = "0.0.0.0"
port = 18080
database_url = "sqlite:///home/ytjungle/.openclaw/workspace/artifacts/YT-panel-Rust/runtime/database/database.db"
uploads_dir = "/home/ytjungle/.openclaw/workspace/artifacts/YT-panel-Rust/runtime/uploads"
frontend_dist = "/home/ytjungle/.openclaw/workspace/artifacts/YT-panel-Rust/frontend-dist"
max_upload_mb = 10
public_user_id = 1
```

### 5.3 配置映射建议

| 旧 INI | 新 TOML | 建议 |
| --- | --- | --- |
| `base.http_port` | `port` | 按切换计划决定，可沿用旧端口，也可先走新端口验证 |
| `sqlite.file_path` | `database_url` | 指向新 runtime DB，不要继续指向旧 repo 内数据库 |
| `base.source_path` | `uploads_dir` | 指向 `artifacts/YT-panel-Rust/runtime/uploads` |
| `base.source_temp_path` | 无直接等价 | 当前 Rust 无强依赖，可忽略；若后续需要，放 `artifacts/.../runtime/temp` |
| `database_drive/cache_drive/queue_drive/redis/mysql` | 当前 Rust 未使用 | 不迁移，除非后续 Rust 功能明确需要 |

### 5.4 建议配置动作

1. 保留一个 `backend/config/app.toml` 作为本机/生产配置
2. 只让它指向 `artifacts/YT-panel-Rust/runtime/...`
3. 不让 Rust 直接写旧项目目录

---

## 6. 切换步骤（推荐执行顺序）

### Phase A：切换前准备

1. **确认旧实例路径**
   - `conf/conf.ini`
   - `database/database.db`
   - `uploads/`
2. **备份旧实例**（至少 DB + uploads + conf）
3. **确保 Rust 前后端已构建完成**
4. **确保 Rust 的 `backend/config/app.toml` 指向 artifacts 路径**
5. **先用 dry-run 跑迁移脚本**

### Phase B：停机迁移

1. 停掉旧 YT-Panel 服务
2. 确认旧 SQLite 不再被写入
3. 执行迁移脚本：

```bash
cd /home/ytjungle/.openclaw/workspace/projects/YT-panel-Rust
python3 scripts/migrate_from_yt_panel.py --force
```

4. 启动 Rust 后端
5. 启动/发布 Rust 前端静态资源
6. 把反代或入口切到 Rust 服务

### Phase C：观察期

建议至少保留旧实例目录和备份 1~3 天，不立刻删：

- 旧 DB 保留
- 旧 uploads 保留
- 旧配置保留
- 旧服务保持停止或仅内网保留备用，不再对外写入

---

## 7. 回滚步骤

如果切换后发现问题，按下面回滚：

1. 停掉 Rust 服务
2. 恢复入口/反代到旧 YT-Panel
3. 启动旧服务
4. 若 Rust runtime 已写入新数据：
   - 不要把新库覆盖回旧库
   - 先保留 Rust runtime 作为现场
5. 如果只是迁移数据不对，可：
   - 恢复 `artifacts/YT-panel-Rust/runtime/database/database.db` 的备份
   - 恢复 `artifacts/YT-panel-Rust/runtime/uploads` 的备份
   - 修正后重新迁移

为什么回滚简单：

- 迁移脚本默认是 **copy**，不是 move
- 旧库/旧 uploads 不会在迁移过程中被改写

---

## 8. 验证清单（切换后必做）

### 8.1 基础连通

- [ ] `GET /ping` 返回 `pong`
- [ ] 首页能正常打开
- [ ] 静态资源加载正常

### 8.2 登录与用户

- [ ] 管理员可以登录
- [ ] `POST /api/user/getInfo` 正常
- [ ] 用户昵称、头像、角色正常
- [ ] 切换后旧登录态失效但重新登录正常

### 8.3 面板数据

- [ ] 图标分组数量一致
- [ ] 每个分组下项目数量一致
- [ ] `lanUrl` / `lanOnly` 行为正常
- [ ] 书签树结构一致
- [ ] 搜索引擎列表一致
- [ ] 用户面板配置（如 `logoText`、`footerHtml`）一致

### 8.4 uploads / 文件

- [ ] 文件管理页能看到历史文件
- [ ] 历史文件 URL 能打开
- [ ] 壁纸/图标引用正常显示
- [ ] 便签中的附件/图片能打开
- [ ] 任选一个测试文件执行删除，确认物理文件确实从 `artifacts/.../runtime/uploads` 移除
- [ ] 再上传一个新文件，确认新记录仍写到 `./uploads/...` 语义、实际文件落到 `artifacts/.../runtime/uploads`

### 8.5 系统设置

- [ ] `system_application` 可读
- [ ] `disclaimer` / `web_about_description` 正常
- [ ] 公开访问模式正常
- [ ] 若依赖 `panel_public_user_id`，公开模式读取的是正确用户

### 8.6 监控 / 其他

- [ ] CPU / 内存 / 磁盘监控接口可用
- [ ] 关于页可用
- [ ] 注册开关行为符合预期

---

## 9. 建议的上线口径

如果这是正式环境切换，建议对外按下面口径处理：

- 有短暂维护窗口
- 切换后需要重新登录
- 历史数据与上传文件会保留
- 若发现异常，可快速回滚到旧版

---

## 9.5 当前已补齐的部署工件

这轮 parity pass 已经把 Rust 版自己的部署骨架补上，不再需要继续借用旧 Go 项目的发布链路：

- `build.sh`
  - `./build.sh -f`：构建前端到 `artifacts/YT-panel-Rust/frontend-dist`
  - `./build.sh -b`：构建 Rust release 二进制到 `artifacts/YT-panel-Rust/backend-target`
  - `./build.sh -r`：打包 release 目录与 `tar.gz`
- `Dockerfile`
  - 多阶段构建前端 + Rust 后端
  - 默认运行目录 `/app/backend`
- `docker-compose.yml`
  - 默认挂载数据库与 uploads
  - 可按需覆盖 `/app/backend/config/app.toml`

所以当前状态已经从“只有迁移方案”推进到：

- **运行时数据怎么迁** 已明确
- **Rust 版怎么打包** 已明确
- **Rust 版怎么容器化部署** 已明确

剩下主要是目标环境的实机验证，而不是再补部署骨架。

---

## 10. 当前建议结论

**推荐直接采用“复制旧库 + 复制 uploads + TOML 指向 artifacts + 短停机切换”的方案。**

原因：

- 旧/新 SQLite schema 已经足够接近
- 风险远低于逐表手写转换
- 满足 YT 的 artifacts 目录规则
- 回滚简单
- 已有最小脚本可执行，不需要引入额外依赖
