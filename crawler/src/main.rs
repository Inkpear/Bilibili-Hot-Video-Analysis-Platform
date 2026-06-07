use anyhow::Result;
use tracing::info;

mod config;
mod api;
mod models;
mod storage;
mod scheduler;
mod error;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    info!("Bilibili Crawler starting...");
    
    let config = config::Config::load()?;
    
    // TODO: 根据参数决定全量/增量模式
    scheduler::run_full_crawl(&config).await?;
    
    Ok(())
}
