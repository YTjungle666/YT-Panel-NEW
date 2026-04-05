use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use sysinfo::{Disks, System};

use crate::auth::{authenticate, AccessMode};
use crate::db::{get_module_config, get_setting, save_module_config, set_setting};
use crate::error::{list_ok, ok, ok_empty, ApiError, ApiResult};
use crate::state::AppState;

// ============================================================================
// 模块配置相关 API
// ============================================================================

/// 获取模块配置请求
#[derive(Debug, Deserialize)]
pub struct GetModuleConfigRequest {
    pub name: String,
}

/// 保存模块配置请求
#[derive(Debug, Deserialize)]
pub struct SaveModuleConfigRequest {
    pub name: String,
    pub value: serde_json::Value,
}

/// 获取模块配置
/// POST /api/system/moduleConfig/getByName
pub async fn get_module_config_by_name(
    State(state): State<Arc<AppState>>,
    Json(req): Json<GetModuleConfigRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    authenticate(&state, AccessMode::Admin).await?;
    
    let config = get_module_config(&state.db, &req.name).await?;
    match config {
        Some(value) => ok(Json(value)),
        None => ok(Json(serde_json::json!({}))),
    }
}

/// 保存模块配置
/// POST /api/system/moduleConfig/save
pub async fn save_module_config_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SaveModuleConfigRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    authenticate(&state, AccessMode::Admin).await?;
    
    save_module_config(&state.db, &req.name, &req.value).await?;
    ok_empty()
}

// ============================================================================
// 系统设置相关 API
// ============================================================================

/// 设置系统配置请求
#[derive(Debug, Deserialize)]
pub struct SetSettingRequest {
    pub key: String,
    pub value: String,
}

/// 获取系统配置请求
#[derive(Debug, Deserialize)]
pub struct GetSettingRequest {
    pub key: String,
}

/// 获取单个配置请求
#[derive(Debug, Deserialize)]
pub struct GetSingleSettingRequest {
    pub key: String,
}

/// 设置系统配置
/// POST /api/system/setting/set
pub async fn set_setting_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SetSettingRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    authenticate(&state, AccessMode::Admin).await?;
    
    set_setting(&state.db, &req.key, &req.value).await?;
    ok_empty()
}

/// 获取系统配置
/// POST /api/system/setting/get
pub async fn get_setting_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<GetSettingRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    authenticate(&state, AccessMode::Admin).await?;
    
    let settings = get_setting(&state.db, &req.key).await?;
    ok(Json(settings))
}

/// 获取单个配置
/// POST /api/system/setting/getSingle
pub async fn get_single_setting(
    State(state): State<Arc<AppState>>,
    Json(req): Json<GetSingleSettingRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    authenticate(&state, AccessMode::Admin).await?;
    
    let value = get_setting(&state.db, &req.key).await?;
    ok(Json(value))
}

// ============================================================================
// 系统监控相关 API
// ============================================================================

/// CPU 信息
#[derive(Debug, Serialize)]
pub struct CpuInfo {
    pub usage: f32,
    pub name: String,
    pub cores: usize,
    pub frequency: u64,
}

/// 内存信息
#[derive(Debug, Serialize)]
pub struct MemoryInfo {
    pub total: u64,
    pub used: u64,
    pub free: u64,
    pub usage: f32,
}

/// 磁盘信息
#[derive(Debug, Serialize)]
pub struct DiskInfo {
    pub path: String,
    pub mount_point: String,
    pub total: u64,
    pub used: u64,
    pub free: u64,
    pub usage: f32,
    pub file_system: String,
}

/// 系统状态信息
#[derive(Debug, Serialize)]
pub struct SystemStatus {
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub disks: Vec<DiskInfo>,
    pub uptime: u64,
    pub hostname: String,
}

/// 获取磁盘状态请求
#[derive(Debug, Deserialize)]
pub struct GetDiskStateRequest {
    pub path: String,
}

/// 获取系统状态
/// POST /api/system/monitor/getAll
pub async fn get_system_status(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<SystemStatus>> {
    authenticate(&state, AccessMode::Admin).await?;
    
    let mut sys = System::new_all();
    sys.refresh_all();
    
    // CPU 信息
    let cpu_usage = sys.global_cpu_usage();
    let cpu_info = CpuInfo {
        usage: cpu_usage,
        name: sys.cpus().first().map(|c| c.brand().to_string()).unwrap_or_default(),
        cores: sys.cpus().len(),
        frequency: sys.cpus().first().map(|c| c.frequency()).unwrap_or(0),
    };
    
    // 内存信息
    let total_memory = sys.total_memory();
    let used_memory = sys.used_memory();
    let memory_info = MemoryInfo {
        total: total_memory,
        used: used_memory,
        free: total_memory.saturating_sub(used_memory),
        usage: if total_memory > 0 {
            (used_memory as f32 / total_memory as f32) * 100.0
        } else {
            0.0
        },
    };
    
    // 磁盘信息
    let disks: Vec<DiskInfo> = Disks::new_with_refreshed_list()
        .iter()
        .map(|disk| {
            let total = disk.total_space();
            let free = disk.available_space();
            let used = total.saturating_sub(free);
            DiskInfo {
                path: disk.name().to_string_lossy().to_string(),
                mount_point: disk.mount_point().to_string_lossy().to_string(),
                total,
                used,
                free,
                usage: if total > 0 {
                    (used as f32 / total as f32) * 100.0
                } else {
                    0.0
                },
                file_system: disk.file_system().to_string_lossy().to_string(),
            }
        })
        .collect();
    
    let status = SystemStatus {
        cpu: cpu_info,
        memory: memory_info,
        disks,
        uptime: System::uptime(),
        hostname: System::host_name().unwrap_or_default(),
    };
    
    ok(Json(status))
}

/// 获取 CPU 状态
/// POST /api/system/monitor/getCpuState
pub async fn get_cpu_state(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<CpuInfo>> {
    authenticate(&state, AccessMode::Admin).await?;
    
    let mut sys = System::new_all();
    sys.refresh_cpu();
    
    let cpu_usage = sys.global_cpu_usage();
    let cpu_info = CpuInfo {
        usage: cpu_usage,
        name: sys.cpus().first().map(|c| c.brand().to_string()).unwrap_or_default(),
        cores: sys.cpus().len(),
        frequency: sys.cpus().first().map(|c| c.frequency()).unwrap_or(0),
    };
    
    ok(Json(cpu_info))
}

/// 获取内存状态
/// POST /api/system/monitor/getMemonyState
pub async fn get_memory_state(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<MemoryInfo>> {
    authenticate(&state, AccessMode::Admin).await?;
    
    let mut sys = System::new_all();
    sys.refresh_memory();
    
    let total_memory = sys.total_memory();
    let used_memory = sys.used_memory();
    let memory_info = MemoryInfo {
        total: total_memory,
        used: used_memory,
        free: total_memory.saturating_sub(used_memory),
        usage: if total_memory > 0 {
            (used_memory as f32 / total_memory as f32) * 100.0
        } else {
            0.0
        },
    };
    
    ok(Json(memory_info))
}

/// 获取磁盘状态
/// POST /api/system/monitor/getDiskStateByPath
pub async fn get_disk_state_by_path(
    State(state): State<Arc<AppState>>,
    Json(req): Json<GetDiskStateRequest>,
) -> ApiResult<Json<Option<DiskInfo>>> {
    authenticate(&state, AccessMode::Admin).await?;
    
    let disks = Disks::new_with_refreshed_list();
    let disk_info = disks.iter().find(|d| {
        d.mount_point().to_string_lossy() == req.path
    }).map(|disk| {
        let total = disk.total_space();
        let free = disk.available_space();
        let used = total.saturating_sub(free);
        DiskInfo {
            path: disk.name().to_string_lossy().to_string(),
            mount_point: disk.mount_point().to_string_lossy().to_string(),
            total,
            used,
            free,
            usage: if total > 0 {
                (used as f32 / total as f32) * 100.0
            } else {
                0.0
            },
            file_system: disk.file_system().to_string_lossy().to_string(),
        }
    });
    
    ok(Json(disk_info))
}

/// 获取磁盘挂载点
/// POST /api/system/monitor/getDiskMountpoints
pub async fn get_disk_mountpoints(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<Vec<String>>> {
    authenticate(&state, AccessMode::Admin).await?;
    
    let disks = Disks::new_with_refreshed_list();
    let mountpoints: Vec<String> = disks
        .iter()
        .map(|d| d.mount_point().to_string_lossy().to_string())
        .collect();
    
    list_ok(mountpoints)
}

// ============================================================================
// 开放接口
// ============================================================================

/// 登录配置
#[derive(Debug, Serialize)]
pub struct LoginConfig {
    pub need_verify: bool,
    pub login_type: String,
}

/// 获取登录配置
/// GET /api/openness/loginConfig
pub async fn get_login_config(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<LoginConfig>> {
    let config = LoginConfig {
        need_verify: false,
        login_type: "password".to_string(),
    };
    ok(Json(config))
}

/// 免责声明响应
#[derive(Debug, Serialize)]
pub struct DisclaimerResponse {
    pub disclaimer: String,
}

/// 获取免责声明
/// GET /api/openness/getDisclaimer
pub async fn get_disclaimer(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<DisclaimerResponse>> {
    let disclaimer = get_setting(&state.db, "disclaimer").await.ok().and_then(|v| v.as_str().map(|s| s.to_string())).unwrap_or_default();
    ok(Json(DisclaimerResponse { disclaimer }))
}

/// 关于描述响应
#[derive(Debug, Serialize)]
pub struct AboutDescriptionResponse {
    pub description: String,
}

/// 获取关于描述
/// GET /api/openness/getAboutDescription
pub async fn get_about_description(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<AboutDescriptionResponse>> {
    let description = get_setting(&state.db, "about_description").await.ok().and_then(|v| v.as_str().map(|s| s.to_string())).unwrap_or_default();
    ok(Json(AboutDescriptionResponse { description }))
}

// ============================================================================
// 其他接口
// ============================================================================

/// 局域网检查响应
#[derive(Debug, Serialize)]
pub struct LanCheckResponse {
    pub is_lan: bool,
}

/// 检查是否局域网
/// GET /api/isLan
pub async fn is_lan(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<LanCheckResponse>> {
    // 简化实现，实际应该根据请求 IP 判断
    ok(Json(LanCheckResponse { is_lan: true }))
}

/// 关于信息请求
#[derive(Debug, Deserialize)]
pub struct AboutRequest {
    #[serde(default)]
    pub detail: bool,
}

/// 关于信息响应
#[derive(Debug, Serialize)]
pub struct AboutResponse {
    pub name: String,
    pub version: String,
    pub description: String,
    pub build_time: String,
    pub git_commit: String,
}

/// 获取关于信息
/// POST /api/about
pub async fn about(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AboutRequest>,
) -> ApiResult<Json<AboutResponse>> {
    let resp = AboutResponse {
        name: "YT-Panel".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        description: "YT Panel 管理面板".to_string(),
        build_time: option_env!("BUILD_TIME").unwrap_or("unknown").to_string(),
        git_commit: option_env!("GIT_COMMIT").unwrap_or("unknown").to_string(),
    };
    ok(Json(resp))
}

/// 健康检查
/// GET /ping
pub async fn ping() -> &'static str {
    "pong"
}

// ============================================================================
// 路由
// ============================================================================

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // 模块配置
        .route("/api/system/moduleConfig/getByName", post(get_module_config_by_name))
        .route("/api/system/moduleConfig/save", post(save_module_config_handler))
        // 系统设置
        .route("/api/system/setting/set", post(set_setting_handler))
        .route("/api/system/setting/get", post(get_setting_handler))
        .route("/api/system/setting/getSingle", post(get_single_setting))
        // 系统监控
        .route("/api/system/monitor/getAll", post(get_system_status))
        .route("/api/system/monitor/getCpuState", post(get_cpu_state))
        .route("/api/system/monitor/getMemonyState", post(get_memory_state))
        .route("/api/system/monitor/getDiskStateByPath", post(get_disk_state_by_path))
        .route("/api/system/monitor/getDiskMountpoints", post(get_disk_mountpoints))
        // 开放接口
        .route("/api/openness/loginConfig", get(get_login_config))
        .route("/api/openness/getDisclaimer", get(get_disclaimer))
        .route("/api/openness/getAboutDescription", get(get_about_description))
        // 其他
        .route("/api/isLan", get(is_lan))
        .route("/api/about", post(about))
        .route("/ping", get(ping))
}
