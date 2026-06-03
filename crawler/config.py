"""
全局配置。Cookie 从环境变量 BILI_COOKIE 读取，不硬编码。
"""

import os

# ---- Cookie ----
COOKIE = os.environ.get("BILI_COOKIE", "").strip('"').strip("'")
if not COOKIE:
    raise RuntimeError(
        "环境变量 BILI_COOKIE 未设置，请先运行:\n"
        "  export BILI_COOKIE=\"SESSDATA=...; bili_jct=...; DedeUserID=...\""
    )

# ---- 请求间隔 ----
DELAY = 2.0  # 秒

# ---- 请求超时 ----
TIMEOUT = 30  # 秒

# ---- 重试 ----
MAX_RETRIES = 2
RETRY_BACKOFF_BASE = 2.0   # 指数退避基数
RATE_LIMIT_BACKOFF = 5.0   # 429 退避起始秒数

# ---- 通用请求头 ----
HEADERS = {
    "User-Agent": (
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) "
        "AppleWebKit/537.36 (KHTML, like Gecko) "
        "Chrome/138.0.0.0 Safari/537.36"
    ),
    "Referer": "https://www.bilibili.com",
}

API_BASE = "https://api.bilibili.com"

# ---- 分区 rid 映射 (ranking/v2 接口) ----
CATEGORY_RIDS = {
    "动画": 1005,
    "游戏": 1008,
    "娱乐": 1002,
    "音乐": 1003,
    "舞蹈": 1004,
    "科技": 1012,
    "知识": 1010,
    "影视": 1001,
    "鬼畜": 1007,
    "美食": 1020,
    "汽车": 1013,
    "时尚美妆": 1014,
    "体育运动": 1018,
    "动物": 1024,
}
CATEGORY_NAMES = {v: k for k, v in CATEGORY_RIDS.items()}

# ---- 采集限制 ----
REPLY_SORT = 1         # 评论排序: 1=热度, 0=时间
REPLY_PS = 20          # 评论每页条数
MAX_COMMENTS = 200     # 每视频评论上限
MAX_DANMAKU = 3000     # 每视频弹幕上限

# ---- 弹幕全局冷却 ----
DANMAKU_429_COOLDOWN = 120           # 连续429后全局冷却 (秒)
MAX_CONSECUTIVE_DANMAKU_429 = 10     # 连续多少段429后触发冷却

# ---- 永久放弃阈值 ----
MAX_FAIL_COUNT = 3                   # 连续失败 N 次后永久放弃

# ---- 输出路径 ----
OUTPUT_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "output")

# ---- 数据库 ----
DB_PATH = os.path.join(os.path.dirname(os.path.abspath(__file__)), "crawler_state.db")
