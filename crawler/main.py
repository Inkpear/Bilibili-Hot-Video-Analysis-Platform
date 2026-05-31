#!/usr/bin/env python3
"""
crawler 入口 — 每 6 小时由 systemd timer 触发。

流程:
  1. 生成 snapshot_id
  2. 采集榜单 (popular + ranking) → rank_video.ndjson
  3. 发现新视频 → 补 /view 详情 → 入队评论/弹幕
  4. 发现新 UP 主 → 补 WBI 信息 → up_info.ndjson
  5. 消费评论队列 (上限 MAX_COMMENTS_PER_RUN)
  6. 消费弹幕队列 (上限 MAX_DANMAKU_PER_RUN)
  7. 写快照日志 + 统计输出

用法:
  python main.py                        # 正常采集
  python main.py --once                 # 同默认
  python main.py --dry-run              # 只采集榜单，不处理队列
  python main.py --queue-only           # 只处理队列，不采集榜单
"""

import argparse
import logging
import os
import sys
import time

from config import (
    OUTPUT_DIR, RUN_TIMEOUT_SECONDS,
    MAX_COMMENTS_PER_RUN, MAX_DANMAKU_PER_RUN,
)
from db import CrawlerDB
from utils import setup_logging, format_snapshot_id, logger as _root_logger
from rank_spider import (
    collect_all_ranks, write_rank_ndjson, discover_new_items,
)
from video_detail import supplement_video_details
from up_info import collect_up_info
from comment import process_comment_queue
from danmaku import process_danmaku_queue

logger = logging.getLogger("crawler")


def _check_timeout(t_start, step_name=""):
    """检查是否超过全局超时。返回 True 表示已超时。"""
    elapsed = time.time() - t_start
    if elapsed >= RUN_TIMEOUT_SECONDS:
        prefix = f"[{step_name}] " if step_name else ""
        logger.warning(
            f"  {prefix}全局超时 {elapsed:.0f}s/{RUN_TIMEOUT_SECONDS}s，"
            f"剩余任务留待下一轮"
        )
        return True
    return False


def run_snapshot(db: CrawlerDB, dry_run=False, max_new=-1):
    """执行一轮完整快照采集。"""
    snapshot_id = format_snapshot_id()
    snapshot_dir = os.path.join(OUTPUT_DIR, snapshot_id)
    t_start = time.time()

    logger.info(f"{'='*60}")
    logger.info(f"快照 {snapshot_id} 开始")
    logger.info(f"{'='*60}")

    db.start_snapshot(snapshot_id)
    rank_count = 0
    new_video_count = 0
    comments_done = 0
    danmaku_done = 0

    # ================================================================
    # Step 1: 采集榜单
    # ================================================================
    rows = collect_all_ranks(snapshot_id, db)
    rank_count = len(rows)
    logger.info(f"榜单采集耗时 {time.time()-t_start:.1f}s")

    if not rows:
        logger.warning("榜单无数据，跳过后续步骤")
        db.finish_snapshot(snapshot_id, rank_count=0, status="no_data")
        return

    # ================================================================
    # Step 2: 发现新视频 + UP 主
    # ================================================================
    new_bvids, new_up_ids = discover_new_items(rows, snapshot_id, db)
    new_video_count = len(new_bvids)

    # ================================================================
    # Step 3: 补充视频详情 (/view) — 先于 rank_video 写入
    # ================================================================
    if max_new == 0:
        logger.info(f"  新视频 {len(new_bvids)} 个, UP {len(new_up_ids)} 个 (跳过详情补充)")
        new_bvids = []
        new_up_ids = []
    elif max_new > 0:
        if len(new_bvids) > max_new or len(new_up_ids) > max_new:
            logger.info(f"  新视频 {len(new_bvids)} 个, UP {len(new_up_ids)} 个 → 限制前 {max_new}")
        new_bvids = new_bvids[:max_new]
        new_up_ids = new_up_ids[:max_new]

    processed_new_bvids = set(new_bvids)
    merge_map = {}

    if new_bvids:
        merge_map.update(supplement_video_details(new_bvids, db, snapshot_dir))
        logger.info(f"视频详情耗时 {time.time()-t_start:.1f}s")

    if new_up_ids:
        collect_up_info(new_up_ids, db, snapshot_dir)
        logger.info(f"UP主信息耗时 {time.time()-t_start:.1f}s")

    # ================================================================
    # Step 3b: 重试遗留详情 + 合并到 merge_map
    # ================================================================
    if not dry_run:
        stale_bvids = db.get_videos_missing_detail(exclude_bvids=processed_new_bvids)
        if stale_bvids:
            max_stale = 50
            if len(stale_bvids) > max_stale:
                stale_bvids = stale_bvids[:max_stale]
            logger.info(f"  重试 {len(stale_bvids)} 个遗留视频详情")
            merge_map.update(supplement_video_details(stale_bvids, db, snapshot_dir))

    # ================================================================
    # Step 3c: 把 description/tags 合并到 rank 行中，然后写入
    # ================================================================
    if merge_map:
        merged = 0
        for row in rows:
            patch = merge_map.get(row["bvid"])
            if patch:
                row["description"] = patch["description"]
                row["tags"] = patch["tags"]
                merged += 1
        logger.info(f"  合并 {merged} 条 description/tags (merge_map 共 {len(merge_map)} 条)")

    write_rank_ndjson(rows, snapshot_dir)

    # ================================================================
    # Step 5-6: 消费队列 (dry-run 模式跳过)
    # ================================================================
    if not dry_run:
        if _check_timeout(t_start, "评论队列前"):
            comments_done = 0
        else:
            comments_done = process_comment_queue(db, snapshot_dir,
                                                  limit=MAX_COMMENTS_PER_RUN)
            logger.info(f"评论队列耗时 {time.time()-t_start:.1f}s")

        if _check_timeout(t_start, "弹幕队列前"):
            danmaku_done = 0
        else:
            danmaku_done = process_danmaku_queue(db, snapshot_dir,
                                                  limit=MAX_DANMAKU_PER_RUN)
            logger.info(f"弹幕队列耗时 {time.time()-t_start:.1f}s")

    # ================================================================
    # 完成
    # ================================================================
    db.finish_snapshot(
        snapshot_id,
        rank_count=rank_count,
        new_video_count=new_video_count,
        comments_processed=comments_done,
        danmaku_processed=danmaku_done,
        status="ok",
    )

    total_time = time.time() - t_start
    stats = db.stats()

    logger.info(f"{'='*60}")
    logger.info(f"快照 {snapshot_id} 完成 ({total_time:.1f}s)")
    logger.info(f"  榜单: {rank_count} 条")
    logger.info(f"  新视频: {new_video_count} 个")
    logger.info(f"  新UP主: {len(new_up_ids)} 个")
    logger.info(f"  评论本轮: {comments_done} 视频")
    logger.info(f"  弹幕本轮: {danmaku_done} 视频")
    logger.info(f"  累计视频: {stats['videos']}")
    logger.info(f"  累计评论: {stats['comments']}")
    logger.info(f"  累计弹幕: {stats['danmaku']}")
    logger.info(f"  累计UP主: {stats['ups']}")
    logger.info(f"  队列评论: {stats['comment_queue']}")
    logger.info(f"  队列弹幕: {stats['danmaku_queue']}")
    logger.info(f"  处理中 评论: {stats['comment_processing']} 弹幕: {stats['danmaku_processing']}")
    logger.info(f"  失败 评论: {stats['comment_failed']} 弹幕: {stats['danmaku_failed']} UP: {stats['up_failed']}")
    logger.info(f"  输出目录: {snapshot_dir}")
    logger.info(f"{'='*60}")


def run_queue_only(db: CrawlerDB):
    """只处理评论/弹幕队列，不做榜单采集。"""
    snapshot_id = format_snapshot_id()
    snapshot_dir = os.path.join(OUTPUT_DIR, snapshot_id)
    t_start = time.time()

    logger.info(f"{'='*60}")
    logger.info(f"队列处理 {snapshot_id} 开始")
    logger.info(f"{'='*60}")

    c_done = process_comment_queue(db, snapshot_dir, limit=MAX_COMMENTS_PER_RUN)

    if _check_timeout(t_start, "弹幕队列前"):
        d_done = 0
    else:
        d_done = process_danmaku_queue(db, snapshot_dir, limit=MAX_DANMAKU_PER_RUN)

    stats = db.stats()
    elapsed = time.time() - t_start
    logger.info(f"队列处理完成 ({elapsed:.1f}s)")
    logger.info(f"  评论本轮: {c_done} 视频")
    logger.info(f"  弹幕本轮: {d_done} 视频")
    logger.info(f"  队列剩余 评论: {stats['comment_queue']} 弹幕: {stats['danmaku_queue']}")
    logger.info(f"  失败 评论: {stats['comment_failed']} 弹幕: {stats['danmaku_failed']}")


def main():
    parser = argparse.ArgumentParser(description="B站热门视频采集")
    parser.add_argument("--once", action="store_true", default=True,
                        help="单轮采集 (默认)")
    parser.add_argument("--dry-run", action="store_true",
                        help="只采集榜单，不处理队列")
    parser.add_argument("--queue-only", action="store_true",
                        help="只处理队列，不采集榜单")
    parser.add_argument("--max-new", type=int, default=-1,
                        help="限制新视频详情+UP主处理数量 (-1=全部, 0=跳过, N=限制N个)")
    parser.add_argument("--log-level", default="INFO",
                        choices=["DEBUG", "INFO", "WARNING", "ERROR"])
    parser.add_argument("--log-file", default=None,
                        help="日志文件路径 (默认仅输出到终端)")
    args = parser.parse_args()

    setup_logging(level=getattr(logging, args.log_level), log_file=args.log_file)

    # 检查 Cookie
    from config import COOKIE
    if not COOKIE:
        logger.error("环境变量 BILI_COOKIE 未设置")
        logger.error("  export BILI_COOKIE=\"SESSDATA=...; bili_jct=...; DedeUserID=...\"")
        sys.exit(1)

    logger.info(f"输出目录: {OUTPUT_DIR}")
    logger.info(f"COOKIE 长度: {len(COOKIE)} 字符")

    db = CrawlerDB()

    try:
        if args.queue_only:
            run_queue_only(db)
        else:
            run_snapshot(db, dry_run=args.dry_run, max_new=args.max_new)
    except Exception as e:
        logger.exception(f"采集异常: {e}")
        sys.exit(1)
    finally:
        db.close()


if __name__ == "__main__":
    main()
