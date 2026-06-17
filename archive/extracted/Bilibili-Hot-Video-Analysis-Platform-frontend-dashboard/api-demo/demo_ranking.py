"""
demo_ranking.py — 分区排行榜

    python demo_ranking.py               # 默认 动画(rid=1)
    python demo_ranking.py 4             # 游戏(rid=4)
    python demo_ranking.py               # rid 见 config.CATEGORY_RIDS

返回字段同 demo_popular.py，额外包含 pts（综合得分）。
需要 Cookie。

注意：ranking 来源的 category 直接由请求的 rid 确定（如 rid=1 → "动画"），
与 popular 来源需要从 tname 映射不同。
"""
import json
import sys

from config import CATEGORY_RIDS
from utils import api_get

rid = int(sys.argv[1]) if len(sys.argv) > 1 else 1
name = next((k for k, v in CATEGORY_RIDS.items() if v == rid), str(rid))

data = api_get("/x/web-interface/ranking/v2", {"rid": rid, "type": "all"}, need_cookie=True)

items = data["data"]["list"]
print(f"# 分区排行 rid={rid}({name}) → {len(items)} 条\n")

for i, item in enumerate(items):
    stat = item["stat"]
    owner = item["owner"]
    row = {
        "rank": i + 1,
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
        "pts": item.get("pts"),
        "url": f"https://www.bilibili.com/video/{item['bvid']}",
    }
    print(json.dumps(row, ensure_ascii=False))
