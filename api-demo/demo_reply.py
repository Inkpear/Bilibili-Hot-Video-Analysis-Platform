"""
demo_reply.py — 评论列表

    python demo_reply.py BV1iQVG6AEtS 116662200173257

参数:
    bvid  — 视频 BV 号
    aid   — 视频 aid（oid 参数需要 aid 而非 bvid！）

评论接口 /x/v2/reply 的 oid 参数使用 aid（不是 bvid）。
sort=1 按热度排序，sort=0 按时间排序。
每页最多 20 条（ps 最大 20），通过 pn 翻页。

需要 Cookie。
"""
import json
import sys

from utils import api_get, get_bvid_from_popular

if len(sys.argv) >= 3:
    bvid = sys.argv[1]
    aid = int(sys.argv[2])
else:
    # 自动从热门榜获取
    data = api_get("/x/web-interface/popular", {"pn": 1, "ps": 1})
    item = data["data"]["list"][0]
    bvid = item["bvid"]
    aid = item["aid"]

print(f"# 评论 bvid={bvid} aid={aid}\n")

data = api_get(
    "/x/v2/reply",
    {"type": 1, "oid": aid, "pn": 1, "ps": 20, "sort": 1},
    need_cookie=True,
)

replies = data["data"].get("replies", [])
print(f"# 本页 {len(replies)} 条，共 {data['data']['page']['count']} 条\n")

for r in replies:
    row = {
        "comment_id": r["rpid"],
        "bvid": bvid,
        "aid": aid,
        "user_id": r["mid"],
        "user_name": r["member"]["uname"],
        "content": r["content"]["message"],
        "like_count": r["like"],
        "reply_count": r["rcount"],
        "comment_time": r["ctime"],
    }
    print(json.dumps(row, ensure_ascii=False))
