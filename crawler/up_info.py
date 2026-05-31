"""
UP 主信息采集：WBI 签名 /x/wbi/acc/info。
成功采集后标记 up_id 已采集；失败不标记，下次快照可重试。
"""

import json
import logging
import os

from tqdm import tqdm

from config import OUTPUT_DIR
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


def collect_up_info(up_ids: list[str], db, snapshot_dir: str, limit: int = 50):
    """采集一批 UP 主信息，成功后在 db 中标记。默认限制 50 个以防过量。"""
    if not up_ids:
        return

    if limit > 0 and len(up_ids) > limit:
        up_ids = up_ids[:limit]

    rows = []
    ok_count = 0
    skip_count = 0
    fail_count = 0

    pbar = tqdm(up_ids, desc="UP主信息", unit="个",
                postfix={"成功": 0, "失败": 0, "跳过": 0})
    with pbar:
        for up_id in pbar:
            if db.is_up_collected(up_id):
                skip_count += 1
                pbar.set_postfix({"成功": ok_count, "失败": fail_count, "跳过": skip_count})
                continue
            if db.should_skip_up(up_id):
                skip_count += 1
                pbar.set_postfix({"成功": ok_count, "失败": fail_count, "跳过": skip_count})
                continue

            info = fetch_up_info(up_id)
            if info is None:
                retry_count = db.record_up_failure(up_id, "fetch_up_info returned None")
                fail_count += 1
                pbar.set_postfix({"成功": ok_count, "失败": fail_count, "跳过": skip_count})
                continue

            db.add_up(up_id)
            rows.append(info)
            ok_count += 1
            pbar.set_postfix({"成功": ok_count, "失败": fail_count, "跳过": skip_count})

    if rows:
        os.makedirs(snapshot_dir, exist_ok=True)
        filepath = os.path.join(snapshot_dir, "up_info.ndjson")
        with open(filepath, "w", encoding="utf-8") as f:
            for row in rows:
                f.write(json.dumps(row, ensure_ascii=False) + "\n")
        logger.info(f"  UP 主信息: {len(rows)} 条 → {filepath}")
