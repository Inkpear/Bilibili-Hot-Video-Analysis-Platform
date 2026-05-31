"""
UP 主信息采集：WBI 签名 /x/wbi/acc/info。
"""

import json
import logging

from utils import api_get_safe

logger = logging.getLogger("crawler")


def fetch_up_info(up_id: str) -> dict | None:
    """获取单个 UP 主信息。"""
    try:
        mid = int(up_id)
    except (ValueError, TypeError):
        return None

    data = api_get_safe(
        "/x/space/wbi/acc/info",
        {"mid": mid},
        need_cookie=True,
        wbi_sign=True,
    )
    if data is None:
        return None

    d = data.get("data", {})
    if not d:
        return None

    official = d.get("official", {}) or {}
    relation = api_get_safe(
        "/x/relation/stat",
        {"vmid": mid},
        need_cookie=True,
    ) or {}
    arc = api_get_safe(
        "/x/space/wbi/arc/search",
        {"mid": mid, "pn": 1, "ps": 1},
        need_cookie=True,
        wbi_sign=True,
    ) or {}

    return {
        "up_id": str(d.get("mid", mid)),
        "up_name": d.get("name", ""),
        "up_avatar": d.get("face", ""),
        "follower_count": (relation.get("data") or {}).get("follower", 0),
        "video_count": ((arc.get("data") or {}).get("page") or {}).get("count", 0),
        "sign": d.get("sign", ""),
        "official_verify": official.get("desc", ""),
    }


def write_up_ndjson_line(fp, info: dict):
    """向已打开的文件追加写入一行 UP 主信息。"""
    fp.write(json.dumps(info, ensure_ascii=False) + "\n")
