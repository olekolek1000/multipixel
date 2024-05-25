#![allow(dead_code)]

use std::error::Error;

const SETTINGS_PATH: &str = "settings.json";

#[derive(serde::Deserialize)]
pub struct PreviewSystem {
    process_all_at_start: bool,
}

#[derive(serde::Deserialize)]
pub struct Config {
    pub listen_ip: String,
    pub listen_port: u16,
    pub autosave_interval_ms: u32,
    pub plugin_list: Vec<String>,
    pub preview_system: PreviewSystem,
}

pub async fn load() -> Result<Config, Box<dyn Error>> {
    let data = tokio::fs::read_to_string(SETTINGS_PATH).await?;
    let conf: Config = serde_json::from_str(data.as_str())?;
    Ok(conf)
}
