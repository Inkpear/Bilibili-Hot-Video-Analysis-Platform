"""
demo_up_info.py — UP主信息（WBI 签名）

    python demo_up_info.py 63231         # 泛式的 mid

使用 WBI 签名的 /x/wbi/acc/info 接口获取:
    - follower_count（粉丝数）
    - video_count（投稿数）
    - sign（个人简介）
    - official_verify（认证信息）

WBI 签名算法已自实现于 utils.py（bilibili-API-collect 已停维）。
需要 Cookie（SESSDATA）。
"""
import json
import sys

from utils import api_get

mid = int(sys.argv[1]) if len(sys.argv) > 1 else 63231

data = api_get(
    "/x/wbi/acc/info",
    {"mid": mid},
    need_cookie=True,
    wbi_sign=True,
)
d = data["data"]

row = {
    "up_id": d["mid"],
    "up_name": d["name"],
    "up_avatar": d.get("face", ""),
    "follower_count": d.get("follower", 0),
    "video_count": d.get("video", 0),
    "sign": d.get("sign", ""),
    "official_verify": d.get("official", {}).get("desc", ""),
    "sex": d.get("sex", ""),
    "level": d.get("level", 0),
}
print(json.dumps(row, ensure_ascii=False, indent=2))
