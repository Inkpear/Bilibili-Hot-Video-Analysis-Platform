"""
评论采集：调用 /x/v2/reply，每视频上限 200 条。
返回采集结果，由调用方负责文件写入。
"""

import logging

from config import MAX_COMMENTS, REPLY_PS, REPLY_SORT
from utils import api_get_safe

logger = logging.getLogger("crawler")


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
    collected_ids = []
    success = True

    existing_ids = db.get_collected_comment_ids_for_bvid(bvid)
    remaining = MAX_COMMENTS - len(existing_ids)
    if remaining <= 0:
        return None, True

    while len(rows) < remaining:
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

    return rows, success
