# B站热门视频分析平台 — 后端 API 文档

> 本地地址: `http://localhost:9090`
>
> 公网地址: `http://111.228.34.34:9091`（通过 FRP 穿透）
>
> 技术栈: Rust Axum + sqlx (MySQL)
>
> 更新时间: 2026-06-01

---

## 目录

1. [通用约定](#1-通用约定)
2. [鉴权](#2-鉴权)
3. [端点列表](#3-端点列表)
4. [/api/overview](#4-apioverview)
5. [/api/hot-videos](#5-apihot-videos)
6. [/api/partition-compare](#6-apipartition-compare)
7. [/api/lifecycle](#7-apilifecycle)
8. [/api/lifecycle/top](#8-apilifecycletop)
9. [/api/partition-detail](#9-apipartition-detail)
9. [/api/word-cloud](#9-apiword-cloud)
10. [/api/hot-comments](#10-apihot-comments)
11. [/api/interaction](#11-apiinteraction)
12. [错误码与状态码](#12-错误码与状态码)
13. [TypeScript 类型定义](#13-typescript-类型定义)

---

## 1. 通用约定

### 响应格式

所有接口返回统一 JSON 结构：

```json
{
  "code": 0,
  "message": "ok",
  "data": { ... }
}
```

| 字段 | 类型 | 说明 |
|---|---|---|
| `code` | int | 业务状态码，`0` 表示成功，非 0 为错误 |
| `message` | string | 状态描述 |
| `data` | any | 响应数据，资源不存在时为 `null` |

### HTTP 状态码

| HTTP Status | 含义 | JSON `code` |
|---|---|---|
| `200 OK` | 成功 / 资源不存在 | `0` |
| `400 Bad Request` | 参数错误 | `400` |
| `500 Internal Server Error` | 服务器内部错误 | `500` |

### 参数约定

- 所有字段使用 `snake_case`
- 时间格式: `yyyy-MM-dd HH:mm:ss`
- 日期格式: `yyyyMMdd`（如 `20260601`）
- 时间点格式: `HH:mm`（如 `18:42`）
- 分区名: 中文，如 `游戏`、`动画`、`音乐` 等
- BV 号: `BV` 开头，如 `BV16YGC6XEco`

### 空数据处理

| 场景 | 返回 |
|---|---|
| 列表为空 | `data.videos = []`、`data.partitions = []`、`data.tags = []` |
| 单资源不存在 | `data = null` |
| overview 无数据 | 计数字段为 `0`，`latest_snapshot_date = null` |

---

## 2. 鉴权

API 支持基于 Token 的鉴权。当配置了 `API_TOKEN` 环境变量时，所有 `/api/*` 请求必须携带有效的 Token。

### 配置方式

**环境变量**（推荐）:

```bash
export API_TOKEN=your_secret_token_here
```

**`.env` 文件**:

```env
API_TOKEN=your_secret_token_here
```

**配置优先级**:

1. 环境变量 `API_TOKEN`
2. `.env` 文件中的 `API_TOKEN`
3. 默认值（空字符串，鉴权禁用）

### 请求方式

在请求头中添加 `Authorization: Bearer <token>`:

```bash
curl -H "Authorization: Bearer your_secret_token_here" http://localhost:3000/api/overview
```

### 鉴权行为

| 场景 | 行为 |
|---|---|
| `API_TOKEN` 未配置或为空 | 鉴权禁用，所有请求直接通过 |
| `API_TOKEN` 已配置 | 必须携带有效 Token |
| 缺少 `Authorization` 头 | 返回 HTTP `401` |
| Token 格式错误（非 Bearer） | 返回 HTTP `401` |
| Token 不匹配 | 返回 HTTP `401` |
| Token 正常 | 正常处理请求 |

### 错误响应

**缺少 Authorization 头**:

```json
{
  "code": 401,
  "message": "missing or invalid Authorization header, expected: Bearer <token>",
  "data": null
}
```

**Token 不匹配**:

```json
{
  "code": 401,
  "message": "invalid token",
  "data": null
}
```

### 示例

```bash
# 无 token（鉴权禁用时可访问）
curl http://localhost:3000/api/overview

# 带 token
curl -H "Authorization: Bearer my_secret_token" http://localhost:3000/api/overview

# 错误 token
curl -H "Authorization: Bearer wrong_token" http://localhost:3000/api/overview
# → HTTP 401
```

---

## 3. 端点列表

| 方法 | 路径 | 说明 |
|---|---|---|
| `GET` | `/api/overview` | 大屏总览指标 |
| `GET` | `/api/hot-videos` | 热门视频排行（分页） |
| `GET` | `/api/partition-compare` | 分区竞争力对比 |
| `GET` | `/api/lifecycle` | 单视频生命周期 |
| `GET` | `/api/lifecycle/top` | TOP N 热门视频生命周期 |
| `GET` | `/api/partition-detail` | 分区下钻详情 |
| `GET` | `/api/word-cloud` | 词云（标题/评论/弹幕） |
| `GET` | `/api/hot-comments` | 热门评论排行 |
| `GET` | `/api/interaction` | 视频互动质量 |

---

## 3. `/api/overview`

大屏总览指标。

### 请求

```
GET /api/overview
```

无参数。

### 响应

```json
{
  "code": 0,
  "message": "ok",
  "data": {
    "video_count": 954,
    "category_count": 10,
    "up_count": 935,
    "total_views": 1966517446,
    "total_interactions": 147167757,
    "latest_snapshot_date": "20260601",
    "update_time": "2026-06-01 11:47:05"
  }
}
```

### 字段说明

| 字段 | 类型 | 说明 |
|---|---|---|
| `video_count` | int | 最新快照中的视频数 |
| `category_count` | int | 涉及分区数（通常 ≤10，含"其他"分区） |
| `up_count` | int | 涉及 UP 主数 |
| `total_views` | int | 样本累计播放量 |
| `total_interactions` | int | 点赞+投币+收藏+评论+分享总和 |
| `latest_snapshot_date` | string\|null | 最新快照日期 `yyyyMMdd`，无数据时为 `null` |
| `update_time` | string | 数据更新时间 `yyyy-MM-dd HH:mm:ss` |

### 空数据返回

```json
{
  "code": 0,
  "message": "ok",
  "data": {
    "video_count": 0,
    "category_count": 0,
    "up_count": 0,
    "total_views": 0,
    "total_interactions": 0,
    "latest_snapshot_date": null,
    "update_time": "2026-06-01 12:00:00"
  }
}
```

> `update_time` 在无数据时返回当前服务时间。

---

## 4. `/api/hot-videos`

热门视频排行，支持排序和分页。

### 请求

```
GET /api/hot-videos?sort=heat&limit=20&offset=0
```

| 参数 | 类型 | 必填 | 默认值 | 说明 |
|---|---|---|---|---|
| `sort` | string | 否 | `heat` | 排序字段 |
| `limit` | int | 否 | `20` | 返回条数（0~100） |
| `offset` | int | 否 | `0` | 偏移量（≥0） |

**`sort` 合法值**:

| 值 | 排序依据 |
|---|---|
| `heat` | 综合热度 `heat_score` 降序 |
| `views` | 播放量 `view_count` 降序 |
| `likes` | 点赞数 `like_count` 降序 |
| `coins` | 投币数 `coin_count` 降序 |

### 响应

```json
{
  "code": 0,
  "message": "ok",
  "data": {
    "total": 100,
    "videos": [
      {
        "bvid": "BV1TfZcYvEs6",
        "title": "陪伴我10年的员工离开了....",
        "category": "生活",
        "up_name": "逗比的雀巢",
        "view_count": 15267571,
        "like_count": 1295325,
        "coin_count": 1240755,
        "favorite_count": 443893,
        "reply_count": 24866,
        "share_count": 75620,
        "rank_position": 5,
        "heat_score": 0.9001
      }
    ]
  }
}
```

### 字段说明

| 字段 | 类型 | 说明 |
|---|---|---|
| `total` | int | 符合条件的视频总数 |
| `videos` | array | 视频列表 |
| `videos[].bvid` | string | BV 号 |
| `videos[].title` | string | 标题 |
| `videos[].category` | string | 分区 |
| `videos[].up_name` | string | UP 主名 |
| `videos[].view_count` | int | 播放量 |
| `videos[].like_count` | int | 点赞数 |
| `videos[].coin_count` | int | 投币数 |
| `videos[].favorite_count` | int | 收藏数 |
| `videos[].reply_count` | int | 评论数 |
| `videos[].share_count` | int | 分享数 |
| `videos[].rank_position` | int | 排名 |
| `videos[].heat_score` | double | 综合热度（0~1） |

### 错误场景

| 场景 | HTTP | code | message |
|---|---|---|---|
| `sort` 不合法 | `400` | `400` | `invalid sort, allowed values: heat, views, likes, coins` |
| `limit < 0` 或 `limit > 100` | `400` | `400` | `limit must be between 0 and 100` |
| `offset < 0` | `400` | `400` | `offset must be >= 0` |
| `limit = 0` | `200` | `0` | 返回 `videos = []`，`total` 为实际总数 |

### 示例

```bash
# 默认排序，前 10 条
curl "http://localhost:3000/api/hot-videos?sort=heat&limit=10"

# 按播放量排序，第 11~20 条
curl "http://localhost:3000/api/hot-videos?sort=views&limit=10&offset=10"

# 非法排序参数
curl "http://localhost:3000/api/hot-videos?sort=invalid"
# → HTTP 400
```

---

## 5. `/api/partition-compare`

分区竞争力对比，返回各分区的聚合指标。

### 请求

```
GET /api/partition-compare
```

无参数。

### 响应

```json
{
  "code": 0,
  "message": "ok",
  "data": {
    "partitions": [
      {
        "category": "生活",
        "hot_video_count": 104,
        "avg_views": 3527463,
        "avg_like_rate": 0.0559,
        "avg_coin_rate": 0.0097,
        "avg_fav_rate": 0.0124,
        "top_view_count": 19240588,
        "total_views": 366856188,
        "up_count": 104
      }
    ]
  }
}
```

### 字段说明

| 字段 | 类型 | 说明 |
|---|---|---|
| `partitions` | array | 分区列表 |
| `partitions[].category` | string | 分区名 |
| `partitions[].hot_video_count` | int | 上榜视频数 |
| `partitions[].avg_views` | int | 平均播放量 |
| `partitions[].avg_like_rate` | double | 平均点赞率（0~1） |
| `partitions[].avg_coin_rate` | double | 平均投币率（0~1） |
| `partitions[].avg_fav_rate` | double | 平均收藏率（0~1） |
| `partitions[].top_view_count` | int | 该分区最高播放量 |
| `partitions[].total_views` | int | 该分区总播放量 |
| `partitions[].up_count` | int | 该分区 UP 主数 |

> 排序按 `total_views` 降序。

### 空数据返回

```json
{
  "code": 0,
  "message": "ok",
  "data": {
    "partitions": []
  }
}
```

---

## 6. `/api/lifecycle`

单个视频的生命周期时间线。

### 请求

```
GET /api/lifecycle?bvid=BV16YGC6XEco
```

| 参数 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `bvid` | string | 是 | 视频 BV 号 |

### 响应

```json
{
  "code": 0,
  "message": "ok",
  "data": {
    "bvid": "BV16YGC6XEco",
    "title": "花30天用巧克力手搓《清明上河图》",
    "category": "其他",
    "timeline": [
      {
        "snapshot_date": "20260531",
        "snapshot_time": "18:42",
        "view_count": 5726720,
        "like_count": 458475,
        "rank_position": 40,
        "rank_source": "popular",
        "rank_name": "全站热门",
        "view_growth": 0,
        "rank_change": 0
      },
      {
        "snapshot_date": "20260601",
        "snapshot_time": "11:20",
        "view_count": 5922819,
        "like_count": 474509,
        "rank_position": 53,
        "rank_source": "popular",
        "rank_name": "全站热门",
        "view_growth": 196099,
        "rank_change": -13
      }
    ]
  }
}
```

### 字段说明

| 字段 | 类型 | 说明 |
|---|---|---|
| `bvid` | string | BV 号 |
| `title` | string | 标题 |
| `category` | string | 分区 |
| `timeline` | array | 时间序列（按 snapshot_id 升序） |
| `timeline[].snapshot_date` | string | 日期 `yyyyMMdd` |
| `timeline[].snapshot_time` | string | 时间 `HH:mm` |
| `timeline[].view_count` | int | 该时间点播放量 |
| `timeline[].like_count` | int | 该时间点点赞数 |
| `timeline[].rank_position` | int | 排名 |
| `timeline[].rank_source` | string | 来源榜单：`popular` / `ranking` |
| `timeline[].rank_name` | string | 来源榜单名 |
| `timeline[].view_growth` | int | 较上一时间点播放增量（首个点为 0） |
| `timeline[].rank_change` | int | 较上一时间点排名变化（正数=排名上升，首个点为 0） |

> **注意**：`rank_change` 正数表示排名上升（数值变小），负数表示排名下降（数值变大）。
>
> 第一轮采集时 timeline 只有 1 个点，`view_growth=0`，`rank_change=0`。

### 错误场景

| 场景 | HTTP | code | message |
|---|---|---|---|
| 缺少 `bvid` | `400` | `400` | `bvid is required` |
| `bvid` 不存在 | `200` | `0` | `data = null` |

---

## 7. `/api/lifecycle/top`

TOP N 热门视频的生命周期。

### 请求

```
GET /api/lifecycle/top?limit=5
```

| 参数 | 类型 | 必填 | 默认值 | 说明 |
|---|---|---|---|---|
| `limit` | int | 否 | `5` | 返回视频数（0~100） |

> TOP 依据：从 `ads_hot_video_rank` 按 `heat_score` 降序选取存在生命周期记录的视频。

### 响应

```json
{
  "code": 0,
  "message": "ok",
  "data": {
    "videos": [
      {
        "bvid": "BV16YGC6XEco",
        "title": "花30天用巧克力手搓《清明上河图》",
        "category": "其他",
        "timeline": [
          {
            "snapshot_date": "20260531",
            "snapshot_time": "18:42",
            "view_count": 5726720,
            "like_count": 458475,
            "rank_position": 40,
            "rank_source": "popular",
            "rank_name": "全站热门",
            "view_growth": 0,
            "rank_change": 0
          }
        ]
      }
    ]
  }
}
```

### 字段说明

| 字段 | 类型 | 说明 |
|---|---|---|
| `videos` | array | 视频列表 |
| `videos[].bvid` | string | BV 号 |
| `videos[].title` | string | 标题 |
| `videos[].category` | string | 分区 |
| `videos[].timeline` | array | 时间线（结构同 `/api/lifecycle`） |

> 若实际数量不足 `limit`，返回全部（不报错）。
>
> `limit=0` 时返回空数组。

### 错误场景

| 场景 | HTTP | code | message |
|---|---|---|---|
| `limit < 0` 或 `limit > 100` | `400` | `400` | `limit must be between 0 and 100` |

---

## 8. `/api/partition-detail`

分区下钻详情，包含分区概要、热门视频和 TOP UP 主。

### 请求

```
GET /api/partition-detail?category=游戏
```

| 参数 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `category` | string | 是 | 分区名，如 `游戏`、`动画`、`音乐` |

### 响应

```json
{
  "code": 0,
  "message": "ok",
  "data": {
    "category": "游戏",
    "hot_video_count": 109,
    "avg_views": 2369599,
    "videos": [
      {
        "bvid": "BV1GyZYYNErW",
        "title": "我的世界克苏鲁全集：一口气看完",
        "view_count": 25314710,
        "heat_score": 0.8494,
        "rank_position": 6
      }
    ],
    "top_ups": [
      {
        "up_id": "411581408",
        "up_name": "这名玩家",
        "follower_count": 462298,
        "hot_video_count": 1,
        "total_views": 25314710
      }
    ]
  }
}
```

### 字段说明

| 字段 | 类型 | 说明 |
|---|---|---|
| `category` | string | 分区名 |
| `hot_video_count` | int | 该分区上榜视频数 |
| `avg_views` | int | 该分区平均播放量 |
| `videos` | array | 该分区热门视频 TOP 10（按 `heat_score` 降序） |
| `videos[].bvid` | string | BV 号 |
| `videos[].title` | string | 标题 |
| `videos[].view_count` | int | 播放量 |
| `videos[].heat_score` | double | 综合热度 |
| `videos[].rank_position` | int | 排名 |
| `top_ups` | array | 该分区 TOP UP 主（按 `total_views` 降序，最多 10 个） |
| `top_ups[].up_id` | string | UP 主 ID |
| `top_ups[].up_name` | string | UP 主名 |
| `top_ups[].follower_count` | int | 粉丝数 |
| `top_ups[].hot_video_count` | int | 该分区上榜视频数 |
| `top_ups[].total_views` | int | 该分区总播放量 |

> **注意**：`top_ups` 通过 `up_name` 关联 `ads_up_summary` 和 `ads_hot_video_rank`，属于展示级近似关联。

### 错误场景

| 场景 | HTTP | code | message |
|---|---|---|---|
| 缺少 `category` | `400` | `400` | `category is required` |
| 分区不存在 | `200` | `0` | `videos = []`，`top_ups = []`，`hot_video_count = 0` |

### 示例

```bash
# 游戏分区下钻
curl "http://localhost:3000/api/partition-detail?category=游戏"

# 动画分区下钻
curl "http://localhost:3000/api/partition-detail?category=动画"

# 不存在的分区
curl "http://localhost:3000/api/partition-detail?category=不存在分区"
# → data.videos = [], data.top_ups = []
```

---

## 9. `/api/word-cloud`

统一词云接口，支持标题词云、评论词云、弹幕词云三种 source。

### 请求

```
GET /api/word-cloud?source=title&category=游戏
```

| 参数 | 类型 | 必填 | 默认值 | 说明 |
|---|---|---|---|---|
| `source` | string | 否 | `title` | 数据来源：`title`（标题）/ `comment`（评论）/ `danmaku`（弹幕） |
| `category` | string | 否 | `全站` | 分区筛选，不传则返回全站 |

### 响应

```json
{
  "code": 0,
  "message": "ok",
  "data": {
    "source": "comment",
    "category": "游戏",
    "words": [
      { "name": "剧情", "value": 42 },
      { "name": "操作", "value": 37 },
      { "name": "画质", "value": 28 }
    ]
  }
}
```

### 字段说明

| 字段 | 类型 | 说明 |
|---|---|---|
| `source` | string | 数据来源类型 |
| `category` | string | 分区名 |
| `words` | array | 词列表（按 `value` 降序，最多 100 个） |
| `words[].name` | string | 词 |
| `words[].value` | int | 出现次数 |

> 三种 source 的语义：
> - `title`：内容主题词云（视频标题分词）
> - `comment`：观众讨论词云（评论内容分词）
> - `danmaku`：即时反应词云（弹幕内容分词）

### 错误场景

| 场景 | HTTP | code | message |
|---|---|---|---|
| `source` 非法 | `400` | `400` | `invalid source 'xxx', allowed: title, comment, danmaku` |
| 无数据 | `200` | `0` | `words = []` |

### 示例

```bash
# 全站标题词云
curl "http://localhost:9090/api/word-cloud"

# 游戏分区评论词云
curl "http://localhost:9090/api/word-cloud?source=comment&category=游戏"

# 全站弹幕词云
curl "http://localhost:9090/api/word-cloud?source=danmaku"
```

---

## 10. `/api/hot-comments`

热门评论排行，按点赞数排序。

### 请求

```
GET /api/hot-comments?category=游戏&limit=20
GET /api/hot-comments?bvid=BVxxx&limit=10
```

| 参数 | 类型 | 必填 | 默认值 | 说明 |
|---|---|---|---|---|
| `category` | string | 否 | 无 | 分区筛选 |
| `bvid` | string | 否 | 无 | 单视频筛选（优先级高于 category） |
| `limit` | int | 否 | `20` | 返回数量，0~100 |

### 响应

```json
{
  "code": 0,
  "message": "ok",
  "data": {
    "comments": [
      {
        "bvid": "BV16YGC6XEco",
        "title": "视频标题",
        "category": "游戏",
        "up_name": "UP主",
        "user_name": "评论用户",
        "content": "高赞评论内容",
        "like_count": 5750,
        "reply_count": 67,
        "comment_time": "2026-06-02 12:00:00"
      }
    ]
  }
}
```

### 字段说明

| 字段 | 类型 | 说明 |
|---|---|---|
| `comments` | array | 评论列表（按 `like_count` 降序） |
| `comments[].bvid` | string | 视频 BV 号 |
| `comments[].title` | string | 视频标题 |
| `comments[].category` | string | 分区 |
| `comments[].up_name` | string | UP 主名 |
| `comments[].user_name` | string | 评论用户名 |
| `comments[].content` | string | 评论内容 |
| `comments[].like_count` | long | 点赞数 |
| `comments[].reply_count` | long | 回复数 |
| `comments[].comment_time` | string | 评论时间 |

### 错误场景

| 场景 | HTTP | code | message |
|---|---|---|---|
| `limit` 超出范围 | `400` | `400` | `limit must be between 0 and 100` |
| 无数据 | `200` | `0` | `comments = []` |

### 示例

```bash
# 全站热门评论
curl "http://localhost:9090/api/hot-comments"

# 游戏分区热门评论
curl "http://localhost:9090/api/hot-comments?category=游戏"

# 单视频热门评论
curl "http://localhost:9090/api/hot-comments?bvid=BV16YGC6XEco&limit=10"
```

---

## 11. `/api/interaction`

单个视频的互动质量分析。

### 请求

```
GET /api/interaction?bvid=BV16YGC6XEco
```

| 参数 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `bvid` | string | 是 | 视频 BV 号 |

### 响应

```json
{
  "code": 0,
  "message": "ok",
  "data": {
    "bvid": "BV16YGC6XEco",
    "view_count": 5922819,
    "like_count": 474509,
    "coin_count": 204080,
    "favorite_count": 115759,
    "reply_count": 5061,
    "share_count": 11602,
    "like_rate": 0.0801,
    "coin_rate": 0.0345,
    "fav_rate": 0.0195,
    "reply_rate": 0.00085,
    "share_rate": 0.00196
  }
}
```

### 字段说明

| 字段 | 类型 | 说明 |
|---|---|---|
| `bvid` | string | BV 号 |
| `view_count` | int | 播放量 |
| `like_count` | int | 点赞数 |
| `coin_count` | int | 投币数 |
| `favorite_count` | int | 收藏数 |
| `reply_count` | int | 评论数 |
| `share_count` | int | 分享数 |
| `like_rate` | double | 点赞率 = like_count / view_count |
| `coin_rate` | double | 投币率 = coin_count / view_count |
| `fav_rate` | double | 收藏率 = favorite_count / view_count |
| `reply_rate` | double | 评论率 = reply_count / view_count |
| `share_rate` | double | 分享率 = share_count / view_count |

> 所有率值基于 `view_count` 计算。`view_count <= 0` 时，所有率值返回 `0.0`。
>
> 指标口径基于"样本"（当前快照数据），非全站真实数据。

### 错误场景

| 场景 | HTTP | code | message |
|---|---|---|---|
| 缺少 `bvid` | `400` | `400` | `bvid is required` |
| `bvid` 不存在 | `200` | `0` | `data = null` |

---

## 12. 错误码与状态码

### HTTP 状态码映射

| HTTP Status | JSON `code` | 场景 |
|---|---|---|
| `200 OK` | `0` | 成功、资源不存在 |
| `400 Bad Request` | `400` | 参数错误（缺少必填参数、参数值非法） |
| `500 Internal Server Error` | `500` | 数据库连接失败、SQL 执行错误 |

### 错误响应示例

**参数错误 (400)**:
```json
{
  "code": 400,
  "message": "invalid sort, allowed values: heat, views, likes, coins",
  "data": null
}
```

**资源不存在 (200)**:
```json
{
  "code": 0,
  "message": "ok",
  "data": null
}
```

**服务器错误 (500)**:
```json
{
  "code": 500,
  "message": "internal server error",
  "data": null
}
```

---

## 13. TypeScript 类型定义

> 以下类型与 API JSON 一一对应，前端可直接用于 `types/index.ts`。

```typescript
// ---- 通用 ----
export interface ApiResponse<T> {
  code: number;
  message: string;
  data: T;
}

// ---- GET /api/overview ----
export interface OverviewData {
  video_count: number;
  category_count: number;
  up_count: number;
  total_views: number;
  total_interactions: number;
  latest_snapshot_date: string | null;
  update_time: string;
}

// ---- GET /api/hot-videos ----
export interface HotVideoItem {
  bvid: string;
  title: string;
  category: string;
  up_name: string;
  view_count: number;
  like_count: number;
  coin_count: number;
  favorite_count: number;
  reply_count: number;
  share_count: number;
  rank_position: number;
  heat_score: number;
}

export interface HotVideosData {
  total: number;
  videos: HotVideoItem[];
}

export type HotVideosSort = 'heat' | 'views' | 'likes' | 'coins';

// ---- GET /api/partition-compare ----
export interface PartitionCompareItem {
  category: string;
  hot_video_count: number;
  avg_views: number;
  avg_like_rate: number;
  avg_coin_rate: number;
  avg_fav_rate: number;
  top_view_count: number;
  total_views: number;
  up_count: number;
}

export interface PartitionCompareData {
  partitions: PartitionCompareItem[];
}

// ---- GET /api/lifecycle ----
export interface LifecyclePoint {
  snapshot_date: string;
  snapshot_time: string;
  view_count: number;
  like_count: number;
  rank_position: number;
  rank_source: string;
  rank_name: string;
  view_growth: number;
  rank_change: number;
}

export interface LifecycleData {
  bvid: string;
  title: string;
  category: string;
  timeline: LifecyclePoint[];
}

// ---- GET /api/lifecycle/top ----
export interface LifecycleTopData {
  videos: LifecycleData[];
}

// ---- GET /api/partition-detail ----
export interface PartitionDetailVideo {
  bvid: string;
  title: string;
  view_count: number;
  heat_score: number;
  rank_position: number;
}

export interface PartitionDetailUp {
  up_id: string;
  up_name: string;
  follower_count: number;
  hot_video_count: number;
  total_views: number;
}

export interface PartitionDetailData {
  category: string;
  hot_video_count: number;
  avg_views: number;
  videos: PartitionDetailVideo[];
  top_ups: PartitionDetailUp[];
}

// ---- GET /api/word-cloud ----
export interface WordCloudItem {
  name: string;
  value: number;
}

export type WordCloudSource = 'title' | 'comment' | 'danmaku';

export interface WordCloudData {
  source: string;
  category: string;
  words: WordCloudItem[];
}

// ---- GET /api/hot-comments ----
export interface HotCommentItem {
  bvid: string;
  title: string;
  category: string;
  up_name: string;
  user_name: string;
  content: string;
  like_count: number;
  reply_count: number;
  comment_time: string | null;
}

export interface HotCommentsData {
  comments: HotCommentItem[];
}

// ---- GET /api/interaction ----
export interface InteractionData {
  bvid: string;
  view_count: number;
  like_count: number;
  coin_count: number;
  favorite_count: number;
  reply_count: number;
  share_count: number;
  like_rate: number;
  coin_rate: number;
  fav_rate: number;
  reply_rate: number;
  share_rate: number;
}
```

---

## 附录: 启动与验证

### 启动服务

**本地开发**:

```bash
cd backend
cargo run
# listening on 0.0.0.0:9090
```

**WSL 部署**:

```bash
ssh UbuntuDesktop
cd ~/backend
./bilibili-hot-backend &
```

### 环境变量

| 变量 | 默认值 | 说明 |
|---|---|---|
| `DATABASE_URL` | `mysql://hive:Hive@123456@192.168.100.10:3306/bilibili` | MySQL 连接串 |
| `BIND` | `0.0.0.0:9090` | 监听地址和端口 |
| `API_TOKEN` | 空（鉴权禁用） | API 鉴权 Token |

### FRP 穿透（公网访问）

架构：
```
公网用户 → 111.228.34.34:9091 → frps:7001 → frpc → WSL:9090 → MySQL
```

**WSL (frpc)**:
```bash
cd ~/backend
./frpc -c frpc.toml &
```

**云服务器 (frps)**:
```bash
frps -c /etc/frp/frps.toml &
```

Dashboard: `http://111.228.34.34:7500`（admin / da6d9087cd0ca58b）

### 快速验证

```bash
# 公网访问
curl http://111.228.34.34:9091/api/overview

# 热门视频
curl "http://111.228.34.34:9091/api/hot-videos?sort=heat&limit=5"

# 分区对比
curl http://111.228.34.34:9091/api/partition-compare

# 生命周期
curl "http://111.228.34.34:9091/api/lifecycle?bvid=BV16YGC6XEco"

# TOP N 生命周期
curl "http://111.228.34.34:9091/api/lifecycle/top?limit=3"

# 分区下钻
curl "http://111.228.34.34:9091/api/partition-detail?category=游戏"

# 词云（标题/评论/弹幕）
curl http://111.228.34.34:9091/api/word-cloud
curl "http://111.228.34.34:9091/api/word-cloud?source=comment&category=游戏"
curl "http://111.228.34.34:9091/api/word-cloud?source=danmaku"

# 热门评论
curl http://111.228.34.34:9091/api/hot-comments
curl "http://111.228.34.34:9091/api/hot-comments?category=游戏"

# 互动质量
curl "http://111.228.34.34:9091/api/interaction?bvid=BV16YGC6XEco"
```
