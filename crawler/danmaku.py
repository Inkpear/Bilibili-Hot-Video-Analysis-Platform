"""
弹幕队列消费：调用 seg.so 段式 API (protobuf)，每视频上限 3000 条。
"""

import json
import logging
import os
import time

import requests
from tqdm import tqdm

from config import (
    HEADERS, DELAY, TIMEOUT,
    OUTPUT_DIR, MAX_DANMAKU,
    MAX_DANMAKU_PER_RUN,
    PER_DANMAKU_TIMEOUT,
    DANMAKU_429_COOLDOWN,
    MAX_CONSECUTIVE_DANMAKU_429,
)
from utils import parse_danmaku_seg

logger = logging.getLogger("crawler")

# ---- 全局 429 冷却状态 ----
_consecutive_429 = 0
_cooldown_until = 0.0


def _check_global_cooldown():
    """检查是否处于全局 429 冷却期。返回 True 表示当前应跳过弹幕采集。"""
    global _consecutive_429, _cooldown_until
    if _cooldown_until > time.time():
        return True
    if _cooldown_until and _cooldown_until <= time.time():
        _cooldown_until = 0.0
        _consecutive_429 = 0
        logger.info("  弹幕全局冷却已解除")
    return False


def _record_429():
    """记录一次 429，超过阈值进入全局冷却。"""
    global _consecutive_429, _cooldown_until
    _consecutive_429 += 1
    if _consecutive_429 >= MAX_CONSECUTIVE_DANMAKU_429:
        _cooldown_until = time.time() + DANMAKU_429_COOLDOWN
        logger.warning(
            f"  弹幕连续 {_consecutive_429} 次 429 → 全局冷却 {DANMAKU_429_COOLDOWN}s"
        )


def _reset_429_counter():
    """成功段后重置 429 计数器。"""
    global _consecutive_429
    if _consecutive_429 > 0:
        _consecutive_429 = 0


def collect_danmaku_for_bvid(bvid: str, cid: int, db, snapshot_dir: str) -> tuple[int, bool]:
    """
    采集单个视频的弹幕 (段式 seg.so, 上限 MAX_DANMAKU 条)。
    逐段拉取直到无数据或达到上限。
    写入 danmaku/{bvid}.ndjson。
    返回 (count, success): count=采集条数, success=是否完整采集未遇错误。
    """
    if _check_global_cooldown():
        return 0, False

    rows = []
    segment = 1
    collected_ids = []
    success = True
    t_start = time.time()

    existing_ids = db.get_collected_danmaku_ids_for_bvid(bvid)
    remaining = MAX_DANMAKU - len(existing_ids)
    if remaining <= 0:
        return 0, True

    while len(rows) < remaining:
        if time.time() - t_start > PER_DANMAKU_TIMEOUT:
            logger.warning(f"  弹幕 {bvid}: 单视频超时 {PER_DANMAKU_TIMEOUT}s")
            success = False
            break

        resp = None
        for retry in range(3):
            try:
                resp = requests.get(
                    "https://api.bilibili.com/x/v2/dm/web/seg.so",
                    params={"type": 1, "oid": cid, "segment_index": segment},
                    headers=HEADERS,
                    timeout=TIMEOUT,
                )
                time.sleep(DELAY)
            except requests.exceptions.RequestException as e:
                logger.warning(f"  弹幕 seg={segment} HTTP 错误: {e}")
                success = False
                break

            if resp.status_code in (412, 429):
                _record_429()
                if _check_global_cooldown():
                    logger.warning(f"  弹幕 {bvid}: 全局冷却中，中断当前视频")
                    success = False
                    break
                if retry >= 2:
                    logger.warning(f"  弹幕 seg={segment} 限流重试耗尽")
                    success = False
                    break
                wait = 5 * (2 ** retry)
                logger.warning(
                    f"  弹幕 限流 {resp.status_code} 等待 {wait}s "
                    f"(seg={segment} retry={retry+1}/3)"
                )
                time.sleep(wait)
                continue
            break

        if resp is None:
            break

        if resp.status_code != 200:
            if resp.status_code in (412, 429):
                break
            if resp.status_code == 304:
                logger.debug(f"  弹幕 seg={segment} 304 (无更多数据)")
            else:
                logger.warning(f"  弹幕 seg={segment} HTTP {resp.status_code}")
                success = False
            break

        elems = parse_danmaku_seg(resp.content)
        _reset_429_counter()

        if not elems:
            break

        for e in elems:
            danmaku_id = e.get("id", 0)
            if not danmaku_id:
                continue
            if danmaku_id in existing_ids:
                continue

            row = {
                "danmaku_id": danmaku_id,
                "bvid": bvid,
                "cid": cid,
                "content": e.get("content", ""),
                "video_time": round(e.get("progress", 0) / 1000.0, 3),
                "mode": e.get("mode"),
                "fontsize": e.get("fontsize"),
                "color": e.get("color"),
                "mid_hash": e.get("mid_hash", ""),
                "send_time": e.get("ctime"),
            }
            rows.append(row)
            collected_ids.append((danmaku_id, bvid))
            existing_ids.add(danmaku_id)

            if len(rows) >= remaining:
                break

        segment += 1
        if segment > 100:
            break

    if collected_ids:
        db.add_danmaku_ids_batch(collected_ids)

    if rows:
        danmaku_dir = os.path.join(snapshot_dir, "danmaku")
        os.makedirs(danmaku_dir, exist_ok=True)
        filepath = os.path.join(danmaku_dir, f"{bvid}.ndjson")
        with open(filepath, "w", encoding="utf-8") as f:
            for row in rows:
                f.write(json.dumps(row, ensure_ascii=False) + "\n")

    return len(rows), success


def process_danmaku_queue(db, snapshot_dir: str, limit: int = None) -> int:
    """
    消费弹幕队列。返回本次处理的视频数。
    """
    if limit is None:
        limit = MAX_DANMAKU_PER_RUN

    # 全局冷却中不消耗队列
    if _check_global_cooldown():
        logger.info("  弹幕全局冷却中，本轮跳过")
        return 0

    tasks = db.dequeue_danmaku(limit)
    if not tasks:
        return 0

    total = 0
    ok_count = 0
    fail_count = 0
    skip_count = 0

    pbar = tqdm(tasks, desc="弹幕队列", unit="个",
                postfix={"成功": 0, "失败": 0, "跳过": 0})
    with pbar:
        for qid, bvid, cid in pbar:
            try:
                count, ok = collect_danmaku_for_bvid(bvid, cid, db, snapshot_dir)
                info = db.get_video_info(bvid)
                expected = info.get("danmaku_count", 0) if info else 0
                if ok and expected > 50 and count == 0:
                    logger.warning(f"  弹幕 {bvid}: 0 条但视频应有 {expected} 条弹幕 → 标记失败")
                    ok = False
                db.complete_danmaku(qid, ok)
                if ok:
                    db.mark_danmaku_done(bvid)
                    if count == 0:
                        skip_count += 1
                    else:
                        ok_count += 1
                else:
                    fail_count += 1
                logger.info(f"  弹幕 {bvid}: {count} 条 (应有{expected}) {'✓' if ok else '✗ 有错误'}")
                total += count
            except Exception as e:
                logger.error(f"  弹幕 {bvid} 失败: {e}")
                db.complete_danmaku(qid, False)
                fail_count += 1

            if _check_global_cooldown():
                logger.info(f"  弹幕队列: 全局冷却触发，本轮已处理 {total} 条，停止")
                break
            pbar.set_postfix({"成功": ok_count, "失败": fail_count, "跳过": skip_count})
    logger.info(f"  弹幕本轮: {len(tasks)} 视频, {total} 条")
    return len(tasks)
