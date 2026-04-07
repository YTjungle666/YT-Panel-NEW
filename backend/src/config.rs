use std::env;
use std::path::PathBuf;

use crate::models::AppConfig;

pub async fn load_config() -> anyhow::Result<AppConfig> {
    let mut candidates = Vec::<PathBuf>::new();
    if let Some(path) = env::var("YT_PANEL_CONFIG")
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        candidates.push(PathBuf::from(path));
    }
    candidates.push(PathBuf::from("config/app.toml"));
    candidates.push(PathBuf::from("config/example.toml"));

    for path in candidates {
        if path.exists() {
            let content = tokio::fs::read_to_string(path).await?;
            return Ok(toml::from_str(&content).unwrap_or_default());
        }
    }

    Ok(AppConfig::default())
}
