use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use indicatif::ProgressStyle;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tracing::{error, info, info_span, warn};
use tracing_indicatif::span_ext::IndicatifSpanExt;

use crate::api::ranking::{self, RankedVideo};
use crate::api::{comment, danmaku, up, video};
use crate::config::Config;
use crate::error::{CrawlerError, Result};
use crate::models::VideoDetail;
use crate::scheduler::queue::Task;
use crate::state::StateManager;
use crate::storage::NdjsonWriter;
use crate::worker::Worker;

// ---------------------------------------------------------------------------
// Snapshot context
// ---------------------------------------------------------------------------

pub(crate) struct SnapshotContext {
    pub snapshot_id: String,
    pub snapshot_dir: std::path::PathBuf,
    pub collect_time: i64,
}

// ---------------------------------------------------------------------------
// Merge helper
// ---------------------------------------------------------------------------

fn build_video_detail(
    ranked: &RankedVideo,
    detail: &VideoDetail,
    ctx: &SnapshotContext,
) -> VideoDetail {
    VideoDetail {
        bvid: detail.bvid.clone(),
        aid: if detail.aid != 0 {
            detail.aid
        } else {
            ranked.aid
        },
        cid: if detail.cid != 0 {
            detail.cid
        } else {
            ranked.cid
        },
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
// Main orchestrator
// ---------------------------------------------------------------------------

pub async fn run_crawl(config: &Config, state: &Arc<StateManager>) -> Result<()> {
    // 1. Create workers from accounts
    let mut workers: Vec<Worker> = Vec::new();
    for (idx, account) in config.accounts.iter().enumerate() {
        let worker = Worker::create(
            idx,
            account,
            &config.api.base_url,
            config.crawler.delay.min_ms,
            config.crawler.delay.max_ms,
        )
        .await?;
        workers.push(worker);
    }

    if workers.is_empty() {
        return Err(CrawlerError::Config("no accounts configured".into()));
    }

    info!(worker_count = workers.len(), "Workers created");

    // 2. Create snapshot
    let writer = NdjsonWriter::new(&config.crawler.output_dir);
    let snapshot_dir = writer.create_snapshot_dir()?;
    let snapshot_id = snapshot_dir
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let collect_time = chrono::Utc::now().timestamp();
    let ctx = Arc::new(SnapshotContext {
        snapshot_id: snapshot_id.clone(),
        snapshot_dir,
        collect_time,
    });

    state.create_snapshot(&snapshot_id)?;
    state.reset_incomplete_flags()?;
    info!(snapshot_id = %snapshot_id, "snapshot created, incomplete flags reset");

    // 3. Phase 1: Fetch rankings with first worker
    let first_worker = &workers[0];
    let cookie = first_worker.cookie();

    let ranking_span = info_span!("ranking");
    ranking_span.pb_set_length(ranking::PARTITION_RIDS.len() as u64);
    ranking_span.pb_set_style(
        &ProgressStyle::with_template("{prefix:>10} {bar} {pos}/{len} partitions").unwrap(),
    );

    let ranked_videos = {
        let _guard = ranking_span.enter();
        ranking::fetch_all_rankings(
            &first_worker.client,
            &cookie,
            &config.api.base_url,
            first_worker.wbi(),
            0,
        )
        .await?
    };
    drop(ranking_span);

    info!(count = ranked_videos.len(), "rankings fetched");

    // 4. Register videos in state
    for (i, rv) in ranked_videos.iter().enumerate() {
        state.insert_video(
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
        if (i + 1) % 200 == 0 {
            info!(
                registered = i + 1,
                total = ranked_videos.len(),
                "registering videos"
            );
        }
    }
    info!(
        total = ranked_videos.len(),
        "all videos registered in state"
    );

    // 5. Phase 2: Semaphore + per-task spawn
    let concurrency = workers.len();
    let semaphore = Arc::new(Semaphore::new(concurrency));
    let pool = Arc::new(StdMutex::new(VecDeque::from(workers)));
    let writer = Arc::new(writer);
    let max_retries = config.limits.max_retries_per_task;

    // Collect tasks first to get the count
    let mut tasks_to_spawn: Vec<Task> = Vec::new();
    for rv in ranked_videos.iter() {
        if state.video_done(&rv.bvid)? {
            continue;
        }
        let tasks = state.get_video_tasks_needed(&rv.bvid, &rv.up_id)?;
        if tasks.needs_detail {
            tasks_to_spawn.push(Task::VideoDetail {
                bvid: rv.bvid.clone(),
                ranked: Box::new(rv.clone()),
            });
        }
        if tasks.needs_up && !state.should_skip_up(&rv.up_id)? {
            tasks_to_spawn.push(Task::UpInfo {
                up_id: rv.up_id.clone(),
                bvid: rv.bvid.clone(),
            });
        }
        if tasks.needs_comments {
            tasks_to_spawn.push(Task::Comments {
                bvid: rv.bvid.clone(),
                aid: rv.aid,
            });
        }
        if tasks.needs_danmaku {
            tasks_to_spawn.push(Task::Danmaku {
                bvid: rv.bvid.clone(),
                cid: rv.cid,
                danmaku_count: rv.danmaku_count,
            });
        }
    }

    let task_count = tasks_to_spawn.len();
    info!(tasks = task_count, "all tasks collected");

    if task_count == 0 {
        info!("no tasks to process");
        state.complete_snapshot(&snapshot_id, 0, 0, 0, 0, "ok")?;
        return Ok(());
    }

    // Create tasks span with progress bar
    let tasks_span = info_span!("tasks", total = task_count);
    tasks_span.pb_set_length(task_count as u64);
    tasks_span.pb_set_style(
        &ProgressStyle::with_template("{prefix:>10} {bar} {pos}/{len} ({percent}%)").unwrap(),
    );
    let _tasks_guard = tasks_span.enter();

    // Spawn all tasks into a JoinSet (completion-order processing)
    let mut join_set = JoinSet::new();
    for task in tasks_to_spawn {
        spawn_task_into(
            task,
            &pool,
            &semaphore,
            state,
            &writer,
            &ctx,
            max_retries,
            &mut join_set,
        );
    }

    // 6. Wait for tasks in completion order — progress bar updates immediately
    let mut success = 0u64;
    let mut failed = 0u64;
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(Ok(())) => success += 1,
            Ok(Err(e)) => {
                warn!(error = %e, "task returned error");
                failed += 1;
            }
            Err(e) => {
                warn!(error = %e, "task panicked");
                failed += 1;
            }
        }
        tasks_span.pb_inc(1);
    }

    tasks_span.pb_set_finish_message(&format!("done ✓{} ✗{}", success, failed));
    drop(_tasks_guard);
    drop(tasks_span);
    info!(success, failed, "all tasks completed");

    // 7. Collect stats
    let video_count = count_files_in_dir(&ctx.snapshot_dir.join("video_detail")) as i64;
    let comment_count = count_files_in_dir(&ctx.snapshot_dir.join("comments")) as i64;
    let danmaku_count = count_files_in_dir(&ctx.snapshot_dir.join("danmaku")) as i64;
    let up_count = count_files_in_dir(&ctx.snapshot_dir.join("up_info")) as i64;

    state.complete_snapshot(
        &snapshot_id,
        video_count,
        comment_count,
        danmaku_count,
        up_count,
        "ok",
    )?;

    info!(
        snapshot_id = %snapshot_id,
        videos = video_count,
        comments = comment_count,
        danmaku = danmaku_count,
        ups = up_count,
        "crawl completed"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// RAII worker guard — guarantees worker is returned to pool on any exit path
// ---------------------------------------------------------------------------

struct WorkerGuard {
    worker: Option<Worker>,
    pool: Arc<StdMutex<VecDeque<Worker>>>,
}

impl WorkerGuard {
    fn worker_mut(&mut self) -> &mut Worker {
        self.worker.as_mut().unwrap()
    }

    fn return_worker(mut self) {
        if let Some(w) = self.worker.take() {
            self.pool.lock().unwrap().push_back(w);
        }
    }
}

impl Drop for WorkerGuard {
    fn drop(&mut self) {
        if let Some(w) = self.worker.take() {
            self.pool.lock().unwrap().push_back(w);
        }
    }
}

// ---------------------------------------------------------------------------
// Task runner (with internal retry loop)
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn spawn_task_into(
    task: Task,
    pool: &Arc<StdMutex<VecDeque<Worker>>>,
    semaphore: &Arc<Semaphore>,
    state: &Arc<StateManager>,
    writer: &Arc<NdjsonWriter>,
    ctx: &Arc<SnapshotContext>,
    max_retries: u32,
    join_set: &mut JoinSet<Result<()>>,
) {
    let pool = pool.clone();
    let semaphore = semaphore.clone();
    let state = state.clone();
    let writer = writer.clone();
    let ctx = ctx.clone();

    let bvid = task.bvid().to_string();
    let task_name = task.task_type_name();
    let span = info_span!("task", bvid = %bvid, task = task_name);

    join_set.spawn(async move {
        let _guard = span.enter();
        let mut fail_count = 0u32;
        let mut rate_limit_count = 0u32;

        loop {
            let permit = semaphore.acquire().await.map_err(|e| {
                CrawlerError::Permanent(format!("semaphore closed: {e}"))
            })?;

            let worker = pool.lock().unwrap().pop_front().expect("worker pool empty");
            let mut guard = WorkerGuard {
                worker: Some(worker),
                pool: pool.clone(),
            };

            guard.worker_mut().rate_limiter.delay().await;

            info!(worker = guard.worker_mut().id, "executing");

            let result =
                execute_task(guard.worker_mut(), &task, &state, &writer, &ctx).await;

            let worker = guard.worker_mut();
            match &result {
                Ok(()) => worker.record_success(),
                Err(CrawlerError::RateLimited { retry_after }) => {
                    let cooldown = retry_after.unwrap_or(60);
                    worker.record_rate_limit(cooldown);
                }
                Err(_) => worker.record_failure(),
            }

            guard.return_worker();
            drop(permit);

            match result {
                Ok(()) => {
                    info!("done");
                    return Ok(());
                }
                Err(CrawlerError::RateLimited { retry_after }) => {
                    rate_limit_count += 1;
                    if rate_limit_count >= max_retries {
                        warn!(count = rate_limit_count, "rate limit exceeded max retries, will retry next run");
                        return Ok(());
                    }
                    let cooldown = retry_after.unwrap_or(60);
                    warn!(cooldown = cooldown, count = rate_limit_count, "rate limited, retrying");
                    tokio::time::sleep(Duration::from_secs(cooldown)).await;
                }
                Err(CrawlerError::Permanent(msg)) => {
                    warn!(reason = %msg, "permanent failure, skipping");
                    mark_task_done(&task, &state);
                    return Ok(());
                }
                Err(e) => {
                    fail_count += 1;
                    if fail_count >= max_retries {
                        error!(error = %e, fail_count = fail_count, "task failed permanently, will retry next run");
                        return Ok(());
                    }
                    warn!(error = %e, fail_count = fail_count, "retrying task");
                    let delay = rand::random::<u64>() % 2000;
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Task execution
// ---------------------------------------------------------------------------

async fn execute_task(
    worker: &mut Worker,
    task: &Task,
    state: &Arc<StateManager>,
    writer: &Arc<NdjsonWriter>,
    ctx: &SnapshotContext,
) -> Result<()> {
    let cookie = worker.cookie();
    let base_url = "https://api.bilibili.com";

    match task {
        Task::VideoDetail { bvid, ranked } => {
            let mut detail =
                video::fetch_video_detail(&worker.client, &cookie, base_url, bvid).await?;
            match video::fetch_tags(&worker.client, &cookie, base_url, bvid).await {
                Ok(tags) if !tags.is_empty() => detail.tags = tags,
                _ => {}
            }
            state.mark_detail_done(bvid)?;

            let merged = build_video_detail(ranked, &detail, ctx);
            writer.write_ndjson(
                &ctx.snapshot_dir
                    .join("video_detail")
                    .join(format!("{}.ndjson", bvid)),
                &[merged],
            )?;

            Ok(())
        }

        Task::UpInfo { up_id, bvid: _ } => {
            let up_info =
                up::fetch_up_info(&worker.client, &cookie, base_url, worker.wbi(), up_id)
                    .await?;
            writer.write_ndjson(
                &ctx.snapshot_dir
                    .join("up_info")
                    .join(format!("{}.ndjson", up_id)),
                &[up_info],
            )?;
            state.mark_up_collected(up_id)?;
            Ok(())
        }

        Task::Comments { bvid, aid } => {
            let existing: HashSet<i64> =
                state.get_collected_comment_ids(bvid)?.into_iter().collect();
            let (new_comments, _) = comment::collect_comments(
                &worker.client,
                &cookie,
                base_url,
                bvid,
                *aid,
                200,
                &existing,
            )
            .await?;
            if !new_comments.is_empty() {
                let ids: Vec<(i64, &str)> = new_comments
                    .iter()
                    .map(|c| (c.comment_id, bvid.as_str()))
                    .collect();
                state.insert_comments_batch(&ids)?;
                writer.write_ndjson(
                    &ctx.snapshot_dir
                        .join("comments")
                        .join(format!("{}.ndjson", bvid)),
                    &new_comments,
                )?;
            }
            state.mark_comments_done(bvid)?;
            Ok(())
        }

        Task::Danmaku {
            bvid,
            cid,
            danmaku_count,
        } => {
            let start_seg = state.get_last_danmaku_seg(bvid)?;
            let existing: HashSet<i64> =
                state.get_collected_danmaku_ids(bvid)?.into_iter().collect();
            let (new_danmaku, last_seg) = danmaku::collect_danmaku(
                &worker.client,
                &cookie,
                bvid,
                *cid,
                *danmaku_count,
                3000,
                start_seg,
                &existing,
            )
            .await?;
            if !new_danmaku.is_empty() {
                let ids: Vec<(i64, &str)> = new_danmaku
                    .iter()
                    .map(|d| (d.danmaku_id, bvid.as_str()))
                    .collect();
                state.insert_danmaku_batch(&ids)?;
                writer.append_ndjson(
                    &ctx.snapshot_dir
                        .join("danmaku")
                        .join(format!("{}.ndjson", bvid)),
                    &new_danmaku,
                )?;
            }
            state.update_danmaku_seg(bvid, last_seg)?;
            let total = existing.len() + new_danmaku.len();
            if total >= 3000 || last_seg > 0 {
                state.mark_danmaku_done(bvid)?;
            }
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn mark_task_done(task: &Task, state: &Arc<StateManager>) {
    let result = match task {
        Task::VideoDetail { bvid, .. } => state.mark_detail_done(bvid),
        Task::UpInfo { up_id, .. } => state.mark_up_collected(up_id),
        Task::Comments { bvid, .. } => state.mark_comments_done(bvid),
        Task::Danmaku { bvid, .. } => state.mark_danmaku_done(bvid),
    };
    if let Err(e) = result {
        tracing::error!(error = %e, "failed to mark task done");
    }
}

fn count_files_in_dir(dir: &std::path::Path) -> usize {
    std::fs::read_dir(dir)
        .map(|rd| rd.filter_map(|e| e.ok()).count())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex as StdMutex};
    use std::time::Duration;
    use tokio::sync::Semaphore;

    #[tokio::test]
    async fn test_semaphore_limits_concurrency() {
        let semaphore = Arc::new(Semaphore::new(2));
        let active = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let max_active = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let mut handles = Vec::new();
        for _ in 0..10 {
            let sem = semaphore.clone();
            let active = active.clone();
            let max_active = max_active.clone();
            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                let cur = active.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                max_active.fetch_max(cur, std::sync::atomic::Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(10)).await;
                active.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            }));
        }
        for h in handles {
            h.await.unwrap();
        }
        assert!(max_active.load(std::sync::atomic::Ordering::SeqCst) <= 2);
    }

    #[tokio::test]
    async fn test_worker_pool_pop_push() {
        let pool = Arc::new(StdMutex::new(VecDeque::from(vec![1, 2, 3])));
        let semaphore = Arc::new(Semaphore::new(3));

        let mut handles = Vec::new();
        for _ in 0..6 {
            let pool = pool.clone();
            let sem = semaphore.clone();
            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                let mut w = pool.lock().unwrap().pop_front().unwrap();
                w += 10;
                tokio::time::sleep(Duration::from_millis(5)).await;
                pool.lock().unwrap().push_back(w);
            }));
        }
        for h in handles {
            h.await.unwrap();
        }
        let pool = pool.lock().unwrap();
        assert_eq!(pool.len(), 3);
        assert_eq!(pool.iter().sum::<i32>(), 21 + 22 + 23);
    }
}
