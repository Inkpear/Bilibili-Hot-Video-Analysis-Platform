"""
demo_weekly_content.py — 每周必看指定期视频列表

    python demo_weekly_content.py 375

实测 /x/web-interface/popular/series/one 比 /popular/series 更稳定。
number 参数从 demo_weekly_list.py 获取。

返回字段（核心字段；stat 中部分字段可能为空，需 crawler 补充 /view 获取完整互动数据）：
    bvid, title, category(tname), up_id, up_name,
    view_count, like_count, coin_count, favorite_count, share_count, reply_count, danmaku_count,
    aid, cid
"""
import json
import sys
import time

from utils import api_get

number = int(sys.argv[1]) if len(sys.argv) > 1 else None
if number is None:
    # 获取最新一期
    data = api_get("/x/web-interface/popular/series/list", need_cookie=False)
    items = data["data"]["list"]
    number = max(s["number"] for s in items)

print(f"# 每周必看 number={number}\n")

data = api_get(
    "/x/web-interface/popular/series/one",
    {"number": number},
    need_cookie=False,
)
items = data["data"]["list"]
print(f"# {len(items)} 个视频\n")

for item in items:
    stat = item["stat"]
    owner = item["owner"]
    row = {
        "bvid": item["bvid"],
        "title": item["title"],
        "category": item.get("tname", ""),
        "up_id": owner["mid"],
        "up_name": owner["name"],
        "view_count": stat["view"],
        "like_count": stat["like"],
        "coin_count": stat.get("coin", 0),
        "favorite_count": stat.get("favorite", 0),
        "share_count": stat.get("share", 0),
        "reply_count": stat.get("reply", 0),
        "danmaku_count": stat.get("danmaku", 0),
        "aid": item["aid"],
        "cid": item["cid"],
    }
    print(json.dumps(row, ensure_ascii=False))
