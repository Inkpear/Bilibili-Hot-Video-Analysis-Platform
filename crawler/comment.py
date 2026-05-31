"""
评论队列消费：调用 /x/v2/reply，每视频上限 200 条。
"""

import json
import logging
import os
import time

from tqdm import tqdm

from config import (
    OUTPUT_DIR, MAX_COMMENTS, REPLY_PS, REPLY_SORT,
    MAX_COMMENTS_PER_RUN, PER_COMMENT_TIMEOUT,
)
from utils import api_get_safe

logger = logging.getLogger("crawler")


def collect_comments_for_bvid(bvid: str, aid: int, db, snapshot_dir: str) -> tuple[int, bool]:
    """
    采集单个视频的评论 (sort=热度, 上限 MAX_COMMENTS 条)。
    写入 comments/{bvid}.ndjson。
    返回 (count, success): count=采集条数, success=是否完整采集未遇错误。
    """
    rows = []
    page = 1
    collected_ids = []
    success = True
    t_start = time.time()

    existing_ids = db.get_collected_comment_ids_for_bvid(bvid)
    remaining = MAX_COMMENTS - len(existing_ids)
    if remaining <= 0:
        return 0, True

    while len(rows) < remaining:
        if time.time() - t_start > PER_COMMENT_TIMEOUT:
            logger.warning(f"  评论 {bvid}: 单视频超时 {PER_COMMENT_TIMEOUT}s")
            success = False
            break

        data = api_get_safe(
            "/x/v2/reply",
            {"type": 1, "oid": aid, "pn": page, "ps": REPLY_PS, "sort": REPLY_SORT},
            need_cookie=True,
        )
        if data is None:
            success = False
            break

        replies = data.get("data", {}).get("replies") or []
        if not replies:
            break

        for r in replies:
            comment_id = r.get("rpid", 0)
            if not comment_id:
                continue
            if comment_id in existing_ids:
                continue

            row = {
                "comment_id": comment_id,
                "bvid": bvid,
                "aid": aid,
                "user_id": r.get("mid", 0),
                "user_name": (r.get("member", {}) or {}).get("uname", ""),
                "content": (r.get("content", {}) or {}).get("message", ""),
                "like_count": r.get("like", 0),
                "reply_count": r.get("rcount", 0),
                "comment_time": r.get("ctime", 0),
            }
            rows.append(row)
            collected_ids.append((comment_id, bvid))
            existing_ids.add(comment_id)

            if len(rows) >= remaining:
                break

        page += 1

    if collected_ids:
        db.add_comment_ids_batch(collected_ids)

    if rows:
        comments_dir = os.path.join(snapshot_dir, "comments")
        os.makedirs(comments_dir, exist_ok=True)
        filepath = os.path.join(comments_dir, f"{bvid}.ndjson")
        with open(filepath, "w", encoding="utf-8") as f:
            for row in rows:
                f.write(json.dumps(row, ensure_ascii=False) + "\n")

    return len(rows), success


def process_comment_queue(db, snapshot_dir: str, limit: int = None) -> int:
    """
    消费评论队列。返回本次处理的视频数。
    """
    if limit is None:
        limit = MAX_COMMENTS_PER_RUN

    tasks = db.dequeue_comments(limit)
    if not tasks:
        return 0

    total = 0
    ok_count = 0
    fail_count = 0

    pbar = tqdm(tasks, desc="评论队列", unit="个",
                postfix={"成功": 0, "失败": 0})
    with pbar:
        for qid, bvid, aid in pbar:
            try:
                count, ok = collect_comments_for_bvid(bvid, aid, db, snapshot_dir)
                db.complete_comment(qid, ok)
                if ok:
                    db.mark_comments_done(bvid)
                    ok_count += 1
                else:
                    fail_count += 1
                logger.info(f"  评论 {bvid}: {count} 条 {'✓' if ok else '✗ 有错误'}")
                total += count
            except Exception as e:
                logger.error(f"  评论 {bvid} 失败: {e}")
                db.complete_comment(qid, False)
                fail_count += 1
            pbar.set_postfix({"成功": ok_count, "失败": fail_count})

    logger.info(f"  评论本轮: {len(tasks)} 视频, {total} 条")
    return len(tasks)
