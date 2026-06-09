use std::sync::Arc;
use std::time::Duration;
use chrono::Timelike;
use tracing::{error, info};

use bilibili_crawler::config::Config;
use bilibili_crawler::error::Result;
use bilibili_crawler::scheduler::{run_full_crawl, run_incremental_crawl};
use bilibili_crawler::state::StateManager;
use bilibili_crawler::wbi::WbiSigner;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config = Config::load()?;
    info!(
        trigger_hour = config.crawler.trigger_hour,
        stop_hour = config.crawler.increment_stop_hour,
        concurrency = config.crawler.concurrency,
        "Starting Bilibili Crawler v2"
    );

    let state = Arc::new(StateManager::new(&config.crawler.db_path)?);
    let client = reqwest::Client::builder()
        .user_agent(&config.api.user_agent)
        .timeout(Duration::from_secs(30))
        .build()?;

    let cookie = std::env::var(&config.api.cookie_env_name).map_err(|_| {
        bilibili_crawler::error::CrawlerError::Config(format!(
            "env var '{}' not set",
            config.api.cookie_env_name
        ))
    })?;

    let mut wbi_signer = WbiSigner::new(cookie, config.api.base_url.clone(), client.clone());

    loop {
        let now = chrono::Local::now();
        let today = now.format("%Y%m%d").to_string();
        let current_hour = now.hour();

        let needs_full = !state.has_full_crawl_today(&today)?;

        if needs_full {
            info!("=== Starting FULL crawl ===");
            match run_full_crawl(&client, &config, &state, &mut wbi_signer).await {
                Ok(()) => info!("=== FULL crawl completed ==="),
                Err(e) => error!(error = %e, "FULL crawl failed"),
            }
        } else if current_hour < config.crawler.increment_stop_hour {
            info!("=== Starting INCREMENTAL crawl ===");
            match run_incremental_crawl(&client, &config, &state, &mut wbi_signer).await {
                Ok(()) => info!("=== INCREMENTAL crawl completed ==="),
                Err(e) => error!(error = %e, "INCREMENTAL crawl failed"),
            }
        } else {
            info!(hour = current_hour, "Past stop hour, sleeping");
        }

        if needs_full {
            let tomorrow = now.date_naive() + chrono::Duration::days(1);
            let next_time = tomorrow
                .and_hms_opt(config.crawler.trigger_hour, 0, 0)
                .unwrap_or_default();
            let now_naive = now.naive_local();
            let duration = (next_time - now_naive).num_seconds().max(0) as u64;
            info!(seconds = duration, "sleeping until next full crawl trigger");
            tokio::time::sleep(Duration::from_secs(duration)).await;
        } else {
            let sleep_secs = config.crawler.increment_interval_secs;
            info!(seconds = sleep_secs, "sleeping until next incremental");
            tokio::time::sleep(Duration::from_secs(sleep_secs)).await;
        }
    }
}
