"""
demo_danmaku.py — 弹幕内容（段式 protobuf）

    python demo_danmaku.py BV1iQVG6AEtS 38717490000
    python demo_danmaku.py BV1iQVG6AEtS 38717490000 1 3   # 段1~3

参数:
    bvid          — 视频 BV 号
    cid           — 视频 cid（从 popular/ranking 响应的 item["cid"] 获取）
    segment_start — 起始段号（默认 1）
    segment_end   — 结束段号（默认 1）

说明:
    B站弹幕采用 6 分钟一段的段式加载。
    段号 = 视频分钟数 / 6（上取整）。
    例如 20 分钟视频 → 4 段。
    本脚本逐段拉取并汇总输出。

    接口: GET /x/v2/dm/web/seg.so?type=1&oid={cid}&segment_index={n}
    格式: DmSegMobileReply protobuf，自解析（不依赖 .proto 编译）

字段映射:
    danmaku_id  → id
    content     → content (UTF-8)
    video_time  → progress / 1000 (毫秒 → 秒)
    send_time   → ctime (Unix timestamp)

不需要 Cookie。
"""
import json
import sys
import time

import requests

from config import HEADERS, DELAY
from utils import parse_danmaku_seg, get_bvid_from_popular, api_get

if len(sys.argv) >= 3:
    bvid = sys.argv[1]
    cid = int(sys.argv[2])
    seg_start = int(sys.argv[3]) if len(sys.argv) > 3 else 1
    seg_end = int(sys.argv[4]) if len(sys.argv) > 4 else seg_start
else:
    data = api_get("/x/web-interface/popular", {"pn": 1, "ps": 1})
    item = data["data"]["list"][0]
    bvid = item["bvid"]
    cid = item["cid"]
    seg_start = 1
    seg_end = 1

print(f"# 弹幕 bvid={bvid} cid={cid} segments={seg_start}..{seg_end}\n")

all_elems = []
for seg in range(seg_start, seg_end + 1):
    resp = requests.get(
        "https://api.bilibili.com/x/v2/dm/web/seg.so",
        params={"type": 1, "oid": cid, "segment_index": seg},
        headers=HEADERS,
        timeout=30,
    )
    time.sleep(DELAY)

    if resp.status_code != 200:
        print(f"# seg {seg}: HTTP {resp.status_code}")
        continue

    elems = parse_danmaku_seg(resp.content)
    print(f"# seg {seg}: {len(elems)} 条 ({len(resp.content)} bytes)")
    all_elems.extend(elems)

print(f"\n# 总计 {len(all_elems)} 条弹幕\n")

for e in all_elems:
    row = {
        "danmaku_id": e.get("id"),
        "bvid": bvid,
        "cid": cid,
        "content": e.get("content", ""),
        "video_time": round(e.get("progress", 0) / 1000.0, 3),
        "mode": e.get("mode"),
        "fontsize": e.get("fontsize"),
        "color": e.get("color"),
        "mid_hash": e.get("mid_hash"),
        "send_time": e.get("ctime"),
    }
    print(json.dumps(row, ensure_ascii=False))
