pub mod db;
pub mod loader;
pub mod word_segment;

use std::path::Path;

use tracing::info;

use crate::config::EtlConfig;
use crate::error::Result;

pub async fn run_etl(config: &EtlConfig, snapshot_dir: &Path, snapshot_date: &str) -> Result<()> {
    let snapshot_date = config
        .snapshot_date
        .as_deref()
        .unwrap_or(snapshot_date);

    info!(date = %snapshot_date, dir = %snapshot_dir.display(), "ETL starting");

    let db = db::MysqlDb::connect(&config.mysql_url).await?;
    let segmenter = word_segment::WordSegmenter::new(Path::new(&config.stopwords_path))?;

    // 1. Load NDJSON data
    let videos = loader::load_videos(&snapshot_dir.join("video_detail"))?;
    let ups = loader::load_ups(&snapshot_dir.join("up_info"))?;
    let comments = loader::load_comments(&snapshot_dir.join("comments"))?;
    let danmaku_list = loader::load_danmaku(&snapshot_dir.join("danmaku"))?;

    // 2. 开启事务，减少 fsync 开销
    let tx = db.begin_etl_tx().await?;

    db.insert_videos_batch(&videos, snapshot_date).await?;
    db.insert_interactions_batch(&videos, snapshot_date).await?;
    db.insert_up_info_batch(&ups, snapshot_date).await?;
    db.insert_comments_batch(&comments, snapshot_date).await?;
    db.insert_danmaku_batch(&danmaku_list, snapshot_date).await?;

    // 3. Word cloud + trends
    let title_words = segmenter.segment_by_bvid_titles(&videos);
    let comment_words = segmenter.segment_by_bvid_comments(&comments, &videos);
    let danmaku_words = segmenter.segment_by_bvid_danmaku(&danmaku_list, &videos);

    db.insert_word_cloud("word_cloud_title", &title_words, snapshot_date)
        .await?;
    db.insert_word_cloud("word_cloud_comment", &comment_words, snapshot_date)
        .await?;
    db.insert_word_cloud("word_cloud_danmaku", &danmaku_words, snapshot_date)
        .await?;

    let trends = segmenter.compute_trends(&title_words, &comment_words, &danmaku_words);
    db.insert_word_trend(&trends, snapshot_date).await?;

    // 4. Snapshot record
    db.insert_snapshot_date(
        snapshot_date,
        videos.len(),
        comments.len(),
        danmaku_list.len(),
    )
    .await?;

    // 5. 提交事务
    tx.commit().await?;

    info!("ETL completed");
    Ok(())
}
