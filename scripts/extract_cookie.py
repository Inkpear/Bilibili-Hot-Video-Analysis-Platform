#!/usr/bin/env python3
"""
extract_cookie.py — 从本地浏览器提取 B站 Cookie 并验证

    python scripts/extract_cookie.py                  # 自动提取
    python scripts/extract_cookie.py --manual         # 手动输入
    python scripts/extract_cookie.py --env-file .env  # 写入 .env 文件

浏览器优先级: Safari → Chrome → Firefox → Edge → 交互模式

依赖:
    pip install browser-cookie3 requests
"""

import argparse
import sys


HEADERS = {
    "User-Agent": (
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) "
        "AppleWebKit/537.36 (KHTML, like Gecko) "
        "Chrome/138.0.0.0 Safari/537.36"
    ),
    "Referer": "https://www.bilibili.com",
}

REQUIRED_KEYS = ("SESSDATA", "bili_jct", "DedeUserID")

# ---------------------------------------------------------------------------
# 浏览器自动提取
# ---------------------------------------------------------------------------

def _try_browser_cookie3():
    try:
        import browser_cookie3
    except ImportError:
        print("⚠️  browser-cookie3 未安装，跳过自动提取")
        print("   安装: pip install browser-cookie3\n")
        return None

    browsers = [
        ("Safari", browser_cookie3.safari),
        ("Chrome", browser_cookie3.chrome),
        ("Firefox", browser_cookie3.firefox),
        ("Edge", browser_cookie3.edge),
        ("Chromium", browser_cookie3.chromium),
        ("Brave", browser_cookie3.brave),
        ("Opera", browser_cookie3.opera),
    ]

    for name, loader in browsers:
        try:
            print(f"🔍 尝试 {name} ... ", end="")
            cj = loader()
            if cj is None:
                print("无数据")
                continue

            # 只为 bilibili.com 加载
            cookies = {}
            for c in cj:
                if "bilibili.com" in c.domain:
                    cookies[c.name] = c.value

            missing = [k for k in REQUIRED_KEYS if k not in cookies]
            if missing:
                print(f"缺少必要字段: {missing}")
                continue

            print(f"✅ 成功 (SESSDATA={cookies['SESSDATA'][:20]}...)")
            return cookies

        except Exception as e:
            print(f"失败 ({e})")
            continue

    print("\n⚠️  自动提取失败，切换到交互模式\n")
    return None


# ---------------------------------------------------------------------------
# 交互式手动输入
# ---------------------------------------------------------------------------

def _manual_input():
    print("=" * 56)
    print("  请从浏览器手动获取 B站 Cookie")
    print("=" * 56)
    print("""
操作步骤:
  1. 浏览器打开 https://www.bilibili.com 并登录
  2. F12 打开 DevTools → Application → Cookies → https://www.bilibili.com
  3. 复制以下 3 个字段的值:
""")

    sessdata = input("  SESSDATA:   ").strip()
    bili_jct = input("  bili_jct:    ").strip()
    dede = input("  DedeUserID:  ").strip()

    if not all([sessdata, bili_jct, dede]):
        print("\n❌ 输入不完整")
        sys.exit(1)

    return {
        "SESSDATA": sessdata,
        "bili_jct": bili_jct,
        "DedeUserID": dede,
    }


# ---------------------------------------------------------------------------
# Cookie 验证
# ---------------------------------------------------------------------------

def _validate(cookies):
    import requests

    cookie_str = "; ".join(
        f"{k}={v}" for k, v in cookies.items() if k in REQUIRED_KEYS
    )

    print(f"\n🔐 验证 Cookie ({len(cookie_str)} 字符) ... ", end="")

    try:
        resp = requests.get(
            "https://api.bilibili.com/x/web-interface/nav",
            headers=HEADERS | {"Cookie": cookie_str},
            timeout=15,
        )
        data = resp.json()
    except Exception as e:
        print(f"\n❌ 请求失败: {e}")
        return None

    if data.get("code") != 0:
        print(f"\n❌ Cookie 无效: code={data['code']} {data.get('message', '')}")
        return None

    user = data["data"]
    uname = user.get("uname", "?")
    uid = user.get("mid", "?")
    is_login = user.get("isLogin", False)

    if not is_login:
        print(f"\n❌ Cookie 已过期 (isLogin=False)")
        return None

    print(f"✅")
    print(f"   用户: {uname} (UID: {uid})")
    return cookie_str


# ---------------------------------------------------------------------------
# 主入口
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(
        description="从浏览器提取 B站 Cookie 并验证"
    )
    parser.add_argument(
        "--manual", action="store_true",
        help="跳过自动提取，直接手动输入"
    )
    parser.add_argument(
        "--env-file", type=str, default=None,
        help="写入 .env 文件 (格式: BILI_COOKIE=...)"
    )
    args = parser.parse_args()

    print("=" * 56)
    print("  B站 Cookie 提取工具")
    print("=" * 56)

    # 1. 获取 Cookie
    if args.manual:
        cookies = _manual_input()
    else:
        cookies = _try_browser_cookie3()
        if cookies is None:
            cookies = _manual_input()

    # 2. 验证
    cookie_str = _validate(cookies)
    if cookie_str is None:
        sys.exit(1)

    # 3. 输出
    print()
    print("=" * 56)
    print("  请复制以下命令到终端执行:")
    print("=" * 56)
    print()
    print(f'  export BILI_COOKIE="{cookie_str}"')

    if args.env_file:
        import os

        env_path = os.path.abspath(args.env_file)
        existing = {}
        if os.path.exists(env_path):
            with open(env_path) as f:
                for line in f:
                    line = line.strip()
                    if line and not line.startswith("#") and "=" in line:
                        k, v = line.split("=", 1)
                        existing[k] = v

        existing["BILI_COOKIE"] = f'"{cookie_str}"'

        with open(env_path, "w") as f:
            for k, v in existing.items():
                f.write(f"{k}={v}\n")

        print(f"\n  ✅ 已写入 {env_path}")

    print()


if __name__ == "__main__":
    main()
