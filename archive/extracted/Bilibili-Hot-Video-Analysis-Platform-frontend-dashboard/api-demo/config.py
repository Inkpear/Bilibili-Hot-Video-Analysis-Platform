# B站 API 采集示例
#
# 所有脚本自包含，可直接运行。
# 使用方法：
#   1. 复制 config.example.py → config.py，填入你的 Cookie
#   2. pip install -r requirements.txt
#   3. python demo_popular.py
#
# 每个 demo 脚本运行后输出 JSON 到控制台，也可重定向到文件。
#
# ⚠️ 安全提示：
#   - 不要将包含真实 Cookie 的 config.py 提交到版本控制
#   - 正式 crawler 应使用环境变量 BILI_COOKIE，不要硬编码
#   - 参见 plan.md Stage 1 中的 Cookie 配置说明

# ---- 通用请求头 ----
HEADERS = {
    "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
    "Referer": "https://www.bilibili.com",
}

# ---- Cookie（必填） ----
# 从浏览器登录 B站 后，F12 → Application → Cookies → bilibili.com
# 复制 SESSDATA, bili_jct, DedeUserID 三项
COOKIE = "SESSDATA=你的SESSDATA; bili_jct=你的bili_jct; DedeUserID=你的DedeUserID"

# ---- 请求间隔（秒），建议 ≥1 ----
DELAY = 1.0

# ---- 采集分区列表 ----
CATEGORY_RIDS = {
    "动画": 1,
    "番剧": 13,
    "游戏": 4,
    "娱乐": 5,
    "生活": 160,
    "音乐": 3,
    "舞蹈": 129,
    "科技": 188,
    "知识": 36,
    "影视": 181,
}
