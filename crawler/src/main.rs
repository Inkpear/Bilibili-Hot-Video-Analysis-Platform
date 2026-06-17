use std::sync::Arc;
use std::time::Duration;

use chrono::Datelike;
use indicatif::ProgressStyle;
use tracing::{error, info};
use tracing_indicatif::IndicatifLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use bilibili_crawler::config::Config;
use bilibili_crawler::error::Result;
use bilibili_crawler::scheduler::run_crawl;
use bilibili_crawler::state::StateManager;

#[tokio::main]
async fn main() -> Result<()> {
    let indicatif_layer = IndicatifLayer::new().with_max_progress_bars(
        12,
        Some(
            ProgressStyle::with_template("  ...and {pending_progress_bars} more not shown")
                .unwrap(),
        ),
    );

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("bilibili_crawler=info,hyper_util=warn,reqwest=warn")
    });

    tracing_subscriber::registry()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(indicatif_layer.get_stderr_writer()),
        )
        .with(indicatif_layer)
        .init();

    let run_now = std::env::args().any(|a| a == "--now");
    let etl_test_date = std::env::args()
        .position(|a| a == "--etl-test")
        .and_then(|i| std::env::args().nth(i + 1));

    let config = Config::load()?;
    info!(
        trigger_hour = config.crawler.trigger_hour,
        accounts = config.accounts.len(),
        concurrency = config.crawler.concurrency,
        "Starting Bilibili Crawler v3"
    );

    let state = Arc::new(StateManager::new(&config.crawler.db_path)?);

    if let Some(date) = etl_test_date {
        let etl_config = config.etl.as_ref().expect("ETL config required for --etl-test");
        let snapshot_dir = std::path::PathBuf::from(&config.crawler.output_dir).join(&date);
        info!(date = %date, dir = %snapshot_dir.display(), "running ETL test");
        match bilibili_crawler::etl::run_etl(etl_config, &snapshot_dir, &date).await {
            Ok(()) => info!("ETL completed"),
            Err(e) => error!(error = %e, "ETL failed"),
        }
        return Ok(());
    }

    if run_now {
        // Run once and exit
        info!("--now flag: running crawl immediately");
        match run_crawl(&config, &state).await {
            Ok(()) => info!("Crawl completed"),
            Err(e) => error!(error = %e, "Crawl failed"),
        }
        return Ok(());
    }

    // Daemon mode: crawl immediately on startup, then sleep until next trigger
    loop {
        let now = chrono::Local::now();
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

        // Sleep until next trigger_hour
        let now = chrono::Local::now();
        let tomorrow = now.date_naive() + chrono::Duration::days(1);
        let next_time = tomorrow
            .and_hms_opt(config.crawler.trigger_hour, 0, 0)
            .unwrap_or_default();
        let duration = (next_time - now.naive_local()).num_seconds().max(0) as u64;
        info!(seconds = duration, "sleeping until next trigger");
        tokio::time::sleep(Duration::from_secs(duration)).await;
    }
}
