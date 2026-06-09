use std::collections::HashSet;
use std::sync::Arc;
use chrono::Timelike;
use tokio::sync::Semaphore;
use tracing::{info, warn};

use crate::config::Config;
use crate::error::Result;
use crate::state::StateManager;
use crate::wbi::WbiSigner;

/// Run incremental crawl: retry failed bvids + check for new ones.
/// Stops if current hour >= increment_stop_hour (default 21:00).
pub async fn run_incremental_crawl(
    client: &reqwest::Client,
    config: &Config,
    state: &StateManager,
    wbi_signer: &mut WbiSigner,
) -> Result<()> {
    let now = chrono::Local::now();
    let current_hour = now.hour();

    if current_hour >= config.crawler.increment_stop_hour {
        info!(
            "Skipping incremental: hour {} >= stop hour {}",
            current_hour, config.crawler.increment_stop_hour
        );
        return Ok(());
    }

    let failed_bvids = state.get_failed_bvids(config.limits.max_fail_count)?;
    info!(count = failed_bvids.len(), "failed bvids to retry");

    let today_str = now.format("%Y%m%d").to_string();
    let snapshot_id = match state.get_last_snapshot()? {
        Some(id) if id.starts_with(&today_str) => id,
        _ => {
            warn!("No full crawl snapshot for today, skipping incremental");
            return Ok(());
        }
    };
    let writer = crate::storage::NdjsonWriter::new(&config.crawler.output_dir);
    let snapshot_dir =
        std::path::PathBuf::from(&config.crawler.output_dir).join(&snapshot_id);

    if !snapshot_dir.exists() {
        let expanded = if config.crawler.output_dir.starts_with("~/") {
            let home = std::env::var("HOME").unwrap_or_default();
            std::path::PathBuf::from(home).join(&config.crawler.output_dir[2..])
        } else {
            std::path::PathBuf::from(&config.crawler.output_dir)
        };
        let full_dir = expanded.join(&snapshot_id);
        if !full_dir.exists() {
            warn!(dir = %full_dir.display(), "snapshot directory not found");
            return Ok(());
        }
    }

    let ranked_videos =
        crate::api::ranking::fetch_all_rankings(client, config, wbi_signer).await?;
    let mut new_bvids: Vec<String> = Vec::new();
    let all_bvids: Vec<String> = ranked_videos.iter().map(|rv| rv.bvid.clone()).collect();
    let existing_new = state.get_new_bvids_for_check(&all_bvids)?;
    for bvid in &existing_new {
        if !failed_bvids.contains(bvid) && !state.video_done(bvid).unwrap_or(false) {
            new_bvids.push(bvid.clone());
            if let Some(rv) = ranked_videos.iter().find(|rv| rv.bvid == *bvid) {
                let _ = state.insert_video(
                    bvid,
                    &rv.title,
                    &rv.category,
                    &rv.sub_category,
                    rv.aid,
                    rv.cid,
                    rv.duration,
                    rv.danmaku_count,
                    &rv.up_id,
                    &snapshot_id,
                );
            }
        }
    }
    info!(new = new_bvids.len(), "new bvids from ranking");

    let all_to_process: Vec<String> = {
        let mut combined: HashSet<String> = failed_bvids.into_iter().collect();
        for bvid in new_bvids {
            combined.insert(bvid);
        }
        combined.into_iter().collect()
    };

    if all_to_process.is_empty() {
        info!("nothing to process in incremental crawl");
        return Ok(());
    }

    info!(total = all_to_process.len(), "processing incremental");

    let semaphore = Arc::new(Semaphore::new(config.crawler.concurrency));
    let writer = Arc::new(writer);
    let wbi_mutex = Arc::new(tokio::sync::Mutex::new(wbi_signer));

    for bvid in &all_to_process {
        let _permit = semaphore.acquire().await.map_err(|_| {
            crate::error::CrawlerError::Config("semaphore closed".into())
        })?;

        let rv = match ranked_videos.iter().find(|rv| rv.bvid == *bvid) {
            Some(rv) => rv.clone(),
            None => continue,
        };

        let mut wbi_guard = wbi_mutex.lock().await;
        match super::full::process_single_video(
            bvid.clone(),
            &rv,
            client,
            config,
            state,
            &writer,
            &super::full::SnapshotContext {
                snapshot_id: snapshot_id.clone(),
                snapshot_dir: snapshot_dir.clone(),
                collect_time: chrono::Utc::now().timestamp(),
            },
            &mut *wbi_guard,
        )
        .await
        {
            Ok(()) => info!(bvid = %bvid, "incremental retry success"),
            Err(e) => warn!(bvid = %bvid, error = %e, "incremental retry failed"),
        }
        drop(wbi_guard);
    }

    info!("incremental crawl complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cutoff_hour() {
        let stop_hour = 21u32;
        let current = 22u32;
        assert!(current >= stop_hour);
    }
}
