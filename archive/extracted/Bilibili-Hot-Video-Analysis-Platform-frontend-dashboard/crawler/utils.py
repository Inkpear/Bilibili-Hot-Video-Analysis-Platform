"""
网络层：HTTP 请求封装 (重试/退避)、WBI 签名、protobuf 弹幕解析、日志。
"""

import sys
import hashlib
import logging
import os
import time
import urllib.parse

import requests

from config import (
    HEADERS, COOKIE, DELAY, TIMEOUT,
    MAX_RETRIES, RETRY_BACKOFF_BASE, RATE_LIMIT_BACKOFF,
    API_BASE,
)

logger = logging.getLogger("crawler")

# ---------------------------------------------------------------------------
# 日志
# ---------------------------------------------------------------------------

def setup_logging(level=logging.INFO, log_file=None):
    root = logging.getLogger("crawler")
    root.setLevel(level)
    root.handlers.clear()
    root.propagate = False

    fmt = logging.Formatter(
        "%(asctime)s [%(levelname)s] %(message)s", "%Y-%m-%d %H:%M:%S"
    )

    handler = logging.StreamHandler(sys.stdout)
    handler.setFormatter(fmt)
    root.addHandler(handler)

    if log_file:
        log_dir = os.path.dirname(log_file)
        if log_dir:
            os.makedirs(log_dir, exist_ok=True)
        fh = logging.FileHandler(log_file, encoding="utf-8")
        fh.setFormatter(fmt)
        root.addHandler(fh)
        root.info(f"日志文件: {log_file}")


# ---------------------------------------------------------------------------
# HTTP 请求 (带重试)
# ---------------------------------------------------------------------------

def _make_headers(need_cookie=True):
    h = dict(HEADERS)
    if need_cookie:
        h["Cookie"] = COOKIE
    return h


# 状态码描述，用于 412/429 判断
def _is_rate_limited(resp):
    return resp.status_code in (412, 429)


def api_get_raw(path, params=None, need_cookie=True):
    """发送 GET，返回 requests.Response（不解析 JSON）。"""
    if params is None:
        params = {}
    url = f"{API_BASE}{path}"
    return requests.get(
        url, params=params, headers=_make_headers(need_cookie), timeout=TIMEOUT,
    )


def api_get(path, params=None, need_cookie=True, wbi_sign=False):
    """
    发送 GET 请求，自动重试 + 指数退避。
    - 网络错误: 指数退避 (1s/2s/4s)
    - 412/429: 更长的退避 (5s/10s/20s)
    - 业务错误 (code!=0): 抛 RuntimeError
    返回解析后的 JSON dict。
    """
    if params is None:
        params = {}
    if wbi_sign:
        params = _wbi_sign_params(params)

    url = f"{API_BASE}{path}"
    headers = _make_headers(need_cookie)

    last_error = None
    for attempt in range(MAX_RETRIES + 1):
        did_request = False
        try:
            did_request = True
            resp = requests.get(url, params=params, headers=headers, timeout=TIMEOUT)
            if _is_rate_limited(resp):
                wait = RATE_LIMIT_BACKOFF * (2 ** attempt)
                logger.warning(
                    f"限流 {resp.status_code} {path} -> 等待 {wait:.0f}s (第{attempt+1}次)"
                )
                time.sleep(wait)
                continue

            resp.raise_for_status()
            data = resp.json()
            if data.get("code") != 0:
                raise RuntimeError(
                    f"API {path} code={data['code']}: {data.get('message', 'N/A')}"
                )

            return data

        except requests.exceptions.HTTPError as e:
            last_error = e
            if _is_rate_limited(e.response):
                wait = RATE_LIMIT_BACKOFF * (2 ** attempt)
                logger.warning(f"限流 {e.response.status_code} 等待 {wait:.0f}s")
                time.sleep(wait)
                continue
            # 4xx 非限流错误不重试 (404 Not Found / 403 Forbidden 等)
            if 400 <= e.response.status_code < 500:
                raise
            if attempt >= MAX_RETRIES:
                raise
            wait = RETRY_BACKOFF_BASE ** attempt
            logger.warning(f"HTTP {e.response.status_code} 重试 {attempt+1}/{MAX_RETRIES}")
            time.sleep(wait)

        except (requests.exceptions.Timeout, requests.exceptions.ConnectionError) as e:
            last_error = e
            if attempt >= MAX_RETRIES:
                raise
            wait = RETRY_BACKOFF_BASE ** attempt
            logger.warning(f"网络错误 {e} 重试 {attempt+1}/{MAX_RETRIES}")
            time.sleep(wait)

        except requests.exceptions.RequestException as e:
            raise RuntimeError(f"请求失败 {path}: {e}")
        finally:
            if did_request:
                time.sleep(DELAY)

    raise RuntimeError(f"请求失败 {path} (重试{MAX_RETRIES}次): {last_error}")


def api_get_safe(path, params=None, need_cookie=True, wbi_sign=False, default=None):
    """不会抛异常的 api_get，失败返回 default。"""
    try:
        return api_get(path, params, need_cookie, wbi_sign)
    except Exception as e:
        logger.error(f"API 失败 {path}: {e}")
        return default


# ---------------------------------------------------------------------------
# WBI 签名 (从 api-demo/utils.py 移植)
# ---------------------------------------------------------------------------

_WBI_KEY_CACHE = {"img_key": "", "sub_key": "", "mixin_key": "", "ts": 0}

MIXIN_KEY_ENC_TAB = [
    46, 47, 18, 2, 53, 8, 23, 32,
    15, 50, 10, 31, 58, 3, 45, 35,
    27, 43, 5, 49, 33, 9, 42, 19,
    29, 28, 14, 39, 12, 38, 41, 13,
    37, 48, 7, 16, 24, 55, 40, 61,
    26, 17, 0, 1, 60, 51, 30, 4,
    22, 25, 54, 21, 56, 59, 6, 63,
    57, 62, 11, 36, 20, 34, 44, 52,
]


def _get_mixin_key(orig: str) -> str:
    return "".join(orig[i] for i in MIXIN_KEY_ENC_TAB)[:32]


def _fetch_wbi_keys():
    now = time.time()
    if _WBI_KEY_CACHE.get("ts", 0) > now - 14400 and _WBI_KEY_CACHE["mixin_key"]:
        return

    data = api_get("/x/web-interface/nav", need_cookie=True)
    wbi = data["data"]["wbi_img"]
    img_url = wbi["img_url"]
    sub_url = wbi["sub_url"]

    def extract_key(url):
        name = url.rsplit("/", 1)[-1]
        return name.rsplit(".", 1)[0]

    img_key = extract_key(img_url)
    sub_key = extract_key(sub_url)
    mixin = _get_mixin_key(img_key + sub_key)

    _WBI_KEY_CACHE["img_key"] = img_key
    _WBI_KEY_CACHE["sub_key"] = sub_key
    _WBI_KEY_CACHE["mixin_key"] = mixin
    _WBI_KEY_CACHE["ts"] = now


def _wbi_sign_params(params):
    _fetch_wbi_keys()
    mixin = _WBI_KEY_CACHE["mixin_key"]

    p = dict(params)
    p["wts"] = int(time.time())
    for key, value in list(p.items()):
        if isinstance(value, str):
            p[key] = "".join(ch for ch in value if ch not in "!'()*")
    ordered = sorted(p.items(), key=lambda x: x[0])
    raw = "&".join(
        f"{k}={urllib.parse.quote(str(v), safe='')}" for k, v in ordered
    )
    p["w_rid"] = hashlib.md5((raw + mixin).encode()).hexdigest()
    return p


# ---------------------------------------------------------------------------
# Protobuf 弹幕解析 (从 api-demo/utils.py 移植)
# ---------------------------------------------------------------------------

def _read_varint(data, pos):
    value = 0
    shift = 0
    while pos < len(data):
        byte = data[pos]
        pos += 1
        value |= (byte & 0x7F) << shift
        if not (byte & 0x80):
            break
        shift += 7
    return value, pos


def _skip_field(data, pos, wire_type):
    if wire_type == 0:
        _, pos = _read_varint(data, pos)
    elif wire_type == 1:
        pos += 8
    elif wire_type == 2:
        length, pos = _read_varint(data, pos)
        pos += length
    elif wire_type == 5:
        pos += 4
    return pos


def _parse_danmaku_elem(data):
    result = {}
    pos = 0
    while pos < len(data):
        tag, pos = _read_varint(data, pos)
        field_num = tag >> 3
        wire_type = tag & 0x7

        if wire_type == 0:
            val, pos = _read_varint(data, pos)
            if field_num == 1:
                result["id"] = val
            elif field_num == 2:
                result["progress"] = val
            elif field_num == 3:
                result["mode"] = val
            elif field_num == 4:
                result["fontsize"] = val
            elif field_num == 5:
                result["color"] = val
            elif field_num == 8:
                result["ctime"] = val
        elif wire_type == 2:
            length, pos = _read_varint(data, pos)
            val = data[pos: pos + length]
            pos += length
            if field_num == 6:
                result["mid_hash"] = val.decode("utf-8", errors="replace")
            elif field_num == 7:
                result["content"] = val.decode("utf-8", errors="replace")
        else:
            pos = _skip_field(data, pos, wire_type)

    return result if result else None


def parse_danmaku_seg(raw_bytes):
    """
    解析 seg.so 返回的 DmSegMobileReply protobuf。
    返回 list[dict]，每条含: id, progress(ms), mode, fontsize,
    color, mid_hash, content, ctime
    """
    elems = []
    pos = 0
    while pos < len(raw_bytes):
        tag, pos = _read_varint(raw_bytes, pos)
        field_num = tag >> 3
        wire_type = tag & 0x7

        if wire_type == 2:
            length, pos = _read_varint(raw_bytes, pos)
            sub = raw_bytes[pos: pos + length]
            pos += length
            if field_num == 1:
                elem = _parse_danmaku_elem(sub)
                if elem:
                    elems.append(elem)
        else:
            pos = _skip_field(raw_bytes, pos, wire_type)

    return elems


# ---------------------------------------------------------------------------
# 时间格式化
# ---------------------------------------------------------------------------

def timestamp_now_ms():
    return int(time.time() * 1000)


def timestamp_now_s():
    return int(time.time())


def format_snapshot_id():
    """返回当前时间的 snapshot_id: yyyyMMdd_HHmm"""
    return time.strftime("%Y%m%d_%H%M", time.localtime())
