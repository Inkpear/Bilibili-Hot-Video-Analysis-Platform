use crate::config::Config;
use crate::api::hot;
use anyhow::Result;
use tracing::info;
use std::collections::HashSet;
use std::path::Path;

pub async fn run_incremental_crawl(config: &Config) -> Result<()> {
    info!("Starting incremental crawl...");
    
    let client = reqwest::Client::new();
    
    // 1. 获取当前热门视频列表
    let current_bvids = hot::fetch_hot_videos(&client, &config.api.base_url).await?;
    
    // 2. 找到最新的快照目录
    let output_dir = Path::new(&config.crawler.output_dir);
    let latest_snapshot = find_latest_snapshot(output_dir)?;
    
    // 3. 读取已采集的视频列表
    let existing_bvids = read_existing_bvids(&latest_snapshot)?;
    
    // 4. 找出新增的视频
    let new_bvids: Vec<String> = current_bvids
        .into_iter()
        .filter(|bvid| !existing_bvids.contains(bvid))
        .collect();
    
    if new_bvids.is_empty() {
        info!("No new videos found");
        return Ok(());
    }
    
    info!("Found {} new videos", new_bvids.len());
    
    // 5. 采集新增视频（复用full crawl的逻辑）
    // TODO: 实现增量采集逻辑
    
    Ok(())
}

fn find_latest_snapshot(output_dir: &Path) -> Result<std::path::PathBuf> {
    let mut entries: Vec<_> = std::fs::read_dir(output_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();
    
    entries.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
    
    entries.first()
        .map(|e| e.path())
        .ok_or_else(|| anyhow::anyhow!("No snapshot directory found"))
}

fn read_existing_bvids(snapshot_dir: &Path) -> Result<HashSet<String>> {
    let video_path = snapshot_dir.join("video_detail.ndjson");
    if !video_path.exists() {
        return Ok(HashSet::new());
    }
    
    let content = std::fs::read_to_string(video_path)?;
    let bvids: HashSet<String> = content
        .lines()
        .filter_map(|line| {
            serde_json::from_str::<serde_json::Value>(line).ok()
                .and_then(|v| v.get("bvid").and_then(|b| b.as_str()).map(String::from))
        })
        .collect();
    
    Ok(bvids)
}
