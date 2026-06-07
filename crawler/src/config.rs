use serde::Deserialize;
use anyhow::Result;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub crawler: CrawlerConfig,
    pub api: ApiConfig,
}

#[derive(Debug, Deserialize)]
pub struct CrawlerConfig {
    pub interval: u64,
    pub concurrency: usize,
    pub max_retries: u32,
    pub output_dir: String,
}

#[derive(Debug, Deserialize)]
pub struct ApiConfig {
    pub base_url: String,
    pub user_agent: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        let content = std::fs::read_to_string("config.toml")?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}
