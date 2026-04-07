use std::{collections::HashMap, sync::Arc};

use axum::{
    http::{
        header::{AUTHORIZATION, CONTENT_TYPE},
        HeaderName, HeaderValue, Method,
    },
    middleware,
    routing::{get, post},
    Router,
};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing::info;
use yt_panel_rust_backend::{
    auth::enforce_password_change_middleware,
    db::{ensure_parent_dirs, init_db, seed_defaults},
    handlers::{
        auth as auth_handlers, file as file_handlers, panel as panel_handlers,
        search as search_handlers, system as system_handlers, user as user_handlers,
    },
    models::{AppConfig, AppState},
};

fn build_cors_layer(config: &AppConfig) -> anyhow::Result<CorsLayer> {
    let base = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            CONTENT_TYPE,
            AUTHORIZATION,
            HeaderName::from_static("lang"),
        ])
        .allow_credentials(true);

    if config.cors_allowed_origins.is_empty() {
        return Ok(base);
    }

    let origins = config
        .cors_allowed_origins
        .iter()
        .map(|origin| HeaderValue::from_str(origin))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(base.allow_origin(AllowOrigin::list(origins)))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = Arc::new(yt_panel_rust_backend::config::load_config().await?);
    ensure_parent_dirs(&config).await?;

    let db = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(10)
        .min_connections(2)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(&config.database_url)
        .await?;

    init_db(&db).await?;
    seed_defaults(&db, &config).await?;

    let state = AppState {
        db,
        config: config.clone(),
        auth_cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
    };

    let frontend_dir = std::path::PathBuf::from(&config.frontend_dist);
    let frontend_index = frontend_dir.join("index.html");
    let cors = build_cors_layer(&config)?;

    let api = Router::new()
        .route("/api/login", post(auth_handlers::login))
        .route("/api/crypto-key", get(auth_handlers::get_crypto_key))
        .route("/api/logout", post(auth_handlers::logout))
        .route("/api/register/commit", post(auth_handlers::register_commit))
        .route("/api/about", post(auth_handlers::about))
        .route("/api/isLan", get(auth_handlers::is_lan))
        .route("/api/user/getInfo", post(user_handlers::user_get_info))
        .route("/api/user/getAuthInfo", post(user_handlers::user_get_auth_info))
        .route("/api/user/updateInfo", post(user_handlers::user_update_info))
        .route("/api/user/updatePassword", post(user_handlers::user_update_password))
        .route("/api/user/getReferralCode", post(user_handlers::user_get_referral_code))
        .route(
            "/api/notice/getListByDisplayType",
            post(system_handlers::notice_get_list),
        )
        .route(
            "/api/system/moduleConfig/getByName",
            post(system_handlers::module_config_get),
        )
        .route(
            "/api/system/moduleConfig/save",
            post(system_handlers::module_config_save),
        )
        .route(
            "/api/system/setting/set",
            post(system_handlers::system_setting_set),
        )
        .route(
            "/api/system/setting/get",
            post(system_handlers::system_setting_get),
        )
        .route(
            "/api/system/setting/getSingle",
            post(system_handlers::system_setting_get_single),
        )
        .route(
            "/api/system/monitor/getAll",
            post(system_handlers::system_monitor_get_all),
        )
        .route(
            "/api/system/monitor/getCpuState",
            post(system_handlers::system_monitor_get_cpu),
        )
        .route(
            "/api/system/monitor/getMemonyState",
            post(system_handlers::system_monitor_get_memory),
        )
        .route(
            "/api/system/monitor/getDiskStateByPath",
            post(system_handlers::system_monitor_get_disk),
        )
        .route(
            "/api/system/monitor/getDiskMountpoints",
            post(system_handlers::system_monitor_get_mountpoints),
        )
        .route(
            "/api/openness/loginConfig",
            get(system_handlers::openness_login_config),
        )
        .route(
            "/api/openness/getDisclaimer",
            get(system_handlers::openness_get_disclaimer),
        )
        .route(
            "/api/openness/getAboutDescription",
            get(system_handlers::openness_get_about_description),
        )
        .route("/api/file/uploadImg", post(file_handlers::file_upload_img))
        .route("/api/file/uploadFiles", post(file_handlers::file_upload_files))
        .route("/api/file/getList", post(file_handlers::file_get_list))
        .route("/api/file/deletes", post(file_handlers::file_deletes))
        .route(
            "/api/panel/userConfig/get",
            post(panel_handlers::panel_user_config_get),
        )
        .route(
            "/api/panel/userConfig/set",
            post(panel_handlers::panel_user_config_set),
        )
        .route("/api/panel/users/create", post(user_handlers::panel_users_create))
        .route("/api/panel/users/update", post(user_handlers::panel_users_update))
        .route(
            "/api/panel/users/getList",
            post(user_handlers::panel_users_get_list),
        )
        .route(
            "/api/panel/users/deletes",
            post(user_handlers::panel_users_deletes),
        )
        .route(
            "/api/panel/users/getPublicVisitUser",
            post(user_handlers::panel_users_get_public_visit_user),
        )
        .route(
            "/api/panel/users/setPublicVisitUser",
            post(user_handlers::panel_users_set_public_visit_user),
        )
        .route(
            "/api/panel/itemIconGroup/getList",
            post(panel_handlers::panel_item_icon_group_get_list),
        )
        .route(
            "/api/panel/itemIconGroup/edit",
            post(panel_handlers::panel_item_icon_group_edit),
        )
        .route(
            "/api/panel/itemIconGroup/deletes",
            post(panel_handlers::panel_item_icon_group_deletes),
        )
        .route(
            "/api/panel/itemIconGroup/saveSort",
            post(panel_handlers::panel_item_icon_group_save_sort),
        )
        .route(
            "/api/panel/itemIcon/getListByGroupId",
            post(panel_handlers::panel_item_icon_get_list_by_group_id),
        )
        .route(
            "/api/panel/itemIcon/edit",
            post(panel_handlers::panel_item_icon_edit),
        )
        .route(
            "/api/panel/itemIcon/addMultiple",
            post(panel_handlers::panel_item_icon_add_multiple),
        )
        .route(
            "/api/panel/itemIcon/deletes",
            post(panel_handlers::panel_item_icon_deletes),
        )
        .route(
            "/api/panel/itemIcon/saveSort",
            post(panel_handlers::panel_item_icon_save_sort),
        )
        .route(
            "/api/panel/itemIcon/getSiteFavicon",
            post(panel_handlers::panel_item_icon_get_site_favicon),
        )
        .route(
            "/api/panel/bookmark/getList",
            post(panel_handlers::panel_bookmark_get_list),
        )
        .route("/api/panel/bookmark/add", post(panel_handlers::panel_bookmark_add))
        .route(
            "/api/panel/bookmark/addMultiple",
            post(panel_handlers::panel_bookmark_add_multiple),
        )
        .route(
            "/api/panel/bookmark/update",
            post(panel_handlers::panel_bookmark_update),
        )
        .route(
            "/api/panel/bookmark/deletes",
            post(panel_handlers::panel_bookmark_deletes),
        )
        .route("/api/panel/notepad/get", get(panel_handlers::panel_notepad_get))
        .route(
            "/api/panel/notepad/getList",
            get(panel_handlers::panel_notepad_get_list),
        )
        .route("/api/panel/notepad/save", post(panel_handlers::panel_notepad_save))
        .route(
            "/api/panel/notepad/delete",
            post(panel_handlers::panel_notepad_delete),
        )
        .route(
            "/api/panel/notepad/upload",
            post(panel_handlers::panel_notepad_upload),
        )
        .route(
            "/api/panel/searchEngine/getList",
            post(panel_handlers::panel_search_engine_get_list),
        )
        .route(
            "/api/panel/searchEngine/add",
            post(panel_handlers::panel_search_engine_add),
        )
        .route(
            "/api/panel/searchEngine/update",
            post(panel_handlers::panel_search_engine_update),
        )
        .route(
            "/api/panel/searchEngine/delete",
            post(panel_handlers::panel_search_engine_delete),
        )
        .route(
            "/api/panel/searchEngine/updateSort",
            post(panel_handlers::panel_search_engine_update_sort),
        )
        .route(
            "/api/search/bookmarks",
            get(search_handlers::search_bookmarks),
        )
        .route(
            "/api/search/suggestions",
            get(search_handlers::search_suggestions),
        )
        .route("/ping", get(auth_handlers::ping))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            enforce_password_change_middleware,
        ))
        .with_state(state.clone());

    let app = api
        .nest_service("/uploads", ServeDir::new(config.uploads_dir.clone()))
        .fallback_service(ServeDir::new(frontend_dir).fallback(ServeFile::new(frontend_index)))
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("YT-panel-Rust backend listening on {}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}
