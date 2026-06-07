use crate::config::Config;
use crate::api::{hot, video, comment, danmaku};
use crate::storage::ndjson::NdjsonWriter;
use anyhow::Result;
use tracing::{info, error};
use std::sync::Arc;
use tokio::sync::Semaphore;

pub async fn run_full_crawl(config: &Config) -> Result<()> {
    info!("Starting full crawl...");
    
    let client = reqwest::Client::new();
    let writer = NdjsonWriter::new(&config.crawler.output_dir);
    let snapshot_dir = writer.create_snapshot_dir()?;
    let semaphore = Arc::new(Semaphore::new(config.crawler.concurrency));
    
    // 1. 获取热门视频列表
    let bvids = hot::fetch_hot_videos(&client, &config.api.base_url).await?;
    info!("Found {} hot videos", bvids.len());
    
    // 2. 并发采集每个视频
    let mut handles = vec![];
    for bvid in bvids {
        let client = client.clone();
        let base_url = config.api.base_url.clone();
        let snapshot_dir = snapshot_dir.clone();
        let sem = semaphore.clone();
        
        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            process_video(&client, &base_url, &snapshot_dir, &bvid).await
        });
        handles.push(handle);
    }
    
    // 等待所有任务完成
    for handle in handles {
        if let Err(e) = handle.await? {
            error!("Task failed: {}", e);
        }
    }
    
    info!("Full crawl completed!");
    Ok(())
}

async fn process_video(
    client: &reqwest::Client,
    base_url: &str,
    snapshot_dir: &str,
    bvid: &str,
) -> Result<()> {
    // 采集视频详情
    let video_detail = video::fetch_video_detail(client, base_url, bvid).await?;
    
    // 采集UP主信息
    let up_info = video::fetch_up_info(client, base_url, &video_detail.up_id).await?;
    
    // 采集评论
    let comments = comment::fetch_comments(client, base_url, bvid, video_detail.aid, 200).await?;
    
    // 采集弹幕
    let danmaku_list = danmaku::fetch_danmaku(client, base_url, video_detail.cid, 3000).await?;
    
    // 写入文件
    let writer = NdjsonWriter::new(snapshot_dir);
    
    // 写入视频详情（追加）
    let video_path = format!("{}/video_detail.ndjson", snapshot_dir);
    writer.append_ndjson(&video_path, &[video_detail])?;
    
    // 写入UP主信息（追加）
    let up_path = format!("{}/up_info.ndjson", snapshot_dir);
    writer.append_ndjson(&up_path, &[up_info])?;
    
    // 写入评论
    if !comments.is_empty() {
        let comment_path = format!("{}/comments/{}.ndjson", snapshot_dir, bvid);
        writer.write_ndjson(&comment_path, &comments)?;
    }
    
    // 写入弹幕
    if !danmaku_list.is_empty() {
        let danmaku_path = format!("{}/danmaku/{}.ndjson", snapshot_dir, bvid);
        writer.write_ndjson(&danmaku_path, &danmaku_list)?;
    }
    
    info!("Processed video: {}", bvid);
    Ok(())
}
