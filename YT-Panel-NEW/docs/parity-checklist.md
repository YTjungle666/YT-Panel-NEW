# YT-panel-Rust 功能迁移清单

## 核心目标

- [x] 新建独立项目目录 `projects/YT-panel-Rust`
- [x] 复制现有前端源码作为迁移基线
- [x] 建立 Rust 后端骨架
- [x] 本机编译通过
- [x] 联调前端（基础烟雾测试已通过）
- [x] 输出旧 `YT-Panel` → Rust 版迁移方案（`docs/migration-plan.md`）
- [x] 补齐独立部署链路（`build.sh` / `Dockerfile` / `docker-compose.yml`）

## API 兼容面

### system
- [x] `POST /api/login`
- [x] `POST /api/logout`
- [x] `POST /api/login/sendResetPasswordVCode`（当前显式返回“未配置邮件重置能力”）
- [x] `POST /api/login/resetPasswordByVCode`（当前显式返回“未配置邮件重置能力”）
- [x] `POST /api/register/commit`
- [x] `POST /api/user/getInfo`
- [x] `POST /api/user/getAuthInfo`
- [x] `POST /api/user/updateInfo`
- [x] `POST /api/user/updatePassword`
- [x] `POST /api/user/getReferralCode`
- [x] `POST /api/notice/getListByDisplayType`
- [x] `POST /api/system/moduleConfig/getByName`
- [x] `POST /api/system/moduleConfig/save`
- [x] `POST /api/system/setting/set`
- [x] `POST /api/system/setting/get`
- [x] `POST /api/system/setting/getSingle`
- [x] `POST /api/system/monitor/getAll`
- [x] `POST /api/system/monitor/getCpuState`
- [x] `POST /api/system/monitor/getMemonyState`
- [x] `POST /api/system/monitor/getDiskStateByPath`
- [x] `POST /api/system/monitor/getDiskMountpoints`
- [x] `POST /api/about`
- [x] `GET /api/isLan`
- [x] `GET /ping`
- [x] `POST /api/file/uploadImg`
- [x] `POST /api/file/uploadFiles`
- [x] `POST /api/file/getList`
- [x] `POST /api/file/deletes`

### openness
- [x] `GET /api/openness/loginConfig`
- [x] `GET /api/openness/getDisclaimer`
- [x] `GET /api/openness/getAboutDescription`

### panel
- [x] `POST /api/panel/userConfig/get`
- [x] `POST /api/panel/userConfig/set`
- [x] `POST /api/panel/users/create`
- [x] `POST /api/panel/users/update`
- [x] `POST /api/panel/users/getList`
- [x] `POST /api/panel/users/deletes`
- [x] `POST /api/panel/users/getPublicVisitUser`
- [x] `POST /api/panel/users/setPublicVisitUser`
- [x] `POST /api/panel/itemIconGroup/getList`
- [x] `POST /api/panel/itemIconGroup/edit`
- [x] `POST /api/panel/itemIconGroup/deletes`
- [x] `POST /api/panel/itemIconGroup/saveSort`
- [x] `POST /api/panel/itemIcon/getListByGroupId`
- [x] `POST /api/panel/itemIcon/edit`
- [x] `POST /api/panel/itemIcon/addMultiple`
- [x] `POST /api/panel/itemIcon/deletes`
- [x] `POST /api/panel/itemIcon/saveSort`
- [x] `POST /api/panel/itemIcon/getSiteFavicon`
- [x] `POST /api/panel/bookmark/getList`
- [x] `POST /api/panel/bookmark/add`
- [x] `POST /api/panel/bookmark/addMultiple`
- [x] `POST /api/panel/bookmark/update`
- [x] `POST /api/panel/bookmark/deletes`
- [x] `GET /api/panel/notepad/get`
- [x] `GET /api/panel/notepad/getList`
- [x] `POST /api/panel/notepad/save`
- [x] `POST /api/panel/notepad/delete`
- [x] `POST /api/panel/notepad/upload`
- [x] `POST /api/panel/searchEngine/getList`
- [x] `POST /api/panel/searchEngine/add`
- [x] `POST /api/panel/searchEngine/update`
- [x] `POST /api/panel/searchEngine/delete`
- [x] `POST /api/panel/searchEngine/updateSort`

## 已知缺口

- 已安装系统级 Go 1.24 / Rust 1.85，并完成本机构建验证。
- 当前前端 `src/api/` 中实际使用到的主要接口，Rust 后端已全部补齐；之前缺失的管理员用户管理接口（`/api/panel/users/*`）已在本轮补上。
- 已补齐 `POST /api/register/commit`，并恢复前端 `/register` 路由与登录页入口；当前注册为“开放注册 + 邮箱格式/后缀校验”模式，不含邮件验证码。
- `GET /api/openness/loginConfig` 已按旧 Go 结构返回 `register: { openRegister, emailSuffix }`，同时兼容旧布尔值配置。
- 旧 `YT-Panel` 真实状态：
  - 前端定义了 `sendResetPasswordVCode` / `resetPasswordByVCode`，邮件库里也有发送验证码函数。
  - 但 `service/router/system/login.go` 实际只注册了 `/login` 与 `/logout`，找回密码链路并未真正接通。
  - 旧 Go 登录页里图形验证码 UI 也处于注释状态。
- Rust 版已补 `/reset-password` 页面入口，并补上 `POST /api/login/sendResetPasswordVCode` / `POST /api/login/resetPasswordByVCode` 两个契约端点；当前行为是**显式拒绝并说明“未配置邮件重置能力”**，不伪造验证码发送成功。
- 仍未补：SMTP 配置读取、验证码存储/过期/频控、邮件模板接入、真正可用的 forgot/reset 闭环。
- 公开访问模式现在已支持：
  - 管理员设置某个用户为访客模式账号
  - 管理员显式关闭公开访问（`panel_public_user_id = null`）
  - 删除公开访问账号后自动回落到剩余账号，避免悬空配置
- 当前已完成验证：
  - `cargo build`
  - `npm run build`
  - `./build.sh -r`
  - 临时 runtime 下的手工 smoke test：`/ping`、`/api/login`、`/api/panel/users/getList`、`/api/panel/users/create`、`/api/panel/users/update`、`/api/panel/users/deletes`、`/api/panel/users/getPublicVisitUser`、`/api/panel/users/setPublicVisitUser`、`/api/user/getAuthInfo`（公开模式开启/关闭两种状态）
- 仍建议补的验证：
  - `docker build` / `docker compose up` 实机验证
  - `POST /api/register/commit`（需先打开 `system_application.register.openRegister=true`）
  - `POST /api/login/sendResetPasswordVCode` / `POST /api/login/resetPasswordByVCode`（应持续返回“未配置邮件重置能力”）
- 书签 HTML 原始导入路径仍建议优先走前端解析后再提交 JSON。
