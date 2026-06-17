"""
demo_weekly_list.py — 每周必看期数列表

    python demo_weekly_list.py

返回所有期数的列表（按时间降序），每条包含 number（期号）、name、subject。
最新一期通过 list[0] 获取。

不需要 Cookie。
"""
import json

from utils import api_get

data = api_get("/x/web-interface/popular/series/list", need_cookie=False)

items = data["data"]["list"]
# 按 number 降序（最新在前）
items.sort(key=lambda x: x["number"], reverse=True)

print(f"# 每周必看共 {len(items)} 期\n")
for s in items[:5]:
    row = {
        "number": s["number"],
        "name": s["name"],
        "subject": s.get("subject", ""),
        "status": s.get("status"),
    }
    print(json.dumps(row, ensure_ascii=False))
