# B站 API 采集示例

每个脚本自包含，可直接运行。不需要额外逆向或推测接口用法。

## 快速开始

```bash
# 1. 安装依赖
pip install -r requirements.txt

# 2. 配置 Cookie（必须）
cp config.py config.local.py
# 编辑 config.local.py，填入浏览器中 B站 的 SESSDATA、bili_jct、DedeUserID
# 然后修改各 demo 的 import 或直接改 config.py

# 3. 运行
python demo_popular.py
```

## 脚本列表

| 脚本 | 用途 | 需要 Cookie | WBI 签名 |
|---|---|---|---|
| `demo_popular.py` | 全站热门排行 | ❌ | ❌ |
| `demo_ranking.py` | 分区排行 | ✅ | ❌ |
| `demo_view.py` | 视频详情（description/tags） | ✅ | ❌ |
| `demo_up_info.py` | UP主信息（粉丝数/投稿数/简介/认证） | ✅ | ✅ 自实现 |
| `demo_reply.py` | 评论列表 | ✅ | ❌ |
| `demo_danmaku.py` | 弹幕内容（段式 seg.so protobuf 自解析） | ❌ | ❌ |
| `demo_weekly_list.py` | 每周必看期数列表 | ❌ | ❌ |
| `demo_weekly_content.py` | 每周必看指定期内容 | ❌ | ❌ |

## 核心注意事项

- **评论 `oid` 使用 `aid`**（不是 `bvid`），从 popular/ranking 响应的 `item["aid"]` 获取
- **弹幕 `oid` 使用 `cid`**，从 `item["cid"]` 获取；使用段式 API `/x/v2/dm/web/seg.so?segment_index=1`（非 `/x/v2/dm/web/view`，后者仅返回控制弹幕/特效配置）
- **popular 和 ranking 响应已内嵌 `stat`**（播放/点赞/投币等全部互动指标），无需单独请求
- **UP主信息需要 WBI 签名**，`utils.py` 已自实现完整签名流程（`/x/web-interface/nav` 获取密钥 → md5 签名）

## 字段覆盖确认

全部 plan 字段均可获取，无缺口：

- 视频基础信息 → `popular` / `ranking` 全覆盖（bvid, title, category, duration, publish_time, up_id, up_name, up_avatar）
- 互动数据 → `popular` / `ranking` 内嵌 `stat` 全覆盖（view, like, coin, favorite, share, reply, danmaku）
- 补充字段 → `view` 补充 description + tags（tags 来自 `d["tags"]` 列表，需提取 `tag_name` 拼接；`d["dynamic"]` 是动态文案，不是标签）
- UP主扩展 → `up_info` (WBI) 补充 follower_count, video_count, sign, official_verify
- 评论 → `reply` 全覆盖
- 弹幕 → `seg.so` 段式 protobuf 全覆盖（非 `/x/v2/dm/web/view`）
- 榜单上下文 → `rank_source`, `rank_name`, `rank_position` 在采集时由脚本设定
