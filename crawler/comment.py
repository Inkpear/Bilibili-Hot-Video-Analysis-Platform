"""
评论采集：调用 /x/v2/reply，每视频上限 200 条。
返回采集结果，由调用方负责文件写入。
"""

import logging
import time

import requests

from config import MAX_COMMENTS, REPLY_PS, REPLY_SORT, DELAY
from config import API_BASE, HEADERS, COOKIE, TIMEOUT

logger = logging.getLogger("crawler")


def _reply_get(params):
    headers = dict(HEADERS)
    headers["Cookie"] = COOKIE
    resp = requests.get(
        f"{API_BASE}/x/v2/reply",
        params=params,
        headers=headers,
        timeout=TIMEOUT,
    )
    time.sleep(DELAY)
    resp.raise_for_status()
    return resp.json()


def collect_comments(bvid: str, aid: int, db) -> tuple[list[dict] | None, bool]:
    """
    采集单个视频的评论 (sort=热度, 上限 MAX_COMMENTS 条)。
    返回 (rows, success):
      - rows: 评论列表 (新增的，不含已采集的)
      - success: 是否完整采集未遇错误
    不写文件，由调用方负责。
    """
    rows = []
    page = 1
    success = True

    existing_ids = db.get_collected_comment_ids_for_bvid(bvid)
    remaining = MAX_COMMENTS - len(existing_ids)
    if remaining <= 0:
        return None, True

    while len(rows) < remaining:
        try:
            data = _reply_get(
                {"type": 1, "oid": aid, "pn": page, "ps": REPLY_PS, "sort": REPLY_SORT}
            )
        except Exception as e:
            logger.error(f"评论采集失败 {bvid}: {e}")
            success = False
            break

        code = data.get("code")
        if code == 12061:
            logger.info(f"  {bvid} 评论区关闭，按 0 条评论完成")
            break
        if code != 0:
            logger.error(f"评论采集失败 {bvid}: code={code} {data.get('message', '')}")
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
            existing_ids.add(comment_id)

            if len(rows) >= remaining:
                break

        page += 1

    return rows, success
