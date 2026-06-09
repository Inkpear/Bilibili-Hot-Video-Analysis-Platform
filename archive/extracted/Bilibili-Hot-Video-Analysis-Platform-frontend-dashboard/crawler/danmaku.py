"""
弹幕采集：调用 seg.so 段式 API (protobuf)，每视频上限 3000 条。
返回采集结果，由调用方负责文件写入。
"""

import logging
import time

import requests

from config import (
    HEADERS, DELAY, TIMEOUT,
    MAX_DANMAKU,
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


def collect_danmaku(bvid: str, cid: int, db) -> tuple[list[dict] | None, bool]:
    """
    采集单个视频的弹幕 (段式 seg.so, 上限 MAX_DANMAKU 条)。
    逐段拉取直到无数据或达到上限。
    返回 (rows, success):
      - rows: 弹幕列表 (新增的，不含已采集的)
      - success: 是否完整采集未遇错误
    不写文件，由调用方负责。
    """
    if _check_global_cooldown():
        return None, False

    rows = []
    segment = 1
    success = True

    existing_ids = db.get_collected_danmaku_ids_for_bvid(bvid)
    remaining = MAX_DANMAKU - len(existing_ids)
    if remaining <= 0:
        return None, True

    while len(rows) < remaining:
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

        if len(resp.content) <= 39:
            logger.warning(
                f"  弹幕 seg={segment}: 空响应 ({len(resp.content)} bytes)，IP限流，停止"
            )
            _record_429()
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
            existing_ids.add(danmaku_id)

            if len(rows) >= remaining:
                break

        segment += 1
        if segment > 100:
            break

    return rows, success
