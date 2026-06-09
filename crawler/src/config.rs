use serde::Deserialize;

use crate::error::{CrawlerError, Result};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub crawler: CrawlerConfig,
    pub api: ApiConfig,
    pub partitions: PartitionConfig,
    pub limits: LimitConfig,
}

#[derive(Debug, Deserialize)]
pub struct CrawlerConfig {
    pub trigger_hour: u32,
    pub full_crawl_timeout_secs: u64,
    pub increment_interval_secs: u64,
    pub increment_stop_hour: u32,
    pub concurrency: usize,
    pub max_retries: u32,
    pub request_delay_ms: u64,
    pub output_dir: String,
    pub db_path: String,
}

#[derive(Debug, Deserialize)]
pub struct ApiConfig {
    pub base_url: String,
    pub user_agent: String,
    pub cookie_env_name: String,
    pub wbi_cache_ttl_secs: u64,
}

#[derive(Debug, Deserialize)]
pub struct PartitionConfig {
    pub rids: Vec<u32>,
}

#[derive(Debug, Deserialize)]
pub struct LimitConfig {
    pub max_comments_per_video: usize,
    pub max_danmaku_per_video: usize,
    pub max_fail_count: u32,
    pub danmaku_429_cooldown_secs: u64,
    pub max_consecutive_danmaku_429: u32,
}

impl Config {
    /// Reads `config.toml` from the current working directory,
    /// parses it, validates fields, and returns the populated `Config`.
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

    /// Validates field constraints.
    fn validate(&self) -> Result<()> {
        if self.crawler.trigger_hour >= 24 {
            return Err(CrawlerError::Config(format!(
                "trigger_hour must be < 24, got {}",
                self.crawler.trigger_hour
            )));
        }
        if self.crawler.increment_stop_hour >= 24 {
            return Err(CrawlerError::Config(format!(
                "increment_stop_hour must be < 24, got {}",
                self.crawler.increment_stop_hour
            )));
        }
        if self.partitions.rids.is_empty() {
            return Err(CrawlerError::Config(
                "partitions.rids must not be empty".to_string(),
            ));
        }
        if self.crawler.concurrency == 0 {
            return Err(CrawlerError::Config(
                "concurrency must be > 0".to_string(),
            ));
        }
        Ok(())
    }

    /// Expands `~` in `output_dir` to the user's home directory.
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

    #[test]
    fn test_validate_trigger_hour_too_high() {
        let config = Config {
            crawler: CrawlerConfig {
                trigger_hour: 24,
                full_crawl_timeout_secs: 7200,
                increment_interval_secs: 600,
                increment_stop_hour: 21,
                concurrency: 5,
                max_retries: 3,
                request_delay_ms: 2000,
                output_dir: "~/output".to_string(),
                db_path: "crawler_state.db".to_string(),
            },
            api: ApiConfig {
                base_url: "https://api.bilibili.com".to_string(),
                user_agent: "test".to_string(),
                cookie_env_name: "BILI_COOKIE".to_string(),
                wbi_cache_ttl_secs: 14400,
            },
            partitions: PartitionConfig {
                rids: vec![1001],
            },
            limits: LimitConfig {
                max_comments_per_video: 200,
                max_danmaku_per_video: 3000,
                max_fail_count: 3,
                danmaku_429_cooldown_secs: 60,
                max_consecutive_danmaku_429: 3,
            },
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_empty_rids() {
        let config = Config {
            crawler: CrawlerConfig {
                trigger_hour: 0,
                full_crawl_timeout_secs: 7200,
                increment_interval_secs: 600,
                increment_stop_hour: 21,
                concurrency: 5,
                max_retries: 3,
                request_delay_ms: 2000,
                output_dir: "~/output".to_string(),
                db_path: "crawler_state.db".to_string(),
            },
            api: ApiConfig {
                base_url: "https://api.bilibili.com".to_string(),
                user_agent: "test".to_string(),
                cookie_env_name: "BILI_COOKIE".to_string(),
                wbi_cache_ttl_secs: 14400,
            },
            partitions: PartitionConfig { rids: vec![] },
            limits: LimitConfig {
                max_comments_per_video: 200,
                max_danmaku_per_video: 3000,
                max_fail_count: 3,
                danmaku_429_cooldown_secs: 60,
                max_consecutive_danmaku_429: 3,
            },
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_concurrency_zero() {
        let config = Config {
            crawler: CrawlerConfig {
                trigger_hour: 0,
                full_crawl_timeout_secs: 7200,
                increment_interval_secs: 600,
                increment_stop_hour: 21,
                concurrency: 0,
                max_retries: 3,
                request_delay_ms: 2000,
                output_dir: "~/output".to_string(),
                db_path: "crawler_state.db".to_string(),
            },
            api: ApiConfig {
                base_url: "https://api.bilibili.com".to_string(),
                user_agent: "test".to_string(),
                cookie_env_name: "BILI_COOKIE".to_string(),
                wbi_cache_ttl_secs: 14400,
            },
            partitions: PartitionConfig {
                rids: vec![1001],
            },
            limits: LimitConfig {
                max_comments_per_video: 200,
                max_danmaku_per_video: 3000,
                max_fail_count: 3,
                danmaku_429_cooldown_secs: 60,
                max_consecutive_danmaku_429: 3,
            },
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_expand_tilde() {
        let mut config = Config {
            crawler: CrawlerConfig {
                trigger_hour: 0,
                full_crawl_timeout_secs: 7200,
                increment_interval_secs: 600,
                increment_stop_hour: 21,
                concurrency: 5,
                max_retries: 3,
                request_delay_ms: 2000,
                output_dir: "~/output".to_string(),
                db_path: "crawler_state.db".to_string(),
            },
            api: ApiConfig {
                base_url: "https://api.bilibili.com".to_string(),
                user_agent: "test".to_string(),
                cookie_env_name: "BILI_COOKIE".to_string(),
                wbi_cache_ttl_secs: 14400,
            },
            partitions: PartitionConfig {
                rids: vec![1001],
            },
            limits: LimitConfig {
                max_comments_per_video: 200,
                max_danmaku_per_video: 3000,
                max_fail_count: 3,
                danmaku_429_cooldown_secs: 60,
                max_consecutive_danmaku_429: 3,
            },
        };
        config.expand_tilde().unwrap();
        // HOME is set in CI/typical envs; at minimum the tilde should be removed
        assert!(!config.crawler.output_dir.contains('~'));
    }

    #[test]
    fn test_toml_round_trip() {
        let toml_str = r#"
[crawler]
trigger_hour = 0
full_crawl_timeout_secs = 7200
increment_interval_secs = 600
increment_stop_hour = 21
concurrency = 5
max_retries = 3
request_delay_ms = 2000
output_dir = "~/output"
db_path = "crawler_state.db"

[api]
base_url = "https://api.bilibili.com"
user_agent = "test-agent"
cookie_env_name = "BILI_COOKIE"
wbi_cache_ttl_secs = 14400

[partitions]
rids = [1001, 1002]

[limits]
max_comments_per_video = 200
max_danmaku_per_video = 3000
max_fail_count = 3
danmaku_429_cooldown_secs = 60
max_consecutive_danmaku_429 = 3
"#;
        let config: Config = toml::from_str(toml_str).expect("valid TOML");
        assert_eq!(config.crawler.trigger_hour, 0);
        assert_eq!(config.crawler.concurrency, 5);
        assert_eq!(config.api.base_url, "https://api.bilibili.com");
        assert_eq!(config.partitions.rids, vec![1001u32, 1002]);
        assert_eq!(config.limits.max_comments_per_video, 200);
    }
}
