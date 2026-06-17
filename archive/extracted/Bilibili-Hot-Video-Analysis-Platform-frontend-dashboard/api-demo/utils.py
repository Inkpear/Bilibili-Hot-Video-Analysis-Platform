"""
公用工具：WBI 签名、protobuf 弹幕解析、请求封装。

WBI 签名算法（bilibili-API-collect 已停维，此处为独立实现）：

    1. GET /x/web-interface/nav → 获取 img_key, sub_key
    2. 从 img_url/sub_url 中提取文件名(不含扩展名)，拼接为 mixin_key
    3. 请求参数按 key 排序，追加 wts=当前秒级时间戳
    4. w_rid = md5(参数字符串 + mixin_key)

弹幕段式 protobuf 格式（seg.so 端点，实测验证）：

    DmSegMobileReply {
        repeated DanmakuElem elems = 1;
    }
    DanmakuElem {
        int64  id       = 1;
        int32  progress = 2;   // 毫秒
        int32  mode     = 3;
        int32  fontsize = 4;
        uint32 color    = 5;
        string mid_hash = 7;
        string content  = 9;
        int64  ctime    = 10;
        int32  weight   = 11;
        string action   = 12;
    }

注意：弹幕接口使用段式加载（每 6 分钟一段），URL 为:
    GET /x/v2/dm/web/seg.so?type=1&oid={cid}&segment_index=1
而非旧版 /x/v2/dm/web/view（该端点仅返回控制弹幕/特效配置）。
"""
import hashlib
import json
import os
import re
import struct
import time
import urllib.parse
from functools import lru_cache

import requests

from config import HEADERS, COOKIE, DELAY  # demo 用，正式 crawler 应改用 os.environ["BILI_COOKIE"]

API_BASE = "https://api.bilibili.com"


# ---------------------------------------------------------------------------
# 请求封装
# ---------------------------------------------------------------------------

def _headers(need_cookie=True):
    h = dict(HEADERS)
    if need_cookie:
        h["Cookie"] = COOKIE
    return h


def api_get(path, params=None, need_cookie=True, wbi_sign=False):
    """发送 GET 请求，返回解析后的 JSON dict。"""
    if params is None:
        params = {}
    if wbi_sign:
        params = _wbi_sign_params(params)

    url = f"{API_BASE}{path}"
    resp = requests.get(
        url, params=params, headers=_headers(need_cookie), timeout=30
    )
    time.sleep(DELAY)
    resp.raise_for_status()
    data = resp.json()
    if data.get("code") != 0:
        raise RuntimeError(
            f"API {path} returned code={data['code']}: {data.get('message', '')}"
        )
    return data


# ---------------------------------------------------------------------------
# WBI 签名
# ---------------------------------------------------------------------------

_WBI_KEY_CACHE = {"img_key": "", "sub_key": "", "mixin_key": ""}


def _fetch_wbi_keys():
    """从 nav 接口获取 WBI 密钥对，缓存 4 小时。"""
    now = time.time()
    # 简单缓存
    if _WBI_KEY_CACHE.get("ts", 0) > now - 14400 and _WBI_KEY_CACHE["mixin_key"]:
        return

    data = api_get("/x/web-interface/nav", need_cookie=True)
    wbi = data["data"]["wbi_img"]
    img_url = wbi["img_url"]
    sub_url = wbi["sub_url"]

    def extract_key(url):
        # url 形如 https://i0.hdslb.com/bfs/wbi/7cd084941338484aae1ad9425b84077c.png
        name = url.rsplit("/", 1)[-1]
        return name.rsplit(".", 1)[0]

    img_key = extract_key(img_url)
    sub_key = extract_key(sub_url)
    mixin = img_key + sub_key

    _WBI_KEY_CACHE["img_key"] = img_key
    _WBI_KEY_CACHE["sub_key"] = sub_key
    _WBI_KEY_CACHE["mixin_key"] = mixin
    _WBI_KEY_CACHE["ts"] = now


def _wbi_sign_params(params):
    """对参数字典追加 wts 和 w_rid，返回新字典（不修改原参数）。"""
    _fetch_wbi_keys()
    mixin = _WBI_KEY_CACHE["mixin_key"]

    p = dict(params)
    p["wts"] = int(time.time())
    # 按 key 排序
    ordered = sorted(p.items(), key=lambda x: x[0])
    # 拼接: key=value&key=value (值需 URL 编码)
    raw = "&".join(
        f"{k}={urllib.parse.quote(str(v), safe='')}" for k, v in ordered
    )
    p["w_rid"] = hashlib.md5((raw + mixin).encode()).hexdigest()
    return p


# ---------------------------------------------------------------------------
# Protobuf 弹幕解析
#
# 弹幕数据来自段式 API: GET /x/v2/dm/web/seg.so?type=1&oid={cid}&segment_index=1
# 返回 DmSegMobileReply protobuf，其中 field 1 为 repeated DanmakuElem。
# 视频长度 / 360 秒 ≈ 总段数，逐段拉取可获全部弹幕。
#
# DanmakuElem 字段映射（2026-05 实测）:
#   field 1  = id        (int64)
#   field 2  = progress  (int32, 毫秒)
#   field 3  = mode      (int32, 1=滚动 4=底端 5=顶端)
#   field 4  = fontsize  (int32)
#   field 5  = color     (uint32)
#   field 6  = mid_hash  (string)
#   field 7  = content   (string)  ← 注意！
#   field 8  = ctime     (int64, Unix timestamp)
# ---------------------------------------------------------------------------

def parse_danmaku_seg(raw_bytes):
    """
    解析 seg.so 返回的 DmSegMobileReply protobuf。
    返回 list[dict]，每条 dict 包含:
        id, progress(ms), mode, fontsize, color, mid_hash, content, ctime
    """
    elems = []
    pos = 0
    while pos < len(raw_bytes):
        tag, pos = _read_varint(raw_bytes, pos)
        field_num = tag >> 3
        wire_type = tag & 0x7

        if wire_type == 2:
            length, pos = _read_varint(raw_bytes, pos)
            sub = raw_bytes[pos : pos + length]
            pos += length

            if field_num == 1:  # DmSegMobileReply.elems (repeated DanmakuElem)
                elem = _parse_danmaku_elem(sub)
                if elem:
                    elems.append(elem)
        else:
            pos = _skip_field(raw_bytes, pos, wire_type)

    return elems


def parse_danmaku_protobuf(raw_bytes):
    """兼容旧版 DmWebViewReply（仅返回控制弹幕），保留此函数但建议用 parse_danmaku_seg。"""
    return parse_danmaku_seg(raw_bytes)


def _parse_danmaku_elem(data):
    """解析单个 DanmakuElem message（2026-05 实测字段映射）。"""
    result = {}
    pos = 0
    while pos < len(data):
        tag, pos = _read_varint(data, pos)
        field_num = tag >> 3
        wire_type = tag & 0x7

        if wire_type == 0:  # varint
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
        elif wire_type == 2:  # length-delimited
            length, pos = _read_varint(data, pos)
            val = data[pos : pos + length]
            pos += length
            if field_num == 6:
                result["mid_hash"] = val.decode("utf-8", errors="replace")
            elif field_num == 7:
                result["content"] = val.decode("utf-8", errors="replace")
        else:
            pos = _skip_field(data, pos, wire_type)

    return result if result else None


def _read_varint(data, pos):
    """读取一个 varint，返回 (value, new_pos)。"""
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
    """跳过未知字段。"""
    if wire_type == 0:  # varint
        _, pos = _read_varint(data, pos)
    elif wire_type == 1:  # 64-bit
        pos += 8
    elif wire_type == 2:  # length-delimited
        length, pos = _read_varint(data, pos)
        pos += length
    elif wire_type == 5:  # 32-bit
        pos += 4
    return pos


# ---------------------------------------------------------------------------
# 便捷访问器
# ---------------------------------------------------------------------------

def get_bvid_from_popular():
    """获取全站热门第一个视频的 bvid，用于快速测试。"""
    data = api_get("/x/web-interface/popular", {"pn": 1, "ps": 1})
    return data["data"]["list"][0]["bvid"]
