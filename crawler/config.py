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

# ---- 采集分区 rid 映射 ----
CATEGORY_RIDS = {
    "动画": 1, "游戏": 4, "娱乐": 5, "生活": 160,
    "音乐": 3, "舞蹈": 129, "科技": 188, "知识": 36, "影视": 181,
}
CATEGORY_NAMES = {v: k for k, v in CATEGORY_RIDS.items()}

# ---- tname → 一级分区映射表 (popular 来源向上归并) ----
TNAME_TO_CATEGORY = {
    # 动画
    "动画": "动画", "MAD·AMV": "动画", "MMD·3D": "动画",
    "短片·手书·配音": "动画", "手办·模玩": "动画", "特摄": "动画",
    "动漫杂谈": "动画", "综合": "动画",
    # 音乐
    "音乐": "音乐", "原创音乐": "音乐", "翻唱": "音乐",
    "VOCALOID·UTAU": "音乐", "演奏": "音乐", "三次元音乐": "音乐",
    "OP/ED/OST": "音乐", "音乐综合": "音乐", "音乐现场": "音乐",
    "MV": "音乐", "说唱": "音乐", "轻音乐": "音乐",
    "同人音乐": "音乐", "电台": "音乐", "音频": "音乐",
    # 游戏
    "游戏": "游戏", "单机游戏": "游戏", "电子竞技": "游戏",
    "手机游戏": "游戏", "网络游戏": "游戏", "桌游卡牌": "游戏",
    "GMV": "游戏", "音游": "游戏", "Mugen": "游戏",
    # 娱乐
    "娱乐": "娱乐", "搞笑": "娱乐", "鬼畜": "娱乐",
    "动物圈": "娱乐", "绘画": "娱乐", "日常": "生活",
    # 生活
    "生活": "生活", "美食": "生活", "手工": "生活",
    "运动": "生活", "汽车": "生活", "健身": "生活",
    "家居": "生活", "亲子": "生活", "时尚": "生活",
    "美妆": "生活", "穿搭": "生活",
    # 舞蹈
    "舞蹈": "舞蹈", "宅舞": "舞蹈", "街舞": "舞蹈",
    "明星舞蹈": "舞蹈", "中国舞": "舞蹈", "舞蹈综合": "舞蹈",
    "舞蹈教程": "舞蹈",
    # 科技
    "科技": "科技", "数码": "科技", "软件应用": "科技",
    "计算机技术": "科技", "工业·工程": "科技", "极客": "科技",
    "工业": "科技",
    # 知识
    "知识": "知识", "科学科普": "知识", "社科·法律·心理": "知识",
    "财经": "知识", "校园学习": "知识", "职业职场": "知识",
    "设计·创意": "知识", "野生技术协会": "知识", "人文历史": "知识",
    # 影视
    "影视": "影视", "纪录片": "影视", "电影": "影视",
    "电视剧": "影视", "影视剪辑": "影视", "影视杂谈": "影视",
    "预告 资讯": "影视",
}

# ---- 采集限制 ----
POPULAR_PS = 50        # 每页条数
POPULAR_PAGES = 2      # 全站热门页数 (2页×50=100条)
REPLY_SORT = 1         # 评论排序: 1=热度, 0=时间
REPLY_PS = 20          # 评论每页条数
MAX_COMMENTS = 200     # 每视频评论上限
MAX_DANMAKU = 3000     # 每视频弹幕上限

# ---- 队列消费上限 (每轮快照) ----
MAX_COMMENTS_PER_RUN = 35   # 每轮最多处理 N 个视频的评论
MAX_DANMAKU_PER_RUN = 35    # 每轮最多处理 N 个视频的弹幕

# ---- 超时控制 ----
RUN_TIMEOUT_SECONDS = 10800          # 单轮全局硬超时 (3h)
PER_COMMENT_TIMEOUT = 300            # 单视频评论超时 (秒)
PER_DANMAKU_TIMEOUT = 600            # 单视频弹幕超时 (秒)
DANMAKU_429_COOLDOWN = 120           # 连续429后全局冷却 (秒)
MAX_CONSECUTIVE_DANMAKU_429 = 10     # 连续多少段429后触发冷却

# ---- 输出路径 ----
OUTPUT_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "output")

# ---- 数据库 ----
DB_PATH = os.path.join(os.path.dirname(os.path.abspath(__file__)), "crawler_state.db")


def map_tname(tname: str) -> str:
    """将子分区名映射到一级分区，未匹配返回 '其他'。"""
    if not tname:
        return "其他"
    return TNAME_TO_CATEGORY.get(tname, "其他")
