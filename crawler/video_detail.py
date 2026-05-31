"""
视频详情补充：调用 /x/web-interface/view 获取 description、tags。
"""

import json
import logging
import os

from tqdm import tqdm

from config import OUTPUT_DIR
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


def supplement_video_details(bvids: list[str], db, snapshot_dir: str) -> dict[str, dict]:
    """
    为一批 bvid 补充视频详情。
    - 更新 db 中的 aid/cid/danmaku_count（若 rank 数据中缺失）
    - 标记 has_detail=1
    - 将详情写入 video_detail.ndjson
    - 返回 {bvid: detail_dict} 用于合并到 rank_video
    """
    if not bvids:
        return {}

    merge_map = {}
    patches = []
    ok_count = 0
    fail_count = 0

    pbar = tqdm(bvids, desc="视频详情", unit="个",
                postfix={"成功": 0, "失败": 0})
    with pbar:
        for bvid in pbar:
            detail = fetch_video_detail(bvid)
            if detail is None:
                fail_count += 1
                pbar.set_postfix({"成功": ok_count, "失败": fail_count})
                continue

            info = db.get_video_info(bvid)
            if info:
                updates = []
                if not info["aid"] and detail["aid"]:
                    updates.append(("aid", detail["aid"]))
                if not info["cid"] and detail["cid"]:
                    updates.append(("cid", detail["cid"]))
                if detail.get("danmaku_count", 0):
                    updates.append(("danmaku_count", detail["danmaku_count"]))
                for col, val in updates:
                    db._conn.execute(
                        f"UPDATE collected_videos SET {col}=? WHERE bvid=?",
                        (val, bvid)
                    )
                if updates:
                    db._conn.commit()

            db.mark_detail_done(bvid)
            merge_map[bvid] = {"description": detail["description"], "tags": detail["tags"]}
            patches.append(detail)
            ok_count += 1
            pbar.set_postfix({"成功": ok_count, "失败": fail_count})

    if patches:
        os.makedirs(snapshot_dir, exist_ok=True)
        filepath = os.path.join(snapshot_dir, "video_detail.ndjson")
        with open(filepath, "w", encoding="utf-8") as f:
            for p in patches:
                f.write(json.dumps(p, ensure_ascii=False) + "\n")
        logger.info(f"  视频详情: {len(patches)} 条 → {filepath}")

    return merge_map
