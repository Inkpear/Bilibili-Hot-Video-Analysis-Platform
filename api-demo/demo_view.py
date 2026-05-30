"""
demo_view.py — 视频详情（补充 description, tags）

    python demo_view.py BV1iQVG6AEtS

popular/ranking 接口已包含 stat 数据，此接口仅用于获取:
    - description（视频简介，d["desc"]）
    - tags（视频标签，d["tags"] 是 [{tag_name, ...}, ...] 列表）
    - aid / cid（用于评论和弹幕接口的 oid）

注意：
    - category = tname 是 API 返回的子分区（如"MAD·AMV"），不是一级分区
    - crawler 需自行维护 tname → 一级分区（动画/游戏/…）的映射表
    - tags 为列表结构，crawler 需提取 tag_name 拼接为逗号分隔字符串

需要 Cookie。
"""
import json
import sys

from utils import api_get, get_bvid_from_popular

bvid = sys.argv[1] if len(sys.argv) > 1 else get_bvid_from_popular()

data = api_get("/x/web-interface/view", {"bvid": bvid}, need_cookie=True)
d = data["data"]

tags_raw = d.get("tags") or []
tags_str = ",".join(t.get("tag_name", "") for t in tags_raw if t.get("tag_name"))

row = {
    "bvid": d["bvid"],
    "aid": d["aid"],
    "cid": d["cid"],
    "title": d["title"],
    "description": d.get("desc", ""),
    "tags": tags_str,
    "sub_category": d.get("tname", ""),
    "duration": d.get("duration", 0),
    "publish_time": d.get("pubdate", 0),
    "up_id": d["owner"]["mid"],
    "up_name": d["owner"]["name"],
    "up_avatar": d["owner"]["face"],
    "pic": d.get("pic", ""),
}
print(json.dumps(row, ensure_ascii=False, indent=2))
