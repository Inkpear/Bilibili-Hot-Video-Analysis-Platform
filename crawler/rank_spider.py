"""
榜单采集：全站热门 + 9 分区排行。
"""

import json
import logging
import os

from config import (
    CATEGORY_RIDS, CATEGORY_NAMES,
    POPULAR_PS, POPULAR_PAGES,
    map_tname,
)
from utils import api_get_safe, timestamp_now_ms

logger = logging.getLogger("crawler")

RANK_SOURCE_POPULAR = "popular"
RANK_SOURCE_RANKING = "ranking"


def collect_popular(snapshot_id: str, db) -> list[dict]:
    """采集全站热门榜 (2页×50条)。"""
    rows = []
    collect_time = timestamp_now_ms()

    for pn in range(1, POPULAR_PAGES + 1):
        data = api_get_safe(
            "/x/web-interface/popular",
            {"pn": pn, "ps": POPULAR_PS},
            need_cookie=False,
        )
        if data is None:
            continue

        items = data.get("data", {}).get("list", [])
        for idx, item in enumerate(items):
            bvid = item.get("bvid", "")
            if not bvid:
                continue

            stat = item.get("stat", {})
            owner = item.get("owner", {})
            tname = item.get("tname", "")
            category = map_tname(tname)

            rank_position = (pn - 1) * POPULAR_PS + idx + 1

            row = {
                "bvid": bvid,
                "title": item.get("title", ""),
                "description": "",
                "category": category,
                "sub_category": tname,
                "tags": "",
                "duration": item.get("duration", 0),
                "publish_time": item.get("pubdate", 0),
                "up_id": str(owner.get("mid", "")),
                "up_name": owner.get("name", ""),
                "up_avatar": owner.get("face", ""),
                "url": f"https://www.bilibili.com/video/{bvid}",
                "aid": item.get("aid", 0),
                "cid": item.get("cid", 0),
                "rank_source": RANK_SOURCE_POPULAR,
                "rank_name": "全站热门",
                "view_count": stat.get("view", 0),
                "like_count": stat.get("like", 0),
                "coin_count": stat.get("coin", 0),
                "favorite_count": stat.get("favorite", 0),
                "share_count": stat.get("share", 0),
                "reply_count": stat.get("reply", 0),
                "danmaku_count": stat.get("danmaku", 0),
                "rank_position": rank_position,
                "snapshot_id": snapshot_id,
                "collect_time": collect_time,
            }
            rows.append(row)

    logger.info(f"  全站热门: {len(rows)} 条")
    return rows


def collect_rankings(snapshot_id: str, db) -> list[dict]:
    """采集 9 个分区排行。"""
    rows = []
    collect_time = timestamp_now_ms()

    for cat_name, rid in CATEGORY_RIDS.items():
        data = api_get_safe(
            "/x/web-interface/ranking/v2",
            {"rid": rid, "type": "all"},
            need_cookie=True,
        )
        if data is None:
            logger.warning(f"  分区 {cat_name}(rid={rid}) 采集失败，跳过")
            continue

        items = data.get("data", {}).get("list", [])
        for idx, item in enumerate(items):
            bvid = item.get("bvid", "")
            if not bvid:
                continue

            stat = item.get("stat", {})
            owner = item.get("owner", {})
            tname = item.get("tname", "")

            row = {
                "bvid": bvid,
                "title": item.get("title", ""),
                "description": "",
                "category": cat_name,
                "sub_category": tname,
                "tags": "",
                "duration": item.get("duration", 0),
                "publish_time": item.get("pubdate", 0),
                "up_id": str(owner.get("mid", "")),
                "up_name": owner.get("name", ""),
                "up_avatar": owner.get("face", ""),
                "url": f"https://www.bilibili.com/video/{bvid}",
                "aid": item.get("aid", 0),
                "cid": item.get("cid", 0),
                "rank_source": RANK_SOURCE_RANKING,
                "rank_name": f"{cat_name}榜",
                "view_count": stat.get("view", 0),
                "like_count": stat.get("like", 0),
                "coin_count": stat.get("coin", 0),
                "favorite_count": stat.get("favorite", 0),
                "share_count": stat.get("share", 0),
                "reply_count": stat.get("reply", 0),
                "danmaku_count": stat.get("danmaku", 0),
                "rank_position": idx + 1,
                "snapshot_id": snapshot_id,
                "collect_time": collect_time,
            }
            rows.append(row)

        logger.info(f"  分区 {cat_name}: {len(items)} 条")

    logger.info(f"  分区排行合计: {len(rows)} 条")
    return rows


def collect_all_ranks(snapshot_id: str, db) -> list[dict]:
    """采集所有榜单，返回 rank_video 行列表。"""
    all_rows = []
    all_rows.extend(collect_popular(snapshot_id, db))
    all_rows.extend(collect_rankings(snapshot_id, db))
    logger.info(f"  榜单总计: {len(all_rows)} 条")
    return all_rows


def write_rank_ndjson(rows: list[dict], snapshot_dir: str):
    """写入 rank_video.ndjson。"""
    os.makedirs(snapshot_dir, exist_ok=True)
    filepath = os.path.join(snapshot_dir, "rank_video.ndjson")
    with open(filepath, "w", encoding="utf-8") as f:
        for row in rows:
            f.write(json.dumps(row, ensure_ascii=False) + "\n")
    logger.info(f"  写入 {filepath} ({len(rows)} 行)")


def discover_new_items(rows: list[dict], snapshot_id: str, db):
    """
    从榜单行中发现新 bvid 和 up_id：
    - 新 bvid → 入 collected_videos + 入队评论/弹幕
    - 新 up_id → 标记待采集
    返回 (new_bvids, new_up_ids)
    """
    new_bvids = []
    new_up_ids = []

    seen_bvid = set()
    seen_upid = set()

    for row in rows:
        bvid = row["bvid"]
        up_id = row["up_id"]

        if bvid not in seen_bvid and not db.is_video_collected(bvid):
            seen_bvid.add(bvid)
            db.add_video(
                bvid=bvid,
                title=row["title"],
                category=row["category"],
                sub_category=row["sub_category"],
                aid=row["aid"],
                cid=row["cid"],
                duration=row["duration"],
                danmaku_count=row.get("danmaku_count", 0),
                snapshot_id=snapshot_id,
            )
            # 入队评论 + 弹幕任务
            if row["aid"]:
                db.enqueue_comment(bvid, row["aid"])
            if row["cid"]:
                db.enqueue_danmaku(bvid, row["cid"])
            new_bvids.append(bvid)

        if up_id and up_id not in seen_upid and not db.is_up_collected(up_id):
            seen_upid.add(up_id)
            new_up_ids.append(up_id)

    logger.info(f"  新发现: {len(new_bvids)} 个 bvid, {len(new_up_ids)} 个 up_id")
    return new_bvids, new_up_ids
