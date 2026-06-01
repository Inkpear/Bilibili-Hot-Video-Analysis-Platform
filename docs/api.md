# B站热门视频分析平台 — 后端 API 文档

> Base URL: `http://localhost:3000`
>
> 技术栈: Rust Axum + sqlx (MySQL)
>
> 更新时间: 2026-06-01

---

## 目录

1. [通用约定](#1-通用约定)
2. [端点列表](#2-端点列表)
3. [/api/overview](#3-apioverview)
4. [/api/hot-videos](#4-apihot-videos)
5. [/api/partition-compare](#5-apipartition-compare)
6. [/api/lifecycle](#6-apilifecycle)
7. [/api/lifecycle/top](#7-apilifecycletop)
8. [/api/partition-detail](#8-apipartition-detail)
9. [/api/tag-cloud](#9-apitag-cloud)
10. [/api/interaction](#10-apiinteraction)
11. [错误码与状态码](#11-错误码与状态码)
12. [TypeScript 类型定义](#12-typescript-类型定义)

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

## 2. 端点列表

| 方法 | 路径 | 说明 |
|---|---|---|
| `GET` | `/api/overview` | 大屏总览指标 |
| `GET` | `/api/hot-videos` | 热门视频排行（分页） |
| `GET` | `/api/partition-compare` | 分区竞争力对比 |
| `GET` | `/api/lifecycle` | 单视频生命周期 |
| `GET` | `/api/lifecycle/top` | TOP N 热门视频生命周期 |
| `GET` | `/api/partition-detail` | 分区下钻详情 |
| `GET` | `/api/tag-cloud` | 标签词云 |
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

## 9. `/api/tag-cloud`

标签词云，基于标题 MapReduce 分词结果。

### 请求

```
GET /api/tag-cloud?category=游戏
```

| 参数 | 类型 | 必填 | 默认值 | 说明 |
|---|---|---|---|---|
| `category` | string | 否 | `全站` | 分区筛选，不传则返回全站 |

### 响应

```json
{
  "code": 0,
  "message": "ok",
  "data": {
    "tags": [
      { "name": "鸣潮", "value": 4 },
      { "name": "漫展", "value": 3 },
      { "name": "生日会", "value": 3 }
    ]
  }
}
```

### 字段说明

| 字段 | 类型 | 说明 |
|---|---|---|
| `tags` | array | 标签列表（按 `value` 降序，最多 100 个） |
| `tags[].name` | string | 标签名 |
| `tags[].value` | int | 出现次数（>0） |

> **注意**：热门视频的 `tags` 原始字段全空（B站数据现象），词云来自 `scripts/run_tag_cloud_mr.sh` 对标题的 MapReduce 分词。
>
> 无 `category` 参数或 `category` 为空字符串时，返回 `category='全站'` 的词云。

### 错误场景

| 场景 | HTTP | code | message |
|---|---|---|---|
| 分区无标签数据 | `200` | `0` | `tags = []` |

### 示例

```bash
# 全站词云
curl "http://localhost:3000/api/tag-cloud"

# 游戏分区词云
curl "http://localhost:3000/api/tag-cloud?category=游戏"

# 空字符串视为全站
curl "http://localhost:3000/api/tag-cloud?category="
```

---

## 10. `/api/interaction`

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

## 11. 错误码与状态码

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

## 12. TypeScript 类型定义

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

// ---- GET /api/tag-cloud ----
export interface TagCloudItem {
  name: string;
  value: number;
}

export interface TagCloudData {
  tags: TagCloudItem[];
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

```bash
cd backend
cargo run
# listening on 0.0.0.0:3000
```

### 环境变量

| 变量 | 默认值 | 说明 |
|---|---|---|
| `DATABASE_URL` | `mysql://hive:Hive@123456@127.0.0.1:3306/bilibili` | MySQL 连接串 |

### 快速验证

```bash
# 总览
curl http://localhost:3000/api/overview

# 热门视频
curl "http://localhost:3000/api/hot-videos?sort=heat&limit=5"

# 分区对比
curl http://localhost:3000/api/partition-compare

# 生命周期
curl "http://localhost:3000/api/lifecycle?bvid=BV16YGC6XEco"

# TOP N 生命周期
curl "http://localhost:3000/api/lifecycle/top?limit=3"

# 分区下钻
curl "http://localhost:3000/api/partition-detail?category=游戏"

# 标签词云
curl http://localhost:3000/api/tag-cloud
curl "http://localhost:3000/api/tag-cloud?category=游戏"

# 互动质量
curl "http://localhost:3000/api/interaction?bvid=BV16YGC6XEco"
```
