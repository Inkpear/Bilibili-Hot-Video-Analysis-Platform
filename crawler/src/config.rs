use serde::Deserialize;

use crate::error::{CrawlerError, Result};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub crawler: CrawlerConfig,
    pub api: ApiConfig,
    pub limits: LimitConfig,
    #[serde(default)]
    pub accounts: Vec<AccountConfig>,
}

#[derive(Debug, Deserialize)]
pub struct CrawlerConfig {
    pub trigger_hour: u32,
    pub full_crawl_timeout_secs: u64,
    pub concurrency: usize,
    pub output_dir: String,
    pub db_path: String,
    #[serde(default)]
    pub delay: DelayConfig,
}

#[derive(Debug, Deserialize)]
pub struct DelayConfig {
    pub min_ms: u64,
    pub max_ms: u64,
}

impl Default for DelayConfig {
    fn default() -> Self {
        Self {
            min_ms: 500,
            max_ms: 2000,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ApiConfig {
    pub base_url: String,
    pub wbi_cache_ttl_secs: u64,
}

#[derive(Debug, Deserialize)]
pub struct LimitConfig {
    pub max_comments_per_video: usize,
    pub max_danmaku_per_video: usize,
    pub max_retries_per_task: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AccountConfig {
    pub sessdata: String,
    pub bili_jct: String,
    pub dede_user_id: String,
    #[serde(default)]
    pub proxy: Option<String>,
    #[serde(default)]
    pub user_agent: Option<String>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let content = std::fs::read_to_string("config.toml").map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                CrawlerError::Config("config.toml not found".to_string())
            } else {
                CrawlerError::Io(e)
            }
        })?;

        let mut config: Config =
            toml::from_str(&content).map_err(|e| CrawlerError::Config(e.to_string()))?;

        config.validate()?;
        config.expand_tilde()?;

        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        if self.crawler.trigger_hour >= 24 {
            return Err(CrawlerError::Config(format!(
                "trigger_hour must be < 24, got {}",
                self.crawler.trigger_hour
            )));
        }
        if self.crawler.concurrency == 0 {
            return Err(CrawlerError::Config("concurrency must be > 0".to_string()));
        }
        if self.accounts.is_empty() {
            return Err(CrawlerError::Config(
                "at least one [[accounts]] entry is required".to_string(),
            ));
        }
        if self.crawler.delay.min_ms > self.crawler.delay.max_ms {
            return Err(CrawlerError::Config(
                "delay.min_ms must be <= delay.max_ms".to_string(),
            ));
        }
        Ok(())
    }

    fn expand_tilde(&mut self) -> Result<()> {
        if self.crawler.output_dir.starts_with('~') {
            let home = std::env::var("HOME").unwrap_or_default();
            self.crawler.output_dir = home + &self.crawler.output_dir[1..];
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config(trigger_hour: u32, concurrency: usize) -> Config {
        Config {
            crawler: CrawlerConfig {
                trigger_hour,
                full_crawl_timeout_secs: 7200,
                concurrency,
                output_dir: "~/output".to_string(),
                db_path: "crawler_state.db".to_string(),
                delay: DelayConfig {
                    min_ms: 500,
                    max_ms: 2000,
                },
            },
            api: ApiConfig {
                base_url: "https://api.bilibili.com".to_string(),
                wbi_cache_ttl_secs: 14400,
            },
            limits: LimitConfig {
                max_comments_per_video: 200,
                max_danmaku_per_video: 3000,
                max_retries_per_task: 3,
            },
            accounts: vec![AccountConfig {
                sessdata: "test".to_string(),
                bili_jct: "test".to_string(),
                dede_user_id: "123".to_string(),
                proxy: None,
                user_agent: None,
            }],
        }
    }

    #[test]
    fn test_validate_trigger_hour_too_high() {
        let config = make_config(24, 5);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_concurrency_zero() {
        let config = make_config(0, 0);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_empty_accounts() {
        let mut config = make_config(0, 5);
        config.accounts = vec![];
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_delay_range() {
        let mut config = make_config(0, 5);
        config.crawler.delay.min_ms = 3000;
        config.crawler.delay.max_ms = 1000;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_expand_tilde() {
        let mut config = make_config(0, 5);
        config.expand_tilde().unwrap();
        assert!(!config.crawler.output_dir.contains('~'));
    }

    #[test]
    fn test_toml_round_trip() {
        let toml_str = r#"
[crawler]
trigger_hour = 0
full_crawl_timeout_secs = 7200
concurrency = 5
output_dir = "~/output"
db_path = "crawler_state.db"

[crawler.delay]
min_ms = 500
max_ms = 2000

[api]
base_url = "https://api.bilibili.com"
wbi_cache_ttl_secs = 14400

[limits]
max_comments_per_video = 200
max_danmaku_per_video = 3000
max_retries_per_task = 3

[[accounts]]
sessdata = "test"
bili_jct = "test"
dede_user_id = "123"
"#;
        let config: Config = toml::from_str(toml_str).expect("valid TOML");
        assert_eq!(config.crawler.trigger_hour, 0);
        assert_eq!(config.crawler.concurrency, 5);
        assert_eq!(config.api.base_url, "https://api.bilibili.com");
        assert_eq!(config.limits.max_comments_per_video, 200);
        assert_eq!(config.accounts.len(), 1);
        assert_eq!(config.accounts[0].sessdata, "test");
    }
}
