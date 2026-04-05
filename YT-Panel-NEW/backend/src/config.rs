use serde::{Deserialize, Serialize};
use std::env;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub uploads_dir: String,
    pub frontend_dist: String,
    pub max_upload_mb: u64,
    pub public_user_id: Option<i64>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".into(),
            port: 80,
            database_url: "sqlite://./database/database.db".into(),
            uploads_dir: "./uploads".into(),
            frontend_dist: "../frontend-dist".into(),
            max_upload_mb: 10,
            public_user_id: Some(1),
        }
    }
}

impl AppConfig {
    pub async fn load() -> anyhow::Result<Self> {
        let mut candidates = Vec::<PathBuf>::new();
        if let Some(path) = env::var("YT_PANEL_CONFIG").ok().filter(|v| !v.trim().is_empty()) {
            candidates.push(PathBuf::from(path));
        }
        candidates.push(PathBuf::from("config/app.toml"));
        candidates.push(PathBuf::from("config/example.toml"));

        for path in candidates {
            if path.exists() {
                let content = tokio::fs::read_to_string(&path).await?;
                let config: AppConfig = toml::from_str(&content)?;
                return Ok(config);
            }
        }
        Ok(AppConfig::default())
    }

    pub async fn ensure_dirs(&self) -> anyhow::Result<()> {
        tokio::fs::create_dir_all(&self.uploads_dir).await?;
        
        if let Some(path) = extract_db_path(&self.database_url) {
            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            if tokio::fs::metadata(&path).await.is_err() {
                tokio::fs::File::create(&path).await?;
            }
        }
        Ok(())
    }
}

fn extract_db_path(database_url: &str) -> Option<PathBuf> {
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
