use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use indicatif::{ProgressBar, ProgressStyle};
use tokio::sync::Semaphore;
use tracing::{error, info, warn};

use crate::api::ranking::RankedVideo;
use crate::api::{comment, danmaku, ranking, up, video};
use crate::config::Config;
use crate::error::{CrawlerError, Result};
use crate::models::VideoDetail;
use crate::state::StateManager;
use crate::storage::NdjsonWriter;
use crate::wbi::WbiSigner;

// ---------------------------------------------------------------------------
// Snapshot context
// ---------------------------------------------------------------------------

pub(crate) struct SnapshotContext {
    pub snapshot_id: String,
    pub snapshot_dir: std::path::PathBuf,
    pub collect_time: i64,
}

// ---------------------------------------------------------------------------
// Merge helper (public for tests)
// ---------------------------------------------------------------------------

/// Merge ranked video metadata with the `/view` API detail into a complete
/// [`VideoDetail`].  Ranking fields (category, rank_*, url, snapshot_id,
/// collect_time) come from `ranked`; description, tags, and accurate aid/cid
/// come from `detail`.
fn build_video_detail(
    ranked: &RankedVideo,
    detail: &VideoDetail,
    ctx: &SnapshotContext,
) -> VideoDetail {
    VideoDetail {
        bvid: detail.bvid.clone(),
        aid: if detail.aid != 0 { detail.aid } else { ranked.aid },
        cid: if detail.cid != 0 { detail.cid } else { ranked.cid },
        title: ranked.title.clone(),
        description: detail.description.clone(),
        tags: detail.tags.clone(),
        category: ranked.category.clone(),
        sub_category: ranked.sub_category.clone(),
        duration: ranked.duration,
        publish_time: ranked.publish_time,
        up_id: ranked.up_id.clone(),
        up_name: ranked.up_name.clone(),
        up_avatar: ranked.up_avatar.clone(),
        url: ranked.url.clone(),
        view_count: ranked.view_count,
        like_count: ranked.like_count,
        coin_count: ranked.coin_count,
        favorite_count: ranked.favorite_count,
        share_count: ranked.share_count,
        reply_count: ranked.reply_count,
        danmaku_count: ranked.danmaku_count,
        rank_source: ranked.rank_source.clone(),
        rank_name: ranked.rank_name.clone(),
        rank_position: ranked.rank_position,
        snapshot_id: ctx.snapshot_id.clone(),
        collect_time: ctx.collect_time,
    }
}

// ---------------------------------------------------------------------------
// Per-video transaction
// ---------------------------------------------------------------------------

/// Process a single video end-to-end: detail → UP → comments → danmaku → write.
///
/// On success the video's failure counter is reset; on failure it is incremented.
pub(crate) async fn process_single_video(
    bvid: String,
    ranked: &RankedVideo,
    client: &reqwest::Client,
    config: &Config,
    state: &StateManager,
    writer: &NdjsonWriter,
    ctx: &SnapshotContext,
    wbi_signer: &mut WbiSigner,
) -> Result<()> {
    // a. Already fully done → skip
    if state.video_done(&bvid)? {
        info!(bvid = %bvid, "video already fully done, skipping");
        return Ok(());
    }

    let result =
        process_video_inner(&bvid, ranked, client, config, state, writer, ctx, wbi_signer).await;

    match &result {
        Ok(()) => {
            state.reset_fail(&bvid)?;
            info!(bvid = %bvid, "video processing completed");
        }
        Err(e) => {
            error!(bvid = %bvid, error = %e, "video processing failed");
            state.increment_fail(&bvid, &e.to_string())?;
        }
    }

    result
}

/// Inner processing logic — separated so the outer function can handle
/// success/fail bookkeeping uniformly.
async fn process_video_inner(
    bvid: &str,
    ranked: &RankedVideo,
    client: &reqwest::Client,
    config: &Config,
    state: &StateManager,
    writer: &NdjsonWriter,
    ctx: &SnapshotContext,
    wbi_signer: &mut WbiSigner,
) -> Result<()> {
    // b. Fetch /view detail
    let detail = video::fetch_video_detail(client, config, bvid).await?;
    state.mark_detail_done(bvid)?;

    // c. Fetch UP info (best-effort — failure does NOT abort the video)
    if !state.is_up_collected(&ranked.up_id)? && !state.should_skip_up(&ranked.up_id)? {
        match up::fetch_up_info(client, config, wbi_signer, &ranked.up_id).await {
            Ok(up_info) => {
                writer.append_ndjson(&ctx.snapshot_dir.join("up_info.ndjson"), &[up_info])?;
                state.mark_up_collected(&ranked.up_id)?;
            }
            Err(e) => {
                warn!(up_id = %ranked.up_id, error = %e, "UP info fetch failed, recording");
                state.record_up_failure(&ranked.up_id, &e.to_string())?;
            }
        }
    }

    // d. Fetch comments
    let existing_comment_ids: HashSet<i64> =
        state.get_collected_comment_ids(bvid)?.into_iter().collect();
    let aid = if detail.aid != 0 { detail.aid } else { ranked.aid };
    let (new_comments, _ok) =
        comment::collect_comments(client, config, bvid, aid, &existing_comment_ids).await?;
    if !new_comments.is_empty() {
        let ids: Vec<(i64, &str)> = new_comments.iter().map(|c| (c.comment_id, bvid)).collect();
        state.insert_comments_batch(&ids)?;
        writer.write_ndjson(
            &ctx.snapshot_dir.join("comments").join(format!("{bvid}.ndjson")),
            &new_comments,
        )?;
    }
    state.mark_comments_done(bvid)?;

    // e. Fetch danmaku
    let cid = if detail.cid != 0 { detail.cid } else { ranked.cid };
    let existing_danmaku_ids: HashSet<i64> =
        state.get_collected_danmaku_ids(bvid)?.into_iter().collect();
    let (new_danmaku, _ok) =
        danmaku::collect_danmaku(client, config, bvid, cid, &existing_danmaku_ids).await?;
    if !new_danmaku.is_empty() {
        let ids: Vec<(i64, &str)> = new_danmaku.iter().map(|d| (d.danmaku_id, bvid)).collect();
        state.insert_danmaku_batch(&ids)?;
        writer.write_ndjson(
            &ctx.snapshot_dir.join("danmaku").join(format!("{bvid}.ndjson")),
            &new_danmaku,
        )?;
    }
    state.mark_danmaku_done(bvid)?;

    // f + g. Build merged VideoDetail and append to video_detail.ndjson
    let video = build_video_detail(ranked, &detail, ctx);
    writer.append_ndjson(&ctx.snapshot_dir.join("video_detail.ndjson"), &[video])?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Concurrent executor (no 'static requirement)
// ---------------------------------------------------------------------------

/// Await all futures concurrently, returning results in order.
///
/// Unlike `tokio::spawn`, this does **not** require `'static` — the futures
/// may borrow data from the calling scope.  Polling is O(n) per wake-up,
/// which is fine for the small batches typical of a crawl.
async fn join_all_concurrent<F, T>(futures: Vec<F>) -> Vec<T>
where
    F: Future<Output = T>,
    T: Unpin,
{
    struct JoinAll<F: Future> {
        futures: Vec<Pin<Box<F>>>,
        results: Vec<Option<F::Output>>,
    }

    impl<F: Future> Future for JoinAll<F>
    where
        F::Output: Unpin,
    {
        type Output = Vec<F::Output>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let this = &mut *self;
            let mut all_done = true;
            for (i, future) in this.futures.iter_mut().enumerate() {
                if this.results[i].is_some() {
                    continue;
                }
                match future.as_mut().poll(cx) {
                    Poll::Ready(val) => this.results[i] = Some(val),
                    Poll::Pending => all_done = false,
                }
            }
            if all_done {
                Poll::Ready(this.results.drain(..).map(Option::unwrap).collect())
            } else {
                Poll::Pending
            }
        }
    }

    let n = futures.len();
    JoinAll {
        futures: futures.into_iter().map(Box::pin).collect(),
        results: (0..n).map(|_| None).collect(),
    }
    .await
}

// ---------------------------------------------------------------------------
// Main orchestrator
// ---------------------------------------------------------------------------

/// Execute a complete daily full crawl end-to-end.
///
/// 1. Fetch 14 partitions → deduplicate videos.
/// 2. For each new / incomplete video: detail → UP → comments → danmaku.
/// 3. Write NDJSON outputs and track state in SQLite.
pub async fn run_full_crawl(
    client: &reqwest::Client,
    config: &Config,
    state: &StateManager,
    wbi_signer: &mut WbiSigner,
) -> Result<()> {
    // a. Create snapshot context
    let writer = NdjsonWriter::new(&config.crawler.output_dir);
    let snapshot_dir = writer.create_snapshot_dir()?;
    let snapshot_id = snapshot_dir
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let collect_time = chrono::Utc::now().timestamp();
    let ctx = SnapshotContext {
        snapshot_id: snapshot_id.clone(),
        snapshot_dir,
        collect_time,
    };

    // b. Register snapshot in state
    state.create_snapshot(&snapshot_id)?;
state.reset_daily_flags()?;
    info!("daily flags reset for full crawl");
    info!(snapshot_id = %snapshot_id, "snapshot created");

    // c. Fetch all rankings (with progress indicator)
    let is_tty = atty::is(atty::Stream::Stderr);
    let rank_pb = if is_tty {
        let pb = ProgressBar::new_spinner();
        pb.set_style(ProgressStyle::with_template("{spinner} {msg}").unwrap());
        pb.set_message("Fetching 14 partitions...");
        Some(pb)
    } else {
        None
    };

    let ranked_videos = ranking::fetch_all_rankings(client, config, wbi_signer).await?;
    if let Some(pb) = &rank_pb {
        pb.finish_with_message(format!("✓ Fetched {} videos from 14 partitions", ranked_videos.len()));
    } else {
        info!(count = ranked_videos.len(), "all rankings fetched");
    }

    // Write initial video_detail.ndjson from ranking data
    // (description/tags will be enriched by process_single_video later)
    let initial_videos: Vec<VideoDetail> = ranked_videos
        .iter()
        .map(|rv| VideoDetail {
            bvid: rv.bvid.clone(),
            aid: rv.aid,
            cid: rv.cid,
            title: rv.title.clone(),
            description: String::new(),
            tags: String::new(),
            category: rv.category.clone(),
            sub_category: rv.sub_category.clone(),
            duration: rv.duration,
            publish_time: rv.publish_time,
            up_id: rv.up_id.clone(),
            up_name: rv.up_name.clone(),
            up_avatar: rv.up_avatar.clone(),
            url: rv.url.clone(),
            view_count: rv.view_count,
            like_count: rv.like_count,
            coin_count: rv.coin_count,
            favorite_count: rv.favorite_count,
            share_count: rv.share_count,
            reply_count: rv.reply_count,
            danmaku_count: rv.danmaku_count,
            rank_source: rv.rank_source.clone(),
            rank_name: rv.rank_name.clone(),
            rank_position: rv.rank_position,
            snapshot_id: snapshot_id.clone(),
            collect_time,
        })
        .collect();
    writer.write_ndjson(&ctx.snapshot_dir.join("video_detail.ndjson"), &initial_videos)?;

    // d. Register new bvids in state
    let mut new_count = 0usize;
    for rv in &ranked_videos {
        let is_new = state.insert_video(
            &rv.bvid,
            &rv.title,
            &rv.category,
            &rv.sub_category,
            rv.aid,
            rv.cid,
            rv.duration,
            rv.danmaku_count,
            &rv.up_id,
            &snapshot_id,
        )?;
        if is_new {
            new_count += 1;
        }
    }
    info!(new = new_count, total = ranked_videos.len(), "videos registered");

    // e. Deduplicate bvids, filter out already-done videos
    let mut seen = HashSet::new();
    let mut to_process: Vec<RankedVideo> = Vec::new();
    for rv in ranked_videos {
        if seen.contains(&rv.bvid) {
            continue;
        }
        seen.insert(rv.bvid.clone());
        if state.video_done(&rv.bvid)? {
            continue;
        }
        to_process.push(rv);
    }
    let total = to_process.len();
    let video_pb = if is_tty {
        let pb = ProgressBar::new(total as u64);
        pb.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta}) {msg}"
            ).unwrap()
        );
        pb.set_message("processing videos");
        Some(Arc::new(pb))
    } else {
        info!(count = total, "videos to process after dedup + done-filter");
        None
    };

    if to_process.is_empty() {
        info!("no videos to process, completing snapshot immediately");
        state.complete_snapshot(&snapshot_id, 0, 0, 0, 0, "ok")?;
        return Ok(());
    }

    // f. Semaphore for concurrency control
    let semaphore = Arc::new(Semaphore::new(config.crawler.concurrency));

    // g. Build concurrent futures (one per video)
    let writer = Arc::new(writer);
    let ctx = Arc::new(ctx);
    let wbi_mutex = Arc::new(tokio::sync::Mutex::new(wbi_signer));

    let futures: Vec<_> = to_process
        .into_iter()
        .map(|rv| {
            let sem = semaphore.clone();
            let writer = writer.clone();
            let ctx = ctx.clone();
            let wbi_mutex = wbi_mutex.clone();
            let pb = video_pb.clone();
            async move {
                let _permit = sem
                    .acquire()
                    .await
                    .map_err(|_| CrawlerError::Config("semaphore closed".into()))?;
                let mut wbi_guard = wbi_mutex.lock().await;
                let result = process_single_video(
                    rv.bvid.clone(),
                    &rv,
                    client,
                    config,
                    state,
                    &writer,
                    &ctx,
                    &mut **wbi_guard,
                )
                .await;
                if let Some(ref pb) = pb {
                    pb.inc(1);
                }
                result
            }
        })
        .collect();

    // h. Wait for all tasks to complete
    let results = join_all_concurrent(futures).await;

    let mut ok_count: i64 = 0;
    let mut err_count: i64 = 0;
    for r in &results {
        match r {
            Ok(()) => ok_count += 1,
            Err(e) => {
                err_count += 1;
                warn!(error = %e, "video task returned error");
            }
        }
    }

    // i. Collect snapshot stats
    let comment_count = count_files_in_dir(&ctx.snapshot_dir.join("comments")) as i64;
    let danmaku_count = count_files_in_dir(&ctx.snapshot_dir.join("danmaku")) as i64;
    let up_count = count_lines_in_file(&ctx.snapshot_dir.join("up_info.ndjson")) as i64;

    state.complete_snapshot(
        &snapshot_id,
        ok_count,
        comment_count,
        danmaku_count,
        up_count,
        "ok",
    )?;

    // j. Final stats log
    info!(
        snapshot_id = %snapshot_id,
        videos_ok = ok_count,
        videos_err = err_count,
        comments = comment_count,
        danmaku = danmaku_count,
        ups = up_count,
        "full crawl completed"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Small counting helpers
// ---------------------------------------------------------------------------

fn count_files_in_dir(dir: &std::path::Path) -> usize {
    std::fs::read_dir(dir)
        .map(|rd| rd.filter_map(|e| e.ok()).count())
        .unwrap_or(0)
}

fn count_lines_in_file(path: &std::path::Path) -> usize {
    std::fs::read_to_string(path)
        .map(|s| s.lines().count())
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ranked() -> RankedVideo {
        RankedVideo {
            bvid: "BV1xx411c7mD".into(),
            aid: 171049,
            cid: 2811658,
            title: "Ranked Title".into(),
            category: "科技".into(),
            sub_category: "计算机".into(),
            duration: 300,
            publish_time: 1_700_000_000,
            up_id: "12345".into(),
            up_name: "TestUP".into(),
            up_avatar: "https://example.com/avatar.jpg".into(),
            url: "https://www.bilibili.com/video/BV1xx411c7mD".into(),
            view_count: 100_000,
            like_count: 5_000,
            coin_count: 2_000,
            favorite_count: 3_000,
            share_count: 500,
            reply_count: 100,
            danmaku_count: 800,
            rank_source: "ranking".into(),
            rank_name: "科技榜".into(),
            rank_position: 1,
        }
    }

    fn make_detail() -> VideoDetail {
        VideoDetail {
            bvid: "BV1xx411c7mD".into(),
            aid: 171049,
            cid: 2811658,
            title: "Detail Title".into(),
            description: "A description from /view".into(),
            tags: "rust,programming".into(),
            category: String::new(),
            sub_category: String::new(),
            duration: 300,
            publish_time: 1_700_000_000,
            up_id: "12345".into(),
            up_name: "TestUP".into(),
            up_avatar: "https://example.com/avatar.jpg".into(),
            url: String::new(),
            view_count: 100_000,
            like_count: 5_000,
            coin_count: 2_000,
            favorite_count: 3_000,
            share_count: 500,
            reply_count: 100,
            danmaku_count: 800,
            rank_source: String::new(),
            rank_name: String::new(),
            rank_position: 0,
            snapshot_id: String::new(),
            collect_time: 0,
        }
    }

    fn make_ctx() -> SnapshotContext {
        SnapshotContext {
            snapshot_id: "20250101_120000".into(),
            snapshot_dir: std::path::PathBuf::from("/tmp/test"),
            collect_time: 1_700_000_000,
        }
    }

    #[test]
    fn test_ranked_to_detail_merge() {
        let ranked = make_ranked();
        let detail = make_detail();
        let ctx = make_ctx();

        let merged = build_video_detail(&ranked, &detail, &ctx);

        // Ranking fields come from `ranked`
        assert_eq!(merged.title, "Ranked Title");
        assert_eq!(merged.category, "科技");
        assert_eq!(merged.sub_category, "计算机");
        assert_eq!(merged.url, "https://www.bilibili.com/video/BV1xx411c7mD");
        assert_eq!(merged.rank_source, "ranking");
        assert_eq!(merged.rank_name, "科技榜");
        assert_eq!(merged.rank_position, 1);

        // Detail fields come from `detail`
        assert_eq!(merged.description, "A description from /view");
        assert_eq!(merged.tags, "rust,programming");

        // Context fields
        assert_eq!(merged.snapshot_id, "20250101_120000");
        assert_eq!(merged.collect_time, 1_700_000_000);

        // aid/cid use detail when non-zero
        assert_eq!(merged.aid, 171049);
        assert_eq!(merged.cid, 2811658);
    }

    #[test]
    fn test_merge_prefers_detail_aid_cid_when_nonzero() {
        let ranked = make_ranked();
        let mut detail = make_detail();
        detail.aid = 999_999;
        detail.cid = 888_888;

        let ctx = make_ctx();
        let merged = build_video_detail(&ranked, &detail, &ctx);

        assert_eq!(merged.aid, 999_999);
        assert_eq!(merged.cid, 888_888);
    }

    #[test]
    fn test_merge_falls_back_to_ranked_aid_cid_when_zero() {
        let ranked = make_ranked();
        let mut detail = make_detail();
        detail.aid = 0;
        detail.cid = 0;

        let ctx = make_ctx();
        let merged = build_video_detail(&ranked, &detail, &ctx);

        assert_eq!(merged.aid, ranked.aid);
        assert_eq!(merged.cid, ranked.cid);
    }
}
