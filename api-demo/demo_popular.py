"""
demo_popular.py — 全站热门排行榜

    python demo_popular.py              # 默认第1页前20条
    python demo_popular.py 1 50         # 第1页50条

字段映射（API 响应 → plan 字段）:

    bvid         → item["bvid"]
    title        → item["title"]
    category     → item["tname"]          ← 注意：这是子分区名，crawler 需映射到一级分区
    duration     → item["duration"]       (秒)
    publish_time → item["pubdate"]        (Unix timestamp)
    up_id        → item["owner"]["mid"]
    up_name      → item["owner"]["name"]
    up_avatar    → item["owner"]["face"]
    view_count   → item["stat"]["view"]
    like_count   → item["stat"]["like"]
    coin_count   → item["stat"]["coin"]
    favorite_cnt → item["stat"]["favorite"]
    share_count  → item["stat"]["share"]
    reply_count  → item["stat"]["reply"]
    danmaku_cnt  → item["stat"]["danmaku"]
    aid          → item["aid"]            (评论接口的 oid)
    cid          → item["cid"]            (弹幕接口的 oid)

注意：demo 直接输出 API 的 tname 作为 category，真实 crawler 需维护
tname → 一级分区映射表（如 "MAD·AMV" → "动画"），无法映射时设为 "其他"。
"""
import json
import sys

from utils import api_get

PN = int(sys.argv[1]) if len(sys.argv) > 1 else 1
PS = min(int(sys.argv[2]), 50) if len(sys.argv) > 2 else 20

data = api_get("/x/web-interface/popular", {"pn": PN, "ps": PS}, need_cookie=False)

items = data["data"]["list"]
print(f"# 全站热门 pn={PN} ps={PS} → {len(items)} 条\n")

for i, item in enumerate(items):
    stat = item["stat"]
    owner = item["owner"]
    row = {
        "bvid": item["bvid"],
        "title": item["title"],
        "category": item["tname"],
        "duration": item["duration"],
        "publish_time": item["pubdate"],
        "up_id": owner["mid"],
        "up_name": owner["name"],
        "up_avatar": owner["face"],
        "view_count": stat["view"],
        "like_count": stat["like"],
        "coin_count": stat["coin"],
        "favorite_count": stat["favorite"],
        "share_count": stat["share"],
        "reply_count": stat["reply"],
        "danmaku_count": stat["danmaku"],
        "aid": item["aid"],
        "cid": item["cid"],
        "url": f"https://www.bilibili.com/video/{item['bvid']}",
    }
    print(json.dumps(row, ensure_ascii=False))
