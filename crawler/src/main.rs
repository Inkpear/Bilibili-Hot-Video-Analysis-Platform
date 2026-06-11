use std::sync::Arc;
use std::time::Duration;

use chrono::{Datelike, Timelike};
use tracing::{error, info};

use bilibili_crawler::config::Config;
use bilibili_crawler::error::Result;
use bilibili_crawler::scheduler::run_crawl;
use bilibili_crawler::state::StateManager;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let run_now = std::env::args().any(|a| a == "--now");

    let config = Config::load()?;
    info!(
        trigger_hour = config.crawler.trigger_hour,
        accounts = config.accounts.len(),
        concurrency = config.crawler.concurrency,
        "Starting Bilibili Crawler v3"
    );

    let state = Arc::new(StateManager::new(&config.crawler.db_path)?);

    if run_now {
        // Run once and exit
        info!("--now flag: running crawl immediately");
        match run_crawl(&config, &state).await {
            Ok(()) => info!("Crawl completed"),
            Err(e) => error!(error = %e, "Crawl failed"),
        }
        return Ok(());
    }

    // Daemon mode: loop daily at trigger_hour
    loop {
        let now = chrono::Local::now();
        let current_hour = now.hour();

        if current_hour == config.crawler.trigger_hour {
            let today = format!("{:04}{:02}{:02}", now.year(), now.month(), now.day());
            if state.has_full_crawl_today(&today).unwrap_or(false) {
                info!(date = %today, "crawl already completed today, skipping");
            } else {
                info!("=== Starting crawl ===");
                match run_crawl(&config, &state).await {
                    Ok(()) => info!("=== Crawl completed ==="),
                    Err(e) => error!(error = %e, "Crawl failed"),
                }
            }

            let tomorrow = now.date_naive() + chrono::Duration::days(1);
            let next_time = tomorrow
                .and_hms_opt(config.crawler.trigger_hour, 0, 0)
                .unwrap_or_default();
            let now_naive = now.naive_local();
            let duration = (next_time - now_naive).num_seconds().max(0) as u64;
            info!(seconds = duration, "sleeping until next crawl trigger");
            tokio::time::sleep(Duration::from_secs(duration)).await;
        } else {
            let next_time = now
                .date_naive()
                .and_hms_opt(config.crawler.trigger_hour, 0, 0)
                .unwrap_or_default();
            let now_naive = now.naive_local();
            let duration = if next_time > now_naive {
                (next_time - now_naive).num_seconds().max(0) as u64
            } else {
                let tomorrow = now.date_naive() + chrono::Duration::days(1);
                let next_time = tomorrow
                    .and_hms_opt(config.crawler.trigger_hour, 0, 0)
                    .unwrap_or_default();
                (next_time - now_naive).num_seconds().max(0) as u64
            };
            info!(seconds = duration, "sleeping until trigger hour");
            tokio::time::sleep(Duration::from_secs(duration)).await;
        }
    }
}
