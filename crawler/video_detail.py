"""
视频详情补充：调用 /x/web-interface/view 获取 description、tags。
"""

import json
import logging

from utils import api_get_safe

logger = logging.getLogger("crawler")


def fetch_video_detail(bvid: str) -> dict | None:
    """获取单个视频详情，返回补充字段 dict 或 None。"""
    data = api_get_safe(
        "/x/web-interface/view",
        {"bvid": bvid},
        need_cookie=True,
    )
    if data is None:
        return None

    d = data.get("data", {})
    if not d:
        return None

    tags_raw = d.get("tags") or []
    tags_str = ",".join(
        t.get("tag_name", "") for t in tags_raw if t.get("tag_name")
    )

    owner = d.get("owner", {})

    return {
        "bvid": d.get("bvid", bvid),
        "aid": d.get("aid", 0),
        "cid": d.get("cid", 0),
        "title": d.get("title", ""),
        "description": d.get("desc", ""),
        "tags": tags_str,
        "sub_category": d.get("tname", ""),
        "duration": d.get("duration", 0),
        "publish_time": d.get("pubdate", 0),
        "up_id": str(owner.get("mid", "")),
        "up_name": owner.get("name", ""),
        "up_avatar": owner.get("face", ""),
        "view_count": d.get("stat", {}).get("view", 0),
        "like_count": d.get("stat", {}).get("like", 0),
        "coin_count": d.get("stat", {}).get("coin", 0),
        "favorite_count": d.get("stat", {}).get("favorite", 0),
        "share_count": d.get("stat", {}).get("share", 0),
        "reply_count": d.get("stat", {}).get("reply", 0),
        "danmaku_count": d.get("stat", {}).get("danmaku", 0),
    }


def write_detail_ndjson_line(fp, detail: dict):
    """向已打开的文件追加写入一行视频详情。"""
    fp.write(json.dumps(detail, ensure_ascii=False) + "\n")
