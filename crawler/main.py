#!/usr/bin/env python3
"""
crawler 入口 — 每天 0 点由 systemd timer 触发。

按视频事务模型：
  对每个新发现或有缺失的 bvid，按序采集 /view → UP → 评论 → 弹幕，
  全部成功则标完成，任何一项失败则 fail_count++（≥3 永久放弃）。
"""

import argparse
import json
import logging
import os
import sys
import time

from tqdm import tqdm

from config import OUTPUT_DIR, COOKIE
from db import CrawlerDB
from utils import setup_logging, format_snapshot_id
from rank_spider import (
    collect_all_ranks, write_rank_ndjson_line, discover_new_items,
)
from video_detail import fetch_video_detail, write_detail_ndjson_line
from up_info import fetch_up_info, write_up_ndjson_line
from comment import collect_comments
from danmaku import collect_danmaku

logger = logging.getLogger("crawler")


def process_video(bvid, row, db, snapshot_dir, detail_fp, up_fp):
    """
    处理单个视频事务。返回 (ok, status_parts, stats_delta)。

    采集顺序: /view → UP → 评论 → 弹幕。
    缺失项逐个补采，全部成功则标完成。
    UP 独立提交（即便视频其他项失败也写入）。
    """
    info = db.get_video_info(bvid)
    if info is None:
        return False, ["无数据"], {}

    ok = True
    status_parts = []
    delta = {"detail": 0, "up": 0, "comments": 0, "danmaku": 0}

    # ---- detail ----
    if info["has_detail"]:
        status_parts.append("详情⏭")
    else:
        detail = fetch_video_detail(bvid)
        if detail:
            row["description"] = detail["description"]
            row["tags"] = detail["tags"]
            if detail.get("aid") and not row.get("aid"):
                row["aid"] = detail["aid"]
            if detail.get("cid") and not row.get("cid"):
                row["cid"] = detail["cid"]
            if detail.get("aid") and not info["aid"]:
                db._conn.execute("UPDATE collected_videos SET aid=? WHERE bvid=?", (detail["aid"], bvid))
            if detail.get("cid") and not info["cid"]:
                db._conn.execute("UPDATE collected_videos SET cid=? WHERE bvid=?", (detail["cid"], bvid))
            if detail.get("danmaku_count", 0):
                db._conn.execute("UPDATE collected_videos SET danmaku_count=? WHERE bvid=?",
                                 (detail["danmaku_count"], bvid))
            db._conn.commit()
            db.mark_detail_done(bvid)
            write_detail_ndjson_line(detail_fp, detail)
            delta["detail"] = 1
            status_parts.append("详情✓")
        else:
            ok = False
            status_parts.append("详情✗")

    # ---- UP 主 (独立提交) ----
    up_id = row.get("up_id", "")
    if up_id and not db.is_up_collected(up_id) and not db.should_skip_up(up_id):
        up_info = fetch_up_info(up_id)
        if up_info:
            write_up_ndjson_line(up_fp, up_info)
            db.add_up(up_id)
            delta["up"] = 1
            status_parts.append("UP✓")
        else:
            db.record_up_failure(up_id, "fetch_up_info returned None")
            status_parts.append("UP✗")

    # ---- 评论 ----
    comment_aid = row.get("aid", 0)
    if info["has_comments"]:
        comments_ok = True
        status_parts.append("评论⏭")
    else:
        comments_rows, comments_ok = collect_comments(bvid, comment_aid, db)
        if comments_ok:
            comments_dir = os.path.join(snapshot_dir, "comments")
            os.makedirs(comments_dir, exist_ok=True)
            if comments_rows:
                with open(os.path.join(comments_dir, f"{bvid}.ndjson"), "w", encoding="utf-8") as f:
                    for cr in comments_rows:
                        f.write(json.dumps(cr, ensure_ascii=False) + "\n")
            db.mark_comments_done(bvid)
            delta["comments"] = 1
            status_parts.append(f"评论✓({len(comments_rows or [])}条)")
        else:
            ok = False
            status_parts.append("评论✗")

    # ---- 弹幕 ----
    danmaku_cid = row.get("cid", 0)
    if info["has_danmaku"]:
        danmaku_ok = True
        status_parts.append("弹幕⏭")
    else:
        danmaku_rows, danmaku_ok = collect_danmaku(bvid, danmaku_cid, db)
        if danmaku_ok:
            danmaku_dir = os.path.join(snapshot_dir, "danmaku")
            os.makedirs(danmaku_dir, exist_ok=True)
            if danmaku_rows:
                with open(os.path.join(danmaku_dir, f"{bvid}.ndjson"), "w", encoding="utf-8") as f:
                    for dr in danmaku_rows:
                        f.write(json.dumps(dr, ensure_ascii=False) + "\n")
            db.mark_danmaku_done(bvid)
            delta["danmaku"] = 1
            status_parts.append(f"弹幕✓({len(danmaku_rows or [])}条)")
        else:
            ok = False
            status_parts.append("弹幕✗")

    # ---- 提交/回滚 ----
    if ok:
        db.reset_fail_count(bvid)
        return True, status_parts, delta
    else:
        count = db.increment_fail_count(bvid)
        return False, status_parts, delta


def run_snapshot(db, dry_run=False, max_videos=0):
    """执行一轮完整快照采集。"""
    snapshot_id = format_snapshot_id()
    snapshot_dir = os.path.join(OUTPUT_DIR, snapshot_id)
    os.makedirs(snapshot_dir, exist_ok=True)
    t_start = time.time()

    logger.info(f"{'='*60}")
    logger.info(f"快照 {snapshot_id} 开始")
    logger.info(f"{'='*60}")

    db.start_snapshot(snapshot_id)

    # Step 1: 采集榜单
    rows = collect_all_ranks(snapshot_id, db)
    logger.info(f"榜单采集耗时 {time.time()-t_start:.1f}s")

    if not rows:
        logger.warning("榜单无数据，跳过后续步骤")
        db.finish_snapshot(snapshot_id, rank_count=0, status="no_data")
        return

    # Step 2: 发现新项目 + 重试列表
    new_bvids, retry_bvids, new_up_ids = discover_new_items(rows, snapshot_id, db)
    all_bvids = retry_bvids + new_bvids

    if max_videos > 0 and len(all_bvids) > max_videos:
        all_bvids = all_bvids[:max_videos]
        logger.info(f"  限制处理前 {max_videos} 个视频")

    row_map = {r["bvid"]: r for r in rows}

    # Step 3: 流式输出文件
    detail_fp = open(os.path.join(snapshot_dir, "video_detail.ndjson"), "w", encoding="utf-8")
    up_fp = open(os.path.join(snapshot_dir, "up_info.ndjson"), "w", encoding="utf-8")

    stats = {"✓": 0, "✗": 0, "描": 0, "评": 0, "弹": 0, "UP": 0}
    videos_ok = 0
    videos_failed = 0

    try:
        if all_bvids:
            pbar = tqdm(total=len(all_bvids), desc="视频", unit="个",
                        ncols=100, postfix=stats)
            with pbar:
                for i, bvid in enumerate(all_bvids):
                    row = row_map.get(bvid)
                    if row is None:
                        continue
                    if dry_run:
                        continue

                    ok, status_parts, delta = process_video(
                        bvid, row, db, snapshot_dir, detail_fp, up_fp
                    )

                    if ok:
                        videos_ok += 1
                        stats["✓"] += 1
                    else:
                        videos_failed += 1
                        stats["✗"] += 1
                    stats["描"] += delta["detail"]
                    stats["评"] += delta["comments"]
                    stats["弹"] += delta["danmaku"]
                    stats["UP"] += delta["up"]

                    pbar.write(f"→ {bvid}: {' '.join(status_parts)}")
                    pbar.update(1)
                    pbar.set_postfix(stats)
    finally:
        detail_fp.close()
        up_fp.close()

    # Step 4: 写入 rank_video.ndjson
    rank_path = os.path.join(snapshot_dir, "rank_video.ndjson")
    with open(rank_path, "w", encoding="utf-8") as f:
        for row in rows:
            write_rank_ndjson_line(f, row)
    logger.info(f"  写入 {rank_path} ({len(rows)} 行)")

    # Step 5: 重试遗留视频详情
    if not dry_run and max_videos == 0:
        stale_bvids = db.get_videos_missing_detail(exclude_bvids=set(all_bvids))
        if stale_bvids:
            max_stale = 50
            if len(stale_bvids) > max_stale:
                stale_bvids = stale_bvids[:max_stale]
            logger.info(f"  重试 {len(stale_bvids)} 个遗留视频详情")
            with open(os.path.join(snapshot_dir, "video_detail.ndjson"), "a", encoding="utf-8") as f:
                for bvid in stale_bvids:
                    detail = fetch_video_detail(bvid)
                    if detail:
                        row = row_map.get(bvid)
                        if row:
                            row["description"] = detail["description"]
                            row["tags"] = detail["tags"]
                        db.mark_detail_done(bvid)
                        db.reset_fail_count(bvid)
                        write_detail_ndjson_line(f, detail)

    # 完成
    db.finish_snapshot(
        snapshot_id,
        rank_count=len(rows),
        new_video_count=len(new_bvids),
        videos_processed=len(all_bvids),
        videos_ok=videos_ok,
        videos_failed=videos_failed,
        status="ok",
    )

    total_time = time.time() - t_start
    stats_final = db.stats()

    logger.info(f"{'='*60}")
    logger.info(f"快照 {snapshot_id} 完成 ({total_time:.1f}s)")
    logger.info(f"  榜单: {len(rows)} 条")
    logger.info(f"  处理: {len(all_bvids)} 视频 (新 {len(new_bvids)}, 重试 {len(retry_bvids)})")
    logger.info(f"  完成: {videos_ok}, 失败: {videos_failed}")
    logger.info(f"  描述: {stats['描']}, 评论: {stats['评']}, 弹幕: {stats['弹']}, UP: {stats['UP']}")
    logger.info(f"  累计视频: {stats_final['videos']} (ok={stats_final['videos_ok']}, 失败={stats_final['videos_failed']}, pending={stats_final['videos_pending']})")
    logger.info(f"  累计评论: {stats_final['comments']}, 弹幕: {stats_final['danmaku']}, UP: {stats_final['ups']}")
    logger.info(f"  描述覆盖: {stats_final['has_detail']}/{stats_final['videos']}")
    logger.info(f"  输出目录: {snapshot_dir}")
    logger.info(f"{'='*60}")


def main():
    parser = argparse.ArgumentParser(description="B站热门视频采集")
    parser.add_argument("--once", action="store_true", default=True,
                        help="单轮采集 (默认)")
    parser.add_argument("--dry-run", action="store_true",
                        help="只采集榜单，不处理视频")
    parser.add_argument("--max-videos", type=int, default=0,
                        help="限制处理视频数 (0=全部)")
    parser.add_argument("--log-level", default="INFO",
                        choices=["DEBUG", "INFO", "WARNING", "ERROR"])
    parser.add_argument("--log-file", default=None,
                        help="日志文件路径 (默认仅输出到终端)")
    args = parser.parse_args()

    setup_logging(level=getattr(logging, args.log_level), log_file=args.log_file)

    logger.info(f"输出目录: {OUTPUT_DIR}")
    logger.info(f"COOKIE 长度: {len(COOKIE)} 字符")

    db = CrawlerDB()

    try:
        run_snapshot(db, dry_run=args.dry_run, max_videos=args.max_videos)
    except Exception as e:
        logger.exception(f"采集异常: {e}")
        sys.exit(1)
    finally:
        db.close()


if __name__ == "__main__":
    main()
