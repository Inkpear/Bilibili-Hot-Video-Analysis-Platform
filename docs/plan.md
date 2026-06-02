# Bilibili Hot Video Analysis Platform

> 大数据课设项目 —— 基于 Hadoop 的哔哩哔哩热门视频分析与可视化

## 1. Architecture

```
B站排行 API (Python requests)
        │
        ▼
  本地 NDJSON 文件  ──hdfs dfs -put──▶  HDFS  (/data/bilibili/ods/)
                                            │
                                    Hive 外部表 (ODS)
                                            │
                                   Spark SQL 清洗 (DWD)
                                            │
                              Spark SQL / Hive SQL 分析 (DWS → ADS)
                                            │
                                  Spark JDBC 导出
                                            │
                                        MySQL (ADS)
                                            │
                                   Rust Axum (只读 API)
                                            │
                                  React + ECharts (大屏)
```

## 2. Environment Setup

| 组件 | 用途 | 版本 | 验证命令 |
|---|---|---|---|
| Hadoop | HDFS + YARN | 3.x | `hdfs dfs -ls /` |
| Hive | 数据仓库 | 3.x | `hive -e "SHOW DATABASES;"` |
| Spark | 清洗与分析 | 3.x | `spark-sql --version` |
| MySQL | ADS 存储 | 8.x | `mysql -u root -e "SELECT 1;"` |
| Rust (Axum) | 后端 API | latest stable | `cargo --version` |
| Node.js + React | 前端大屏 | 18+ | `node --version` |

额外依赖：
- Hive `hive-hcatalog-core` jar（提供 JsonSerDe）
- MySQL JDBC driver `mysql-connector-j`（Spark 访问 MySQL）

## 3. Data Specification

### 3.1 采集范围

| 参数 | 值 |
|---|---|
| 数据源 | 全站热门 + 9 个分区榜 |
| 包含分区 | 动画(1)、游戏(4)、娱乐(5)、生活(160)、音乐(3)、舞蹈(129)、科技(188)、知识(36)、影视(181) |
| 采集周期 | 7 天 |
| 快照频率 | 每天 1 轮，共 7 轮 |
| 评论限制 | 每视频 200 条（去重后只采一次） |
| 弹幕限制 | 每视频 3000 条（去重后只采一次） |

**采集策略**（日结事务模式）：

每轮对每个新发现或上次有缺失的 bvid，按序采集 /view → UP → 评论 → 弹幕。
全部成功则标完成（has_detail/comments/danmaku 全部=1），任何一项失败则 fail_count++（≥3 永久放弃）。
UP 主作为视频内嵌环节独立提交（成功即写入，不参与视频事务回滚）。

> **已知数据限制**：
> - B站热门视频 `/x/web-interface/view` 返回的 `tags` 字段全部为空（数据源现象，非代码问题）。标签词云需通过后续 Stage 的 MapReduce 标题分词填补。
> - 生命周期图表（`ads_video_lifecycle`）Day 1 只有 1 个快照点，`view_growth=0`、`rank_change=0`。至少 2 天采集后才有增量和排名变化数据。

| 数据类型 | 采集频率 | 去重键 | 说明 |
|---|---|---|---|
| 排行榜快照 | 每天 1 次 | `bvid + snapshot_id + rank_source` | 7 个时间点，每次全量采集 |
| 视频详情 | 首次 + 缺失重试 | `bvid` | has_detail=0 时采集，失败下轮重试 |
| UP 主信息 | 首次 + 失败冷却 | `up_id` | 嵌入 per-video 循环，失败 3 次后 24h 冷却 |
| 评论 | 首次 + 缺失重试 | `comment_id` | 每 bvid 最多 200 条，采集完标记 has_comments=1 |
| 弹幕 | 首次 + 缺失重试 | `danmaku_id` | 每 bvid 最多 3000 条，采集完标记 has_danmaku=1 |

进度展示：TTY 前台运行时使用 tqdm；systemd 非 TTY 运行时输出逐视频 `INFO` 文本日志。

**自动化策略**：

采集部署为 user-level systemd 常驻服务（`crawler.service`），采用"立即启动 + 循环休眠"模式：

1. **启动行为**：服务启动后立即开始第一轮采集
2. **循环逻辑**：采集完成 → 运行 ETL 管线（HDFS 上传 + Spark ETL + 词云 + MySQL 导出）→ 休眠 6 小时 → 下一轮
3. **停止行为**：收到 SIGTERM 信号后，当前轮结束后优雅退出
4. **重启策略**：`Restart=always`，服务异常退出后 60 秒自动重启

核心脚本 `scripts/crawler_loop.sh`：
```bash
while [ "$stop_requested" -eq 0 ]; do
    python3 main.py --once          # 采集
    bash run_pipeline.sh            # ETL
    sleep 21600                     # 休眠 6 小时
done
```

环境变量：
- `CRAWLER_SLEEP_SECONDS`：休眠间隔（默认 21600 = 6 小时）
- `CRAWLER_DIR`：crawler 脚本目录（默认 `$HOME/crawler`）
- `SCRIPTS_DIR`：scripts 目录（默认 `$HOME/scripts`）

查看日志：`journalctl --user -u crawler.service -f`
停止服务：`systemctl --user stop crawler.service`

### 3.2 数据规模

| 项目 | 估算 |
|---|---|
| 独立视频 | ~500 个（保守估计，7 天榜单轮换可能达 1000-2000） |
| 视频快照 | ~3,500 行（500 × 7，实际按轮换频率可达 7,000-14,000） |
| 评论 | ~100,000 行 |
| 弹幕 | ~1,500,000 行 |
| ODS 总大小 | ~500MB~2GB |

> 以上为保守下限。实际独立视频数取决于榜单轮换频率。7 天内可能积累 1000-2000 个独立 bvid。Hadoop 环境需预留足够存储空间。

### 3.3 统一字段字典

> 以下字段从采集到 API 全链路使用，是整个项目的字段契约。

**视频基础信息 (dwd_video_info)**

| 字段 | 类型 | 来源 | 说明 |
|---|---|---|---|
| bvid | STRING | `item["bvid"]` | BV 号，主键 |
| title | STRING | `item["title"]` | 标题 |
| description | STRING | `/view` 补充 | 简介 |
| category | STRING | 请求分区或 tname 向上映射 | 九大分区名（动画/游戏/…）；popular 按 tname 归并，无法归并时为"其他" |
| sub_category | STRING | `item["tname"]` | API 返回的子分区名 |
| tags | STRING | `/view` 补充 | 逗号分隔标签 |
| duration | INT | `item["duration"]` | 秒 |
| publish_time | BIGINT | `item["pubdate"]` | Unix 秒级时间戳 |
| up_id | STRING | `item["owner"]["mid"]` | UP 主 mid |
| up_name | STRING | `item["owner"]["name"]` | UP 主名 |
| up_avatar | STRING | `item["owner"]["face"]` | UP 主头像 URL |
| url | STRING | 拼接 | 视频链接 |
| aid | BIGINT | `item["aid"]` | 评论接口的 oid |
| cid | BIGINT | `item["cid"]` | 弹幕接口的 oid |
| rank_source | STRING | 采集时设定 | `popular` / `ranking` |
| rank_name | STRING | 采集时设定 | 来源榜单名，如"全站热门"/"游戏榜" |

**视频快照 (dwd_video_snapshot)** — 分区键 `snapshot_date`

| 字段 | 类型 | 来源 |
|---|---|---|
| bvid | STRING | `item["bvid"]` |
| aid | BIGINT | `item["aid"]` |
| view_count | BIGINT | `item["stat"]["view"]` |
| like_count | BIGINT | `item["stat"]["like"]` |
| coin_count | BIGINT | `item["stat"]["coin"]` |
| favorite_count | BIGINT | `item["stat"]["favorite"]` |
| share_count | BIGINT | `item["stat"]["share"]` |
| reply_count | BIGINT | `item["stat"]["reply"]` |
| danmaku_count | BIGINT | `item["stat"]["danmaku"]` |
| rank_position | INT | 采集时序号 |
| rank_source | STRING | `popular` / `ranking` |
| rank_name | STRING | 来源榜单名，如"全站热门"/"游戏榜" |
| snapshot_id | STRING | 快照批次标识，格式 `yyyyMMdd_HHmm`，同一批次所有记录共用 |
| collect_time | BIGINT | 采集时毫秒级时间戳 |
| snapshot_date | STRING | 从 snapshot_id 提取 yyyyMMdd |

**UP 主 (dwd_up_info)**

| 字段 | 类型 | 来源 |
|---|---|---|
| up_id | BIGINT | `d["mid"]` |
| up_name | STRING | `d["name"]` |
| up_avatar | STRING | `d["face"]` |
| follower_count | BIGINT | `d["follower"]` |
| video_count | BIGINT | `d["video"]` |
| sign | STRING | `d["sign"]` |
| official_verify | STRING | `d["official"]["desc"]` |

**评论 (dwd_comment_info)**

| 字段 | 类型 | 来源 |
|---|---|---|
| comment_id | BIGINT | `r["rpid"]` |
| bvid | STRING | 采集时关联（从排行响应获取） |
| aid | BIGINT | 评论接口的 oid |
| user_id | BIGINT | `r["mid"]` |
| user_name | STRING | `r["member"]["uname"]` |
| content | STRING | `r["content"]["message"]` |
| like_count | BIGINT | `r["like"]` |
| reply_count | BIGINT | `r["rcount"]` |
| comment_time | BIGINT | `r["ctime"]`（Unix 秒） |

**弹幕 (dwd_danmaku_info)**

| 字段 | 类型 | 来源 |
|---|---|---|
| danmaku_id | BIGINT | protobuf `id` 字段 |
| bvid | STRING | 采集时关联 |
| cid | BIGINT | 采集时关联 |
| content | STRING | protobuf `content` 字段 |
| video_time | DOUBLE | protobuf `progress` / 1000.0（毫秒→秒） |
| mode | INT | protobuf `mode` |
| fontsize | INT | protobuf `fontsize` |
| color | BIGINT | protobuf `color` |
| mid_hash | STRING | protobuf `mid_hash` |
| send_time | BIGINT | protobuf `ctime`（Unix 秒） |

### 3.4 分析指标

| 指标 | 公式 |
|---|---|
| 点赞率 | `like_count / NULLIF(view_count, 0)` |
| 投币率 | `coin_count / NULLIF(view_count, 0)` |
| 收藏率 | `favorite_count / NULLIF(view_count, 0)` |
| 评论率 | `reply_count / NULLIF(view_count, 0)` |
| 综合互动率 | `(like + coin + favorite + reply + share) / NULLIF(view_count, 0)` |
| 综合热度 | `view_norm*0.4 + like_norm*0.2 + coin_norm*0.15 + fav_norm*0.15 + reply_norm*0.05 + share_norm*0.05` |

**归一化方法**（min-max）：

```
log_x = LN(x + 1)
x_norm = COALESCE((log_x - log_min) / NULLIF(log_max - log_min, 0), 0)
```

若 `log_max = log_min`（所有值相同），`x_norm = 0`。使用 `COALESCE` 确保不会产生 NULL。

### 3.5 数据仓库表

| 层 | 表 | 存储 | 分区 | 说明 |
|---|---|---|---|---|
| ODS | ods_rank_video_raw | Hive/HDFS | — | 排行榜原始快照 |
| ODS | ods_video_comment_raw | Hive/HDFS | — | 评论原始数据 |
| ODS | ods_video_danmaku_raw | Hive/HDFS | — | 弹幕原始数据 |
| ODS | ods_up_raw | Hive/HDFS | — | UP 主原始数据 |
| DWD | dwd_video_info | Hive | — | 视频基础信息（按 bvid 唯一） |
| DWD | dwd_video_snapshot | Hive | snapshot_date | 视频互动快照 |
| DWD | dwd_comment_info | Hive | — | 评论明细 |
| DWD | dwd_danmaku_info | Hive | — | 弹幕明细 |
| DWD | dwd_up_info | Hive | — | UP 主信息 |
| DWS | dws_partition_daily_stats | Hive | snapshot_date | 分区每日汇总 |
| DWS | dws_video_lifecycle_stats | Hive | — | 视频生命周期（按 snapshot_id 排序） |
| DWS | dws_up_summary_stats | Hive | — | UP 主汇总 |
| ADS | ads_screen_overview | MySQL | — | 大屏总览 |
| ADS | ads_hot_video_rank | MySQL | — | 热门视频排行 |
| ADS | ads_partition_compare | MySQL | — | 分区竞争力 |
| ADS | ads_video_lifecycle | MySQL | — | 视频生命周期 |
| ADS | ads_partition_daily | MySQL | — | 分区每日汇总 |
| ADS | ads_up_summary | MySQL | — | UP 主汇总 |
| ADS | ads_word_cloud | MySQL | — | 统一词云（标题/评论/弹幕） |
| ADS | ads_hot_comments | MySQL | — | 热门评论 |

HDFS 目录结构：

```
/data/bilibili/
├── ods/
│   ├── rank_video/
│   ├── video_comment/
│   ├── video_danmaku/
│   └── up_info/
├── dwd/
│   ├── video_info/
│   ├── video_snapshot/
│   ├── comment_info/
│   ├── danmaku_info/
│   └── up_info/
└── dws/
    ├── partition_daily/
    ├── video_lifecycle/
    └── up_summary/
```

### 3.6 数据清洗规则

| 清洗项 | 规则 |
|---|---|
| 去重 | 快照：`bvid + snapshot_id + rank_source`；基础信息：`bvid`；评论：`comment_id`；弹幕：`danmaku_id`；UP主：`up_id` |
| 缺失 | `bvid`/`title`/`view_count` 为空则丢弃；其他字段为空填默认值（0 或空字符串） |
| 格式 | Unix 秒级时间戳 → `yyyy-MM-dd HH:mm:ss`；JSON 数组 tags → 逗号分隔字符串 |
| 异常 | 负值置 0；播放量 < 点赞数时保留但标记 |
| 命名 | 全部字段用 `snake_case` |

## 4. Stage-by-stage Implementation

### Stage 1: Data Collection (Python)

**目标**：爬取 B 站榜单数据，输出本地 NDJSON 文件（每行一个 JSON 对象）。

**B站 API 端点**（2026-05-30 实测验证）：

| 用途 | API 端点 | 关键参数 | 需要 Cookie | Demo |
|---|---|---|---|---|
| 全站热门 | `GET /x/web-interface/popular` | `pn`, `ps` | ❌ | [`api-demo/demo_popular.py`](../api-demo/demo_popular.py) |
| 分区排行 | `GET /x/web-interface/ranking/v2` | `rid`, `type` | ✅ | [`api-demo/demo_ranking.py`](../api-demo/demo_ranking.py) |
| 视频详情 | `GET /x/web-interface/view` | `bvid` | ✅ | [`api-demo/demo_view.py`](../api-demo/demo_view.py) |
| 评论列表 | `GET /x/v2/reply` | `type=1`, `oid`(aid), `pn`, `ps`, `sort=1` | ✅ | [`api-demo/demo_reply.py`](../api-demo/demo_reply.py) |
| 弹幕内容 | `GET /x/v2/dm/web/seg.so` | `type=1`, `oid`(cid), `segment_index` | ❌ | [`api-demo/demo_danmaku.py`](../api-demo/demo_danmaku.py) |
| UP主信息 | `GET /x/wbi/acc/info` | `mid`, `w_rid`, `wts` | ✅ + WBI | [`api-demo/demo_up_info.py`](../api-demo/demo_up_info.py) |

> 所有 Demo 脚本自包含，详情见 [`api-demo/README.md`](../api-demo/README.md)。

**实测说明**：
- popular 和 ranking API 的响应已内嵌 `stat` 和 `owner`，**无需再调用 `/x/web-interface/archive/stat`**。
- 视频 description 和 tags 需通过 `/x/web-interface/view` 补充获取。tags 来自 `data["tags"]`（列表结构，需提取 `tag_name` 拼接为逗号分隔字符串），`data["dynamic"]` 是视频动态文案不是标签。
- follower_count 需通过 WBI 签名的 UP 主接口 `/x/wbi/acc/info` 获取。
- 评论的 `oid` 参数是视频的 `aid`（不是 bvid）。
- 弹幕的 `oid` 参数是视频的 `cid`，使用段式 API，每 6 分钟一段。
- **分区 `category` 映射规则**：`ranking` 来源直接用 `CATEGORY_NAMES[rid]`（九大分区之一）；`popular` 来源使用 API 返回的 `item["tname"]` 向上归并到九大分区（如"MAD·AMV"→"动画"），无法归并时设为"其他"。采集时需维护一份 `tname → category` 映射表。
- **快照批次 `snapshot_id`**：每天一轮采集开始时生成一个批次标识，格式 `yyyyMMdd_HHmm`（如 `20260601_0000`）。本轮所有 API 请求产生的记录共用同一个 `snapshot_id`。后续 ADS 中"最新快照"统一用 `MAX(snapshot_id)` 而非逐条 `collect_time`。

**Cookie 配置**：

```python
# crawler/config.py — 使用环境变量，不硬编码
import os

HEADERS = {
    "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
    "Referer": "https://www.bilibili.com",
}
COOKIE = os.environ.get("BILI_COOKIE", "")
DELAY = 2.0

CATEGORY_RIDS = {
    "动画": 1, "游戏": 4, "娱乐": 5, "生活": 160,
    "音乐": 3, "舞蹈": 129, "科技": 188, "知识": 36, "影视": 181,
}
CATEGORY_NAMES = {v: k for k, v in CATEGORY_RIDS.items()}
```

**脚本模块**：

```
crawler/
├── main.py              # 入口，per-video 事务编排
├── rank_spider.py       # 全站热门 + 分区排行（popular + ranking/v2）
├── video_detail.py      # 视频详情补充字段（description、tags、aid、cid）
├── up_info.py           # UP 主粉丝数（WBI 签名）
├── comment.py           # 评论（oid=aid，sort=1，每 bvid 最多 200 条）
├── danmaku.py           # 弹幕（oid=cid，protobuf 解析，每 bvid 最多 3000 条）
├── db.py                # SQLite 状态管理：去重/失败追踪/快照日志
├── config.py            # 全局配置：Cookie/DELAY/分区映射
└── utils.py             # 请求头、限速(≥2s)、重试、WBI 签名、protobuf 解析
```

**采集输出格式**：每个采集批次一个目录，所有文件均为 NDJSON（每行一个 JSON 对象）。

```
output/20260601_0000/
├── rank_video.ndjson      # 每行一条：{bvid, title, category, ..., snapshot_id, collect_time, rank_source, rank_name, rank_position}
├── up_info.ndjson         # 每行一条：{up_id, up_name, follower_count, ...}
├── comments/
│   ├── BV1xxx.ndjson      # 每行一条评论，含 bvid 字段
│   └── BV2xxx.ndjson
└── danmaku/
    ├── BV1xxx.ndjson      # 每行一条弹幕，含 bvid 和 cid 字段
    └── BV2xxx.ndjson
```

**rank_video.ndjson 每行字段**：

```json
{
  "bvid": "BV1xx411c7mD",
  "title": "【年度巨制】2024动画混剪",
  "description": "...",
  "category": "动画",
  "sub_category": "MAD·AMV",
  "tags": "动画,混剪,原创",
  "duration": 360,
  "publish_time": 1714560000,
  "up_id": "12345",
  "up_name": "动画区一哥",
  "up_avatar": "https://i0.hdslb.com/...",
  "url": "https://www.bilibili.com/video/BV1xx411c7mD",
  "aid": 116662200173257,
  "cid": 38717490000,
  "rank_source": "popular",
  "rank_name": "全站热门",
  "view_count": 5234000,
  "like_count": 432100,
  "coin_count": 187200,
  "favorite_count": 234500,
  "share_count": 89300,
  "reply_count": 15600,
  "danmaku_count": 12000,
  "rank_position": 1,
  "snapshot_id": "20260601_0000",
  "collect_time": 1717200000000
}
```

**验证**：

```bash
ls output/20260601_0000/
wc -l output/20260601_0000/rank_video.ndjson
head -1 output/20260601_0000/rank_video.ndjson | python3 -c "import sys,json; d=json.load(sys.stdin); assert 'aid' in d and 'cid' in d and 'snapshot_id' in d and 'category' in d"
```

### Stage 2: Upload to HDFS

**目标**：将本地 NDJSON 文件上传到 HDFS ODS 目录。

```bash
hdfs dfs -mkdir -p /data/bilibili/ods/rank_video/
hdfs dfs -mkdir -p /data/bilibili/ods/video_comment/
hdfs dfs -mkdir -p /data/bilibili/ods/video_danmaku/
hdfs dfs -mkdir -p /data/bilibili/ods/up_info/

hdfs dfs -put output/20260601_0000/rank_video.ndjson     /data/bilibili/ods/rank_video/
hdfs dfs -put output/20260601_0000/comments/*.ndjson     /data/bilibili/ods/video_comment/
hdfs dfs -put output/20260601_0000/danmaku/*.ndjson      /data/bilibili/ods/video_danmaku/
hdfs dfs -put output/20260601_0000/up_info.ndjson        /data/bilibili/ods/up_info/
```

**验证**：

```bash
hdfs dfs -ls /data/bilibili/ods/
hdfs dfs -du -h /data/bilibili/ods/
hdfs dfs -head /data/bilibili/ods/rank_video/rank_video.ndjson
```

### Stage 3: Hive External Tables (ODS)

**目标**：在 Hive 中建外部表指向 HDFS NDJSON 数据。每行一个 JSON 对象，使用 JsonSerDe。

```sql
-- ods_rank_video_raw（榜单视频快照，含互动指标 + 上下文字段）
CREATE EXTERNAL TABLE ods_rank_video_raw (
    bvid            STRING,
    title           STRING,
    description     STRING,
    category        STRING,
    sub_category    STRING,
    tags            STRING,
    duration        INT,
    publish_time    BIGINT,
    up_id           STRING,
    up_name         STRING,
    up_avatar       STRING,
    url             STRING,
    aid             BIGINT,
    cid             BIGINT,
    rank_source     STRING,
    rank_name       STRING,
    view_count      BIGINT,
    like_count      BIGINT,
    coin_count      BIGINT,
    favorite_count  BIGINT,
    share_count     BIGINT,
    reply_count     BIGINT,
    danmaku_count   BIGINT,
    rank_position   INT,
    snapshot_id     STRING,
    collect_time    BIGINT
)
ROW FORMAT SERDE 'org.apache.hive.hcatalog.data.JsonSerDe'
LOCATION '/data/bilibili/ods/rank_video/';

-- ods_video_comment_raw（评论原始数据，采集时已关联 bvid）
CREATE EXTERNAL TABLE ods_video_comment_raw (
    comment_id      BIGINT,
    bvid            STRING,
    aid             BIGINT,
    user_id         BIGINT,
    user_name       STRING,
    content         STRING,
    like_count      BIGINT,
    reply_count     BIGINT,
    comment_time    BIGINT
)
ROW FORMAT SERDE 'org.apache.hive.hcatalog.data.JsonSerDe'
LOCATION '/data/bilibili/ods/video_comment/';

-- ods_video_danmaku_raw（弹幕原始数据，采集时已关联 bvid 和 cid）
CREATE EXTERNAL TABLE ods_video_danmaku_raw (
    danmaku_id      BIGINT,
    bvid            STRING,
    cid             BIGINT,
    content         STRING,
    video_time      DOUBLE,
    mode            INT,
    fontsize        INT,
    color           BIGINT,
    mid_hash        STRING,
    send_time       BIGINT
)
ROW FORMAT SERDE 'org.apache.hive.hcatalog.data.JsonSerDe'
LOCATION '/data/bilibili/ods/video_danmaku/';

-- ods_up_raw（UP 主原始数据）
CREATE EXTERNAL TABLE ods_up_raw (
    up_id           BIGINT,
    up_name         STRING,
    up_avatar       STRING,
    follower_count  BIGINT,
    video_count     BIGINT,
    sign            STRING,
    official_verify STRING
)
ROW FORMAT SERDE 'org.apache.hive.hcatalog.data.JsonSerDe'
LOCATION '/data/bilibili/ods/up_info/';
```

**验证**：

```sql
SELECT COUNT(*) FROM ods_rank_video_raw;
SELECT * FROM ods_rank_video_raw LIMIT 5;
SELECT COUNT(*) FROM ods_video_comment_raw;
SELECT COUNT(*) FROM ods_video_danmaku_raw;
```

### Stage 4: Data Cleaning → DWD (Spark SQL)

**目标**：从 ODS 表中清洗数据，写入 DWD 管理表。

**4.1 视频快照表（按 snapshot_date 分区）**

```sql
CREATE TABLE dwd_video_snapshot (
    bvid            STRING,
    aid             BIGINT,
    view_count      BIGINT,
    like_count      BIGINT,
    coin_count      BIGINT,
    favorite_count  BIGINT,
    share_count     BIGINT,
    reply_count     BIGINT,
    danmaku_count   BIGINT,
    rank_position   INT,
    rank_source     STRING,
    rank_name       STRING,
    snapshot_id     STRING,
    collect_time    BIGINT
)
PARTITIONED BY (snapshot_date STRING)
STORED AS PARQUET;

INSERT OVERWRITE TABLE dwd_video_snapshot PARTITION (snapshot_date)
SELECT
    bvid,
    aid,
    GREATEST(COALESCE(view_count, 0), 0)     AS view_count,
    GREATEST(COALESCE(like_count, 0), 0)     AS like_count,
    GREATEST(COALESCE(coin_count, 0), 0)     AS coin_count,
    GREATEST(COALESCE(favorite_count, 0), 0) AS favorite_count,
    GREATEST(COALESCE(share_count, 0), 0)    AS share_count,
    GREATEST(COALESCE(reply_count, 0), 0)    AS reply_count,
    GREATEST(COALESCE(danmaku_count, 0), 0)  AS danmaku_count,
    rank_position,
    rank_source,
    rank_name,
    snapshot_id,
    collect_time,
    SUBSTR(snapshot_id, 1, 8) AS snapshot_date
FROM (
    SELECT *,
        ROW_NUMBER() OVER (
            PARTITION BY bvid, snapshot_id, rank_source
            ORDER BY rank_position
        ) AS rn
    FROM ods_rank_video_raw
    WHERE bvid IS NOT NULL
      AND title IS NOT NULL
      AND view_count IS NOT NULL
      AND view_count >= 0
) t
WHERE t.rn = 1;
```

**4.2 视频基础信息表（按 bvid 唯一）**

```sql
CREATE TABLE dwd_video_info (
    bvid            STRING,
    title           STRING,
    description     STRING,
    category        STRING,
    sub_category    STRING,
    tags            STRING,
    duration        INT,
    publish_time    BIGINT,
    up_id           STRING,
    up_name         STRING,
    up_avatar       STRING,
    url             STRING,
    aid             BIGINT,
    cid             BIGINT
)
STORED AS PARQUET;

INSERT OVERWRITE TABLE dwd_video_info
SELECT
    bvid,
    title,
    COALESCE(description, '') AS description,
    category,
    sub_category,
    COALESCE(tags, '') AS tags,
    duration,
    publish_time,
    up_id,
    up_name,
    up_avatar,
    url,
    aid,
    cid
FROM (
    SELECT *,
        ROW_NUMBER() OVER (
            PARTITION BY bvid
            ORDER BY CASE rank_source
                WHEN 'popular' THEN 1
                WHEN 'ranking' THEN 2
                ELSE 9
            END,
            CASE WHEN category != '其他' AND category IS NOT NULL THEN 0 ELSE 1 END,
            CASE WHEN COALESCE(description, '') != '' THEN 0 ELSE 1 END
        ) AS rn
    FROM ods_rank_video_raw
    WHERE bvid IS NOT NULL
      AND title IS NOT NULL
) t
WHERE t.rn = 1;
```

**4.3 评论明细表（采集时已带 bvid，直接清洗）**

```sql
CREATE TABLE dwd_comment_info (
    comment_id      BIGINT,
    bvid            STRING,
    aid             BIGINT,
    user_id         BIGINT,
    user_name       STRING,
    content         STRING,
    like_count      BIGINT,
    reply_count     BIGINT,
    comment_time    TIMESTAMP
)
STORED AS PARQUET;

INSERT OVERWRITE TABLE dwd_comment_info
SELECT
    comment_id,
    bvid,
    aid,
    user_id,
    user_name,
    content,
    COALESCE(like_count, 0) AS like_count,
    COALESCE(reply_count, 0) AS reply_count,
    FROM_UNIXTIME(CAST(comment_time AS BIGINT)) AS comment_time
FROM ods_video_comment_raw
WHERE content IS NOT NULL
  AND content != ''
  AND bvid IS NOT NULL;
```

**4.4 弹幕明细表（采集时已带 bvid 和 cid）**

```sql
CREATE TABLE dwd_danmaku_info (
    danmaku_id      BIGINT,
    bvid            STRING,
    cid             BIGINT,
    content         STRING,
    video_time      DOUBLE,
    mode            INT,
    fontsize        INT,
    color           BIGINT,
    mid_hash        STRING,
    send_time       TIMESTAMP
)
STORED AS PARQUET;

INSERT OVERWRITE TABLE dwd_danmaku_info
SELECT
    danmaku_id,
    bvid,
    cid,
    content,
    video_time,
    mode,
    fontsize,
    color,
    mid_hash,
    FROM_UNIXTIME(CAST(send_time AS BIGINT)) AS send_time
FROM ods_video_danmaku_raw
WHERE content IS NOT NULL
  AND content != ''
  AND bvid IS NOT NULL;
```

**4.5 UP 主信息表**

```sql
CREATE TABLE dwd_up_info (
    up_id           BIGINT,
    up_name         STRING,
    up_avatar       STRING,
    follower_count  BIGINT,
    video_count     BIGINT,
    sign            STRING,
    official_verify STRING
)
STORED AS PARQUET;

INSERT OVERWRITE TABLE dwd_up_info
SELECT
    up_id,
    up_name,
    COALESCE(up_avatar, '') AS up_avatar,
    COALESCE(follower_count, 0) AS follower_count,
    COALESCE(video_count, 0) AS video_count,
    COALESCE(sign, '') AS sign,
    COALESCE(official_verify, '') AS official_verify
FROM ods_up_raw
WHERE up_id IS NOT NULL;
```

**验证**：

```sql
SELECT snapshot_date, COUNT(*) FROM dwd_video_snapshot GROUP BY snapshot_date;
SELECT COUNT(DISTINCT bvid) FROM dwd_video_info;
SELECT * FROM dwd_video_snapshot WHERE view_count < 0;
```

### Stage 5: Analysis → DWS / ADS (Spark SQL)

**目标**：按分析模块写出统计 SQL，结果写入 DWS/ADS 表。

**5.1 分区每日汇总 (dws_partition_daily_stats)**

```sql
CREATE TABLE dws_partition_daily_stats AS
WITH canonical AS (
    SELECT *
    FROM (
        SELECT *,
            ROW_NUMBER() OVER (
                PARTITION BY bvid, snapshot_id
                ORDER BY CASE rank_source
                    WHEN 'popular' THEN 1
                    WHEN 'ranking' THEN 2
                    ELSE 9
                END
            ) AS rn
        FROM dwd_video_snapshot
    ) t
    WHERE rn = 1
),
latest_per_day AS (
    SELECT c.*
    FROM canonical c
    JOIN (
        SELECT SUBSTR(snapshot_id, 1, 8) AS sd, MAX(snapshot_id) AS max_sid
        FROM canonical
        GROUP BY SUBSTR(snapshot_id, 1, 8)
    ) m ON c.snapshot_id = m.max_sid
)
SELECT
    v.category,
    s.snapshot_date,
    COUNT(DISTINCT s.bvid) AS video_count,
    SUM(s.view_count) AS total_views,
    SUM(s.like_count + s.coin_count + s.favorite_count
        + s.reply_count + s.share_count) AS total_interactions,
    COUNT(DISTINCT v.up_id) AS up_count
FROM latest_per_day s
JOIN dwd_video_info v ON s.bvid = v.bvid
GROUP BY v.category, s.snapshot_date;
```

**5.2 视频生命周期 (dws_video_lifecycle_stats)**

```sql
CREATE TABLE dws_video_lifecycle_stats AS
WITH latest_collect AS (
    SELECT MAX(snapshot_id) AS max_sid FROM dwd_video_snapshot
),
top_videos AS (
    SELECT s.bvid
    FROM dwd_video_snapshot s
    JOIN latest_collect lc ON s.snapshot_id = lc.max_sid
    WHERE s.rank_source = 'popular'
    ORDER BY s.view_count DESC
    LIMIT 20
)
SELECT
    s.bvid,
    v.title,
    v.category,
    s.snapshot_date,
    s.snapshot_id,
    s.collect_time,
    s.rank_source,
    s.rank_name,
    SUBSTR(s.snapshot_id, 1, 8)  AS snapshot_date_fmt,
    CONCAT(SUBSTR(s.snapshot_id, 10, 2), ':', SUBSTR(s.snapshot_id, 12, 2)) AS snapshot_time,
    s.view_count,
    s.like_count,
    s.rank_position,
    s.view_count - LAG(s.view_count) OVER (
        PARTITION BY s.bvid, s.rank_source ORDER BY s.snapshot_id
    ) AS view_growth,
    LAG(s.rank_position) OVER (
        PARTITION BY s.bvid, s.rank_source ORDER BY s.snapshot_id
    ) - s.rank_position AS rank_change
FROM dwd_video_snapshot s
JOIN dwd_video_info v ON s.bvid = v.bvid
WHERE s.bvid IN (SELECT bvid FROM top_videos)
  AND s.rank_source = 'popular';
```

> `rank_change` 正数 = 排名上升（数值变小），负数 = 排名下降。第一个点 `view_growth = 0`，`rank_change = 0`。Day 1 仅有 1 个快照点，需 ≥2 天数据才有折线。TOP 视频按最新快照的 `view_count` 降序选取，默认使用 `rank_source='popular'` 以保证排名可比较。`/api/lifecycle` 和 `/api/lifecycle/top` 只覆盖 popular 样本；若 bvid 不在 popular 生命周期中，返回 `data=null`。

**5.3 UP 主汇总 (dws_up_summary_stats)**

```sql
CREATE TABLE dws_up_summary_stats AS
WITH canonical AS (
    SELECT *
    FROM (
        SELECT *,
            ROW_NUMBER() OVER (
                PARTITION BY bvid, snapshot_id
                ORDER BY CASE rank_source
                    WHEN 'popular' THEN 1
                    WHEN 'ranking' THEN 2
                    ELSE 9
                END
            ) AS rn
        FROM dwd_video_snapshot
        WHERE snapshot_id = (SELECT MAX(snapshot_id) FROM dwd_video_snapshot)
    ) t
    WHERE rn = 1
)
SELECT
    u.up_id,
    u.up_name,
    u.up_avatar,
    u.follower_count,
    u.video_count AS published_video_count,
    COUNT(DISTINCT v.bvid) AS hot_video_count,
    SUM(s.view_count) AS total_views
FROM dwd_up_info u
LEFT JOIN dwd_video_info v ON CAST(u.up_id AS STRING) = v.up_id
LEFT JOIN canonical s ON v.bvid = s.bvid
GROUP BY u.up_id, u.up_name, u.up_avatar, u.follower_count, u.video_count;
```

**5.4 大屏总览 (ads_screen_overview)**

```sql
CREATE TABLE ads_screen_overview AS
WITH canonical AS (
    SELECT *
    FROM (
        SELECT *,
            ROW_NUMBER() OVER (
                PARTITION BY bvid, snapshot_id
                ORDER BY CASE rank_source
                    WHEN 'popular' THEN 1
                    WHEN 'ranking' THEN 2
                    ELSE 9
                END
            ) AS rn
        FROM dwd_video_snapshot
        WHERE snapshot_id = (SELECT MAX(snapshot_id) FROM dwd_video_snapshot)
    ) t
    WHERE rn = 1
)
SELECT
    COUNT(DISTINCT s.bvid)                                    AS video_count,
    COUNT(DISTINCT v.category)                                AS category_count,
    COUNT(DISTINCT v.up_id)                                   AS up_count,
    SUM(s.view_count)                                         AS total_views,
    SUM(s.like_count + s.coin_count + s.favorite_count
        + s.share_count + s.reply_count)                      AS total_interactions,
    MAX(s.snapshot_date)                                      AS latest_snapshot_date
FROM canonical s
JOIN dwd_video_info v ON s.bvid = v.bvid;
```

**5.5 热门视频排行 (ads_hot_video_rank)**

```sql
CREATE TABLE ads_hot_video_rank AS
WITH latest AS (
    SELECT *
    FROM (
        SELECT *,
            ROW_NUMBER() OVER (
                PARTITION BY bvid, snapshot_id
                ORDER BY CASE rank_source
                    WHEN 'popular' THEN 1
                    WHEN 'ranking' THEN 2
                    ELSE 9
                END
            ) AS rn
        FROM dwd_video_snapshot
        WHERE snapshot_id = (SELECT MAX(snapshot_id) FROM dwd_video_snapshot)
    ) t
    WHERE rn = 1
),
norm AS (
    SELECT
        LN(MAX(view_count) + 1)     AS log_view_max,
        LN(MIN(view_count) + 1)     AS log_view_min,
        LN(MAX(like_count) + 1)     AS log_like_max,
        LN(MIN(like_count) + 1)     AS log_like_min,
        LN(MAX(coin_count) + 1)     AS log_coin_max,
        LN(MIN(coin_count) + 1)     AS log_coin_min,
        LN(MAX(favorite_count) + 1) AS log_fav_max,
        LN(MIN(favorite_count) + 1) AS log_fav_min,
        LN(MAX(reply_count) + 1)    AS log_reply_max,
        LN(MIN(reply_count) + 1)    AS log_reply_min,
        LN(MAX(share_count) + 1)    AS log_share_max,
        LN(MIN(share_count) + 1)    AS log_share_min
    FROM latest
)
SELECT
    s.bvid,
    v.title,
    v.category,
    v.up_name,
    s.view_count,
    s.like_count,
    s.coin_count,
    s.favorite_count,
    s.reply_count,
    s.share_count,
    s.rank_position,
    (
        COALESCE((LN(s.view_count + 1) - n.log_view_min) / NULLIF(n.log_view_max - n.log_view_min, 0), 0) * 0.4
      + COALESCE((LN(s.like_count + 1) - n.log_like_min) / NULLIF(n.log_like_max - n.log_like_min, 0), 0) * 0.2
      + COALESCE((LN(s.coin_count + 1) - n.log_coin_min) / NULLIF(n.log_coin_max - n.log_coin_min, 0), 0) * 0.15
      + COALESCE((LN(s.favorite_count + 1) - n.log_fav_min) / NULLIF(n.log_fav_max - n.log_fav_min, 0), 0) * 0.15
      + COALESCE((LN(s.reply_count + 1) - n.log_reply_min) / NULLIF(n.log_reply_max - n.log_reply_min, 0), 0) * 0.05
      + COALESCE((LN(s.share_count + 1) - n.log_share_min) / NULLIF(n.log_share_max - n.log_share_min, 0), 0) * 0.05
    ) AS heat_score
FROM latest s
JOIN dwd_video_info v ON s.bvid = v.bvid
CROSS JOIN norm n
ORDER BY heat_score DESC
LIMIT 100;
```

**5.6 分区竞争力 (ads_partition_compare)**

```sql
CREATE TABLE ads_partition_compare AS
WITH canonical AS (
    SELECT *
    FROM (
        SELECT *,
            ROW_NUMBER() OVER (
                PARTITION BY bvid, snapshot_id
                ORDER BY CASE rank_source
                    WHEN 'popular' THEN 1
                    WHEN 'ranking' THEN 2
                    ELSE 9
                END
            ) AS rn
        FROM dwd_video_snapshot
        WHERE snapshot_id = (SELECT MAX(snapshot_id) FROM dwd_video_snapshot)
    ) t
    WHERE rn = 1
)
SELECT
    v.category,
    COUNT(DISTINCT s.bvid)                                            AS hot_video_count,
    ROUND(AVG(s.view_count), 0)                                       AS avg_views,
    ROUND(AVG(CAST(s.like_count AS DOUBLE) / NULLIF(s.view_count, 0)), 4)      AS avg_like_rate,
    ROUND(AVG(CAST(s.coin_count AS DOUBLE) / NULLIF(s.view_count, 0)), 4)      AS avg_coin_rate,
    ROUND(AVG(CAST(s.favorite_count AS DOUBLE) / NULLIF(s.view_count, 0)), 4)  AS avg_fav_rate,
    MAX(s.view_count)                                                  AS top_view_count,
    SUM(s.view_count)                                                  AS total_views,
    COUNT(DISTINCT v.up_id)                                            AS up_count
FROM canonical s
JOIN dwd_video_info v ON s.bvid = v.bvid
GROUP BY v.category;
```

**5.7 统一词云 (ads_word_cloud)**

```sql
CREATE TABLE ads_word_cloud (
    source_type  STRING,    -- title/comment/danmaku
    category     STRING,    -- 全站/游戏/动画/...
    word         STRING,
    word_count   INT
)
STORED AS PARQUET;
```

> 标题词云由 `scripts/run_tag_cloud_mr.sh`（参数化，支持 title/comment/danmaku）通过 Hadoop Streaming + Python3/jieba 生成。
> 评论词云和弹幕词云分别从 `dwd_comment_info` 和 `dwd_danmaku_info` 取数据。
> 结果写入 `ads_word_cloud`，`source_type` 区分三种词云。

**5.8 热门评论 (ads_hot_comments)**

```sql
CREATE TABLE ads_hot_comments (
    bvid         STRING,
    title        STRING,
    category     STRING,
    up_name      STRING,
    user_name    STRING,
    content      STRING,
    like_count   BIGINT,
    reply_count  BIGINT,
    comment_time TIMESTAMP
)
STORED AS PARQUET;
```

> 从 `dwd_comment_info` JOIN `dwd_video_info` 取高赞评论，按 `like_count DESC` 排序，LIMIT 5000。

**5.8 视频生命周期 ADS (ads_video_lifecycle)**

```sql
CREATE TABLE ads_video_lifecycle AS
SELECT
    bvid,
    title,
    category,
    snapshot_date,
    CONCAT(SUBSTR(snapshot_id, 10, 2), ':', SUBSTR(snapshot_id, 12, 2)) AS snapshot_time,
    snapshot_id,
    collect_time,
    rank_source,
    rank_name,
    view_count,
    like_count,
    rank_position,
    COALESCE(view_growth, 0) AS view_growth,
    COALESCE(rank_change, 0) AS rank_change
FROM dws_video_lifecycle_stats;
```

**5.9 分区每日 ADS (ads_partition_daily)**

```sql
CREATE TABLE ads_partition_daily AS
SELECT * FROM dws_partition_daily_stats;
```

**5.10 UP 主汇总 ADS (ads_up_summary)**

```sql
CREATE TABLE ads_up_summary AS
SELECT * FROM dws_up_summary_stats;
```

**MapReduce 任务：标题词频统计**

```text
mapreduce/
├── mapper.py       # Python3/jieba 标题分词，输出 category\tword\t1
├── reducer.py      # 聚合 category + word 词频
└── stopwords.txt   # 停用词
```

运行脚本：

```bash
bash scripts/run_tag_cloud_mr.sh
```

HDFS 输入输出：

```text
/data/bilibili/mr_input/title_words   # category\ttitle
/data/bilibili/mr_output/title_words  # category\ttag\ttag_count
```

结果写回 Hive `ads_word_cloud`，其中 `category='全站'` 表示全局词云，其余为分区词云。

**验证**：

```sql
SELECT * FROM ads_screen_overview;
SELECT * FROM ads_hot_video_rank ORDER BY heat_score DESC LIMIT 5;
SELECT * FROM ads_partition_compare ORDER BY hot_video_count DESC;
SELECT * FROM ads_word_cloud WHERE source_type = 'title' ORDER BY word_count DESC LIMIT 20;
SELECT bvid, COUNT(*) FROM ads_video_lifecycle GROUP BY bvid;
```

### Stage 6: Export to MySQL

**目标**：将 ADS 结果同步到 MySQL。使用 PySpark JDBC。

```bash
# 自动管线（部署在 WSL，crawler 完成后触发）
bash scripts/run_pipeline.sh

# 或手动执行各步骤:
# Step 1: 上传 NDJSON 到 HDFS
bash scripts/upload_hdfs.sh

# Step 2: Spark SQL ETL（在 master 上运行）
ssh hadoop@master spark-sql --master spark://master:7077 -f /home/hadoop/sql/spark_etl.sql

# Step 3: 标题词云 MapReduce（在 WSL 脚本中远程触发 master）
bash scripts/run_tag_cloud_mr.sh

# Step 4: Spark JDBC 导出到 MySQL
ssh hadoop@master spark-submit --jars /opt/spark/jars/mysql-connector-j-8.0.33.jar \
    --master spark://master:7077 \
    /home/hadoop/scripts/export_to_mysql.py

# 首次初始化（只需一次）
bash scripts/setup_pipeline.sh
```

**MySQL 建表**（`sql/mysql_ads_ddl.sql`，与 API 响应字段一一对应）：

```sql
CREATE DATABASE IF NOT EXISTS bilibili DEFAULT CHARSET utf8mb4;
USE bilibili;

-- 大屏总览（单行表）
CREATE TABLE ads_screen_overview (
    video_count          INT,
    category_count       INT,
    up_count             INT,
    total_views          BIGINT,
    total_interactions   BIGINT,
    latest_snapshot_date VARCHAR(8),
    update_time          DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 热门视频排行
CREATE TABLE ads_hot_video_rank (
    bvid           VARCHAR(32) PRIMARY KEY,
    title          VARCHAR(512),
    category       VARCHAR(64),
    up_name        VARCHAR(128),
    view_count     BIGINT,
    like_count     BIGINT,
    coin_count     BIGINT,
    favorite_count BIGINT,
    reply_count    BIGINT,
    share_count    BIGINT,
    rank_position  INT,
    heat_score     DOUBLE,
    update_time    DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 分区竞争力
CREATE TABLE ads_partition_compare (
    category        VARCHAR(64) PRIMARY KEY,
    hot_video_count INT,
    avg_views       BIGINT,
    avg_like_rate   DOUBLE,
    avg_coin_rate   DOUBLE,
    avg_fav_rate    DOUBLE,
    top_view_count  BIGINT,
    total_views     BIGINT,
    up_count        INT,
    update_time     DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 视频生命周期
CREATE TABLE ads_video_lifecycle (
    id             INT AUTO_INCREMENT PRIMARY KEY,
    bvid           VARCHAR(32),
    title          VARCHAR(512),
    category       VARCHAR(64),
    snapshot_date  VARCHAR(8),
    snapshot_time  VARCHAR(5),
    snapshot_id    VARCHAR(16),
    collect_time   BIGINT,
    rank_source    VARCHAR(16),
    rank_name      VARCHAR(32),
    view_count     BIGINT,
    like_count     BIGINT,
    rank_position  INT,
    view_growth    BIGINT,
    rank_change    INT,
    update_time    DATETIME DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_bvid (bvid),
    INDEX idx_date (snapshot_date),
    INDEX idx_snapshot (bvid, snapshot_id)
);

-- 分区每日汇总
CREATE TABLE ads_partition_daily (
    id                 INT AUTO_INCREMENT PRIMARY KEY,
    category           VARCHAR(64),
    snapshot_date      VARCHAR(8),
    video_count        INT,
    total_views        BIGINT,
    total_interactions BIGINT,
    up_count           INT,
    update_time        DATETIME DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_category_date (category, snapshot_date)
);

-- UP主汇总
CREATE TABLE ads_up_summary (
    up_id               VARCHAR(32) PRIMARY KEY,
    up_name             VARCHAR(128),
    up_avatar           VARCHAR(512),
    follower_count      BIGINT,
    published_video_count BIGINT,
    hot_video_count     BIGINT,
    total_views         BIGINT,
    update_time         DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 标签词云
CREATE TABLE ads_word_cloud (
    id          INT AUTO_INCREMENT PRIMARY KEY,
    tag         VARCHAR(64),
    tag_count   INT,
    category    VARCHAR(64),
    update_time DATETIME DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_tag (tag),
    INDEX idx_category (category)
);
```

**验证**：

```bash
mysql -u root bilibili -e "SELECT * FROM ads_screen_overview;"
mysql -u root bilibili -e "SELECT COUNT(*) FROM ads_hot_video_rank;"
mysql -u root bilibili -e "SELECT COUNT(*) FROM ads_video_lifecycle;"
mysql -u root bilibili -e "SELECT * FROM ads_word_cloud WHERE source_type = 'title' ORDER BY word_count DESC LIMIT 10;"
```

## 5. Backend API (Rust Axum)

### 5.1 Endpoints

| 方法 | 路径 | 查询参数 | 返回 | MySQL 表 |
|---|---|---|---|---|
| GET | `/api/overview` | — | 大屏总览指标 | ads_screen_overview |
| GET | `/api/hot-videos` | `?sort=heat&limit=20&offset=0` | 热门视频排行（分页） | ads_hot_video_rank |
| GET | `/api/partition-compare` | — | 分区竞争力数据 | ads_partition_compare |
| GET | `/api/lifecycle` | `?bvid=BVxxx` | 单视频生命周期 | ads_video_lifecycle |
| GET | `/api/lifecycle/top` | `?limit=5` | TOP N 视频生命周期 | ads_video_lifecycle |
| GET | `/api/partition-detail` | `?category=游戏` | 分区详情（视频+UP主） | 多表 JOIN |
| GET | `/api/word-cloud` | `?source=title&category=游戏` | 词云（标题/评论/弹幕） | ads_word_cloud |
| GET | `/api/hot-comments` | `?category=游戏&limit=20` | 热门评论排行 | ads_hot_comments |
| GET | `/api/interaction` | `?bvid=BVxxx` | 互动质量数据 | ads_hot_video_rank |

Base URL: `http://localhost:9090`，所有响应统一包裹为 `{ "code": 0, "message": "ok", "data": {...} }`。

**错误码约定**：

| code | 含义 | 场景 |
|---|---|---|
| 0 | 成功 | 请求成功（资源不存在时 `data=null`） |
| 400 | 参数错误 | 缺少必填参数、参数格式错误 |
| 500 | 服务器内部错误 | 数据库异常等 |

### 5.2 项目结构

```
backend/
├── Cargo.toml
├── src/
│   ├── main.rs          # Axum 启动 + 路由 + CORS
│   ├── db.rs            # MySQL 连接池 (sqlx)
│   ├── models.rs        # 数据结构 (serde Serialize)
│   ├── handlers.rs      # 各端点处理函数
│   └── error.rs         # 统一错误处理
```

**Cargo.toml 关键依赖**：

```toml
[dependencies]
axum = "0.7"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sqlx = { version = "0.7", features = ["runtime-tokio", "mysql"] }
tower-http = { version = "0.5", features = ["cors"] }
```

### 5.3 API Contract

**通用约定**：

- Base URL: `http://localhost:9090`
- 所有响应均为 JSON，包裹在统一结构中：

```json
{
  "code": 0,
  "message": "ok",
  "data": { ... }
}
```

- `code=0` 表示成功，非 0 为错误
- 时间字段格式：`"2026-06-01 12:00:00"` (yyyy-MM-dd HH:mm:ss)
- 缺少必填参数：`code=400`
- 资源不存在：`code=0`，`data=null`

---

#### GET `/api/overview`

**请求**：无参数

**响应 `data`**：

```json
{
  "video_count": 286,
  "category_count": 10,
  "up_count": 203,
  "total_views": 452800310,
  "total_interactions": 18720045,
  "latest_snapshot_date": "20260607",
  "update_time": "2026-06-07 18:00:00"
}
```

| 字段 | 类型 | 说明 |
|---|---|---|
| video_count | int | 最新快照中的视频数 |
| category_count | int | 涉及分区数（通常 ≤9，若存在"其他"分区则可能为 10） |
| up_count | int | 涉及 UP 主数 |
| total_views | int | 样本累计播放量 |
| total_interactions | int | 点赞+投币+收藏+评论+分享总和 |
| latest_snapshot_date | string | 最新快照日期 yyyyMMdd |
| update_time | string | 数据更新时间 |

**数据库为空时**：所有计数字段为 0，`latest_snapshot_date` 为 `null`。

---

#### GET `/api/hot-videos`

**请求**：

| 参数 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| sort | string | heat | heat / views / likes / coins |
| limit | int | 20 | 返回条数 |
| offset | int | 0 | 偏移量 |

**响应 `data`**：

```json
{
  "total": 100,
  "videos": [
    {
      "bvid": "BV1xx411c7mD",
      "title": "【年度巨制】2024动画混剪",
      "category": "动画",
      "up_name": "动画区一哥",
      "view_count": 5234000,
      "like_count": 432100,
      "coin_count": 187200,
      "favorite_count": 234500,
      "reply_count": 15600,
      "share_count": 89300,
      "rank_position": 1,
      "heat_score": 0.9543
    }
  ]
}
```

**videos[].** 字段：

| 字段 | 类型 | 说明 |
|---|---|---|
| bvid | string | BV 号 |
| title | string | 标题 |
| category | string | 分区 |
| up_name | string | UP 主名 |
| view_count | int | 播放量 |
| like_count | int | 点赞数 |
| coin_count | int | 投币数 |
| favorite_count | int | 收藏数 |
| reply_count | int | 评论数 |
| share_count | int | 分享数 |
| rank_position | int | 排名 |
| heat_score | double | 综合热度 (0~1) |

**非法参数处理**：
- `sort` 不在合法值列表中 → `code=400`
- `limit=0` → 返回空数组 `videos: []`
- `limit` 缺失 → 使用默认值 20

---

#### GET `/api/partition-compare`

**请求**：无参数（返回最新快照的全部分区）

**响应 `data`**：

```json
{
  "partitions": [
    {
      "category": "游戏",
      "hot_video_count": 52,
      "avg_views": 2310000,
      "avg_like_rate": 0.0842,
      "avg_coin_rate": 0.0315,
      "avg_fav_rate": 0.0451,
      "top_view_count": 8234000,
      "total_views": 120120000,
      "up_count": 38
    }
  ]
}
```

| 字段 | 类型 | 说明 |
|---|---|---|
| category | string | 分区名 |
| hot_video_count | int | 上榜视频数 |
| avg_views | int | 平均播放量 |
| avg_like_rate | double | 平均点赞率 |
| avg_coin_rate | double | 平均投币率 |
| avg_fav_rate | double | 平均收藏率 |
| top_view_count | int | 该分区最高播放量 |
| total_views | int | 该分区总播放量 |
| up_count | int | 该分区 UP 主数 |

---

#### GET `/api/lifecycle`

**请求**：

| 参数 | 类型 | 必填 | 说明 |
|---|---|---|---|
| bvid | string | 是 | 视频 BV 号 |

**响应 `data`**：

```json
{
  "bvid": "BV1xx411c7mD",
  "title": "【年度巨制】2024动画混剪",
  "category": "动画",
  "timeline": [
    {
      "snapshot_date": "20260601",
      "snapshot_time": "00:00",
      "view_count": 1023400,
      "like_count": 87200,
      "rank_position": 15,
      "rank_source": "popular",
      "rank_name": "全站热门",
      "view_growth": 0,
      "rank_change": 0
    },
    {
      "snapshot_date": "20260601",
      "snapshot_time": "06:00",
      "view_count": 1234500,
      "like_count": 93400,
      "rank_position": 8,
      "rank_source": "popular",
      "rank_name": "全站热门",
      "view_growth": 211100,
      "rank_change": 7
    }
  ]
}
```

| 字段 | 类型 | 说明 |
|---|---|---|
| bvid | string | BV 号 |
| title | string | 标题 |
| category | string | 分区 |
| timeline | array | 时间序列（按 snapshot_id 升序） |

**timeline[].** 字段：

| 字段 | 类型 | 说明 |
|---|---|---|
| snapshot_date | string | 日期 yyyyMMdd |
| snapshot_time | string | 时间 HH:mm（同一日多次快照时区分） |
| view_count | int | 该时间点播放量 |
| like_count | int | 该时间点点赞数 |
| rank_position | int | 排名 |
| rank_source | string | 来源榜单：popular/ranking |
| rank_name | string | 来源榜单名 |
| view_growth | int | 较上一时间点播放增量（首个点为 0） |
| rank_change | int | 较上一时间点排名变化（正=上升，首个点为 0） |

**bvid 不存在时**：`code=0`，`data=null`。

---

#### GET `/api/lifecycle/top`

**请求**：

| 参数 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| limit | int | 5 | TOP N 个视频 |

**响应 `data`**：

```json
{
  "videos": [
    {
      "bvid": "BV1xx411c7mD",
      "title": "【年度巨制】2024动画混剪",
      "category": "动画",
      "timeline": [ ... ]
    }
  ]
}
```

> `timeline` 字段结构同 `/api/lifecycle`。若实际数量不足 limit，返回全部（不报错）。

---

#### GET `/api/partition-detail`

**请求**：

| 参数 | 类型 | 必填 | 说明 |
|---|---|---|---|
| category | string | 是 | 分区名，如"游戏" |

**响应 `data`**：

```json
{
  "category": "游戏",
  "hot_video_count": 52,
  "avg_views": 2310000,
  "videos": [
    {
      "bvid": "BV1xx411c7mD",
      "title": "【年度巨制】2024动画混剪",
      "view_count": 5234000,
      "heat_score": 0.9543,
      "rank_position": 1
    }
  ],
  "top_ups": [
    {
      "up_id": "123456",
      "up_name": "游戏区一哥",
      "follower_count": 5200000,
      "hot_video_count": 5,
      "total_views": 28000000
    }
  ]
}
```

**缺少 category 参数**：`code=400`。

**分区不存在**：`code=0`，`data.videos=[]`，`data.top_ups=[]`。

---

#### GET `/api/word-cloud`

**请求**：

| 参数 | 类型 | 必填 | 说明 |
|---|---|---|---|
| source | string | 否 | 数据来源：title/comment/danmaku，默认 title |
| category | string | 否 | 分区筛选，不传则返回全站 |

**响应 `data`**：

```json
{
  "source": "comment",
  "category": "游戏",
  "words": [
    { "name": "剧情", "value": 42 },
    { "name": "操作", "value": 37 }
  ]
}
```

| 字段 | 类型 | 说明 |
|---|---|---|
| source | string | 数据来源类型 |
| category | string | 分区名 |
| words | array | 词列表 |
| words[].name | string | 词 |
| words[].value | int | 出现次数（>0） |

---

#### GET `/api/interaction`

**请求**：

| 参数 | 类型 | 必填 | 说明 |
|---|---|---|---|
| bvid | string | 是 | 视频 BV 号 |

**响应 `data`**：

```json
{
  "bvid": "BV1xx411c7mD",
  "view_count": 5234000,
  "like_count": 432100,
  "coin_count": 187200,
  "favorite_count": 234500,
  "reply_count": 15600,
  "share_count": 89300,
  "like_rate": 0.0826,
  "coin_rate": 0.0358,
  "fav_rate": 0.0448,
  "reply_rate": 0.0030,
  "share_rate": 0.0171
}
```

**bvid 不存在时**：`code=0`，`data=null`。

### 5.4 验证

```bash
cargo run
curl http://localhost:9090/api/overview
curl "http://localhost:9090/api/hot-videos?sort=heat&limit=10"
```

### 5.5 API Test Cases

> 每条用例含 4 部分：**Given**（前置条件）、**When**（请求）、**Then**（期望响应）。

#### TC-01: `/api/overview` — 正常获取总览

| 项 | 内容 |
|---|---|
| Given | MySQL `ads_screen_overview` 表有数据 |
| When | `GET /api/overview` |
| Then | `code=0`，`data` 包含 `video_count`/`category_count`/`up_count`/`total_views`/`total_interactions`/`latest_snapshot_date`/`update_time`，值均 ≥0 |

#### TC-02: `/api/overview` — 数据库为空

| 项 | 内容 |
|---|---|
| Given | MySQL `ads_screen_overview` 表无数据 |
| When | `GET /api/overview` |
| Then | `code=0`，`data` 中所有计数字段为 0，`latest_snapshot_date` 为 `null` |

#### TC-03: `/api/hot-videos` — 默认排序分页

| 项 | 内容 |
|---|---|
| Given | `ads_hot_video_rank` 表有 100 条数据 |
| When | `GET /api/hot-videos`（无参数） |
| Then | `code=0`，`data.total=100`，`data.videos` 长度为 20（默认 limit），按 `heat_score` 降序 |

#### TC-04: `/api/hot-videos` — 按播放量排序 + 分页

| 项 | 内容 |
|---|---|
| Given | 同上 |
| When | `GET /api/hot-videos?sort=views&limit=5&offset=10` |
| Then | `code=0`，`data.total=100`，`data.videos` 长度为 5，按 `view_count` 降序，从第 11 条开始 |

#### TC-05: `/api/hot-videos` — 非法排序参数

| 项 | 内容 |
|---|---|
| Given | — |
| When | `GET /api/hot-videos?sort=invalid` |
| Then | `code=400`，`message` 包含参数错误说明 |

#### TC-06: `/api/hot-videos` — limit 为 0

| 项 | 内容 |
|---|---|
| Given | — |
| When | `GET /api/hot-videos?limit=0` |
| Then | `code=0`，`data.videos` 为空数组，`data.total` 为总数 |

#### TC-07: `/api/partition-compare` — 正常获取

| 项 | 内容 |
|---|---|
| Given | `ads_partition_compare` 表有数据 |
| When | `GET /api/partition-compare` |
| Then | `code=0`，`data.partitions` 非空，每条含 `category`/`hot_video_count`/`avg_views`/`avg_like_rate`/`avg_coin_rate`/`avg_fav_rate`/`top_view_count`/`total_views`/`up_count`；各率值 ∈ [0, 1] |

#### TC-08: `/api/partition-compare` — 数据为空

| 项 | 内容 |
|---|---|
| Given | `ads_partition_compare` 表无数据 |
| When | `GET /api/partition-compare` |
| Then | `code=0`，`data.partitions` 为空数组 |

#### TC-09: `/api/lifecycle` — 正常获取

| 项 | 内容 |
|---|---|
| Given | `ads_video_lifecycle` 表有对应 bvid 的 28 个时间点数据 |
| When | `GET /api/lifecycle?bvid=BVxxx`（使用数据库中真实存在的 bvid） |
| Then | `code=0`，`data.bvid="BVxxx"`，`data.timeline` 长度为 28，每个点含 `snapshot_date`/`snapshot_time`/`view_count`/`like_count`/`rank_position`/`rank_source`/`rank_name`/`view_growth`/`rank_change`；第一个点 `view_growth=0` 且 `rank_change=0` |

#### TC-10: `/api/lifecycle` — 缺少 bvid 参数

| 项 | 内容 |
|---|---|
| Given | — |
| When | `GET /api/lifecycle`（无参数） |
| Then | `code=400`，`message` 包含 "bvid" 或 "required" |

#### TC-11: `/api/lifecycle` — bvid 不存在

| 项 | 内容 |
|---|---|
| Given | 数据库中无此 bvid |
| When | `GET /api/lifecycle?bvid=NOTEXIST` |
| Then | `code=0`，`data=null` |

#### TC-12: `/api/lifecycle/top` — 默认 limit=5

| 项 | 内容 |
|---|---|
| Given | `ads_video_lifecycle` 表有 ≥10 个不同 bvid 的数据 |
| When | `GET /api/lifecycle/top`（无参数） |
| Then | `code=0`，`data.videos` 长度为 5，每个元素含 `bvid`/`title`/`category`/`timeline` |

#### TC-13: `/api/lifecycle/top` — limit 超出实际数量

| 项 | 内容 |
|---|---|
| Given | 只有 3 个视频有生命周期数据 |
| When | `GET /api/lifecycle/top?limit=10` |
| Then | `code=0`，`data.videos` 长度为 3（返回全部，不报错） |

#### TC-14: `/api/partition-detail` — 正常获取

| 项 | 内容 |
|---|---|
| Given | 数据库有某分区的视频和 UP 主数据 |
| When | `GET /api/partition-detail?category=游戏`（使用数据库中真实存在的分区名） |
| Then | `code=0`，`data.category="游戏"`，`data.videos` 非空，`data.top_ups` 非空 |

#### TC-15: `/api/partition-detail` — 分区不存在

| 项 | 内容 |
|---|---|
| Given | 数据库无"不存在分区" |
| When | `GET /api/partition-detail?category=不存在分区` |
| Then | `code=0`，`data.videos=[]`，`data.top_ups=[]`，`data.category="不存在分区"` |

#### TC-16: `/api/partition-detail` — 缺少 category 参数

| 项 | 内容 |
|---|---|
| Given | — |
| When | `GET /api/partition-detail`（无参数） |
| Then | `code=400`，`message` 包含 "category" 或 "required" |

#### TC-17: `/api/tag-cloud` — 正常获取全站

| 项 | 内容 |
|---|---|
| Given | `ads_word_cloud` 表有数据 |
| When | `GET /api/tag-cloud`（无参数） |
| Then | `code=0`，`data.tags` 为数组，每条含 `name`(string) 和 `value`(int>0)，按 `value` 降序 |

#### TC-18: `/api/tag-cloud` — 按分区筛选

| 项 | 内容 |
|---|---|
| Given | `ads_word_cloud` 表有"游戏"分区的标签 |
| When | `GET /api/tag-cloud?category=游戏` |
| Then | `code=0`，`data.tags` 只包含 `category="游戏"` 的标签 |

#### TC-19: `/api/tag-cloud` — 数据为空

| 项 | 内容 |
|---|---|
| Given | `ads_word_cloud` 表无数据 |
| When | `GET /api/tag-cloud` |
| Then | `code=0`，`data.tags=[]` |

#### TC-20: `/api/interaction` — 正常获取

| 项 | 内容 |
|---|---|
| Given | `ads_hot_video_rank` 表中有对应 bvid |
| When | `GET /api/interaction?bvid=BVxxx`（使用数据库中真实存在的 bvid） |
| Then | `code=0`，`data.bvid="BVxxx"`；含 `like_rate`/`coin_rate`/`fav_rate`/`reply_rate`/`share_rate`，各率值 ∈ [0, 1] |

#### TC-21: `/api/interaction` — bvid 不存在

| 项 | 内容 |
|---|---|
| Given | 数据库中无此 bvid |
| When | `GET /api/interaction?bvid=NOTEXIST` |
| Then | `code=0`，`data=null` |

#### TC-22: 通用 — 响应格式校验（全部端点）

| 项 | 内容 |
|---|---|
| Given | 任一端点返回成功 |
| When | 检查 HTTP 状态码和响应 JSON |
| Then | HTTP 200；Content-Type 为 `application/json`；JSON 根节点含 `code`/`message`/`data` 三个字段；`code=0`；`message` 为非空字符串 |

#### TC-23: 通用 — CORS 校验

| 项 | 内容 |
|---|---|
| Given | 后端启动 |
| When | 用 `curl -H "Origin: http://localhost:5173" -I $API_URL` 请求任一端点 |
| Then | 响应头含 `Access-Control-Allow-Origin` |

### 5.6 测试执行脚本

```bash
#!/bin/bash
# test_api.sh — 运行后可快速核验 8 个端点
# 使用方法: bash scripts/test_api.sh [BASE_URL]

BASE="${1:-http://localhost:9090}"
PASS=0
FAIL=0

run_test() {
    local name="$1"
    local cmd="$2"
    echo "=== $name ==="
    if eval "$cmd"; then
        PASS=$((PASS + 1))
    else
        FAIL=$((FAIL + 1))
        echo "FAIL"
    fi
}

# 先从 lifecycle/top 获取一个真实 bvid（保证在 lifecycle 表中有数据）
BV=$(curl -s "$BASE/api/lifecycle/top?limit=1" | python3 -c "import sys,json; d=json.load(sys.stdin); v=d['data']['videos']; print(v[0]['bvid'] if v else '')" 2>/dev/null)
if [ -z "$BV" ]; then
    # fallback: 从 hot-videos 获取（可能 lifecycle 中无数据，此时 lifecycle/interaction 测试会跳过）
    BV=$(curl -s "$BASE/api/hot-videos?limit=1" | python3 -c "import sys,json; d=json.load(sys.stdin); v=d['data']['videos']; print(v[0]['bvid'] if v else '')" 2>/dev/null)
    if [ -n "$BV" ]; then
        echo "WARNING: lifecycle/top 无数据，使用 hot-videos bvid=$BV（lifecycle/interaction 测试可能跳过）"
    fi
fi
if [ -z "$BV" ]; then
    echo "WARNING: 无数据，跳过 lifecycle/interaction 测试"
else
    echo "Using bvid: $BV"
fi

# 从 partition-compare 获取一个真实 category 用于 partition-detail 测试
CAT=$(curl -s "$BASE/api/partition-compare" | python3 -c "import sys,json,urllib.parse; d=json.load(sys.stdin); p=d['data']['partitions']; print(urllib.parse.quote(p[0]['category']) if p else '')" 2>/dev/null)
CAT_RAW=$(curl -s "$BASE/api/partition-compare" | python3 -c "import sys,json; d=json.load(sys.stdin); p=d['data']['partitions']; print(p[0]['category'] if p else '')" 2>/dev/null)
if [ -z "$CAT" ]; then
    echo "WARNING: 无分区数据，跳过 partition-detail 测试"
fi

run_test "TC-01: overview" \
  'curl -s "$BASE/api/overview" | python3 -c "import sys,json; d=json.load(sys.stdin); assert d[\"code\"]==0; assert d[\"data\"][\"video_count\"]>=0; print(\"PASS\")"'

run_test "TC-03: hot-videos default" \
  'curl -s "$BASE/api/hot-videos" | python3 -c "import sys,json; d=json.load(sys.stdin); assert d[\"code\"]==0; assert len(d[\"data\"][\"videos\"])<=20; print(\"PASS\")"'

run_test "TC-04: hot-videos sorted" \
  'curl -s "$BASE/api/hot-videos?sort=views&limit=5&offset=0" | python3 -c "
import sys,json
d=json.load(sys.stdin)
assert d[\"code\"]==0
videos=d[\"data\"][\"videos\"]
assert len(videos) <= 5
for i in range(len(videos)-1):
    assert videos[i][\"view_count\"] >= videos[i+1][\"view_count\"]
print(\"PASS\")"'

run_test "TC-05: hot-videos bad sort" \
  'curl -s "$BASE/api/hot-videos?sort=invalid" | python3 -c "import sys,json; d=json.load(sys.stdin); assert d[\"code\"]==400; print(\"PASS\")"'

run_test "TC-07: partition-compare" \
  'curl -s "$BASE/api/partition-compare" | python3 -c "
import sys,json
d=json.load(sys.stdin)
assert d[\"code\"]==0
for p in d[\"data\"][\"partitions\"]:
    assert 0 <= p[\"avg_like_rate\"] <= 1
    assert 0 <= p[\"avg_coin_rate\"] <= 1
print(\"PASS\")"'

if [ -n "$BV" ]; then
    run_test "TC-09: lifecycle (bvid=$BV)" \
      "curl -s '$BASE/api/lifecycle?bvid=$BV' | python3 -c \"
import sys,json
d=json.load(sys.stdin)
assert d['code']==0
assert d['data'] is not None
tl=d['data']['timeline']
assert len(tl) > 0
assert tl[0]['view_growth'] == 0
assert tl[0]['rank_change'] == 0
print('PASS')\""
fi

run_test "TC-10: lifecycle missing param" \
  'curl -s "$BASE/api/lifecycle" | python3 -c "import sys,json; d=json.load(sys.stdin); assert d[\"code\"]==400; print(\"PASS\")"'

run_test "TC-12: lifecycle/top" \
  'curl -s "$BASE/api/lifecycle/top?limit=3" | python3 -c "
import sys,json
d=json.load(sys.stdin)
assert d[\"code\"]==0
assert len(d[\"data\"][\"videos\"]) <= 3
for v in d[\"data\"][\"videos\"]:
    assert len(v[\"timeline\"]) > 0
print(\"PASS\")"'

if [ -n "$CAT" ]; then
    run_test "TC-14: partition-detail ($CAT_RAW)" \
      "curl -s '$BASE/api/partition-detail?category=$CAT' | python3 -c \"
import sys,json
d=json.load(sys.stdin)
assert d['code']==0
assert d['data']['category']=='$CAT_RAW'
assert len(d['data']['videos']) > 0
print('PASS')\""
fi

run_test "TC-17: tag-cloud" \
  'curl -s "$BASE/api/tag-cloud" | python3 -c "
import sys,json
d=json.load(sys.stdin)
assert d[\"code\"]==0
for t in d[\"data\"][\"tags\"]:
    assert isinstance(t[\"name\"], str)
    assert t[\"value\"] > 0
print(\"PASS\")"'

if [ -n "$BV" ]; then
    run_test "TC-20: interaction (bvid=$BV)" \
      "curl -s '$BASE/api/interaction?bvid=$BV' | python3 -c \"
import sys,json
d=json.load(sys.stdin)
assert d['code']==0
assert d['data']['bvid']=='$BV'
assert 0 <= d['data']['like_rate'] <= 1
print('PASS')\""
fi

run_test "TC-22: response format" \
  'curl -s "$BASE/api/overview" | python3 -c "
import sys,json
d=json.load(sys.stdin)
assert \"code\" in d
assert \"message\" in d
assert \"data\" in d
assert d[\"code\"]==0
print(\"PASS\")"'

echo ""
echo "Results: $PASS passed, $FAIL failed"
```

## 6. Frontend (React + ECharts)

### 6.1 大屏布局

```
┌──────────────────────────────────────────────────────┐
│  项目标题           数据时间 | 视频数 | 分区数 | 播放量 | 互动量   │
├──────────────┬───────────────────────┬────────────────┤
│ 分区排行     │   分区竞争力气泡图     │  热门视频排行   │
│ (柱状图)     │   / 热力图            │  (表格+排序)   │
│              │                       │                │
├──────────────┤                       ├────────────────┤
│ 标签词云     │                       │  互动质量分析   │
│              │                       │  (雷达图)      │
├──────────────┴───────────────────────┴────────────────┤
│              视频榜单生命周期 (折线图 + 时间轴)          │
└──────────────────────────────────────────────────────┘
```

### 6.2 图表 → API 映射

| 图表组件 | API | 交互 |
|---|---|---|
| 顶部 KPI 卡片 | `/api/overview` | — |
| 分区排行柱状图 | `/api/partition-compare` | 点击分区 → 联动词云和详情 |
| 标题词云 | `/api/word-cloud?source=title` | 点击词 → 筛选视频列表 |
| 分区竞争力气泡图 | `/api/partition-compare` | 悬停显示详情 |
| 热门视频排行 | `/api/hot-videos` | 点击视频 → 生命周期详情 |
| 互动质量雷达图 | `/api/interaction` | 跟随视频选择联动 |
| 生命周期折线图 | `/api/lifecycle/top` | 时间轴拖动、指标切换、多视频对比 |

### 6.3 TypeScript 类型定义

> 以下类型匹配 Section 5.3 API Contract，前端可直接用于 `types/index.ts`。

```typescript
// types/index.ts

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

// ---- 错误响应 ----
export interface ErrorData {
  code: number;
  message: string;
  data: null;
}
```

### 6.4 组件结构

```
frontend/src/
├── App.tsx                    # 大屏主布局
├── api/
│   └── index.ts               # axios 封装，统一请求
├── components/
│   ├── KPICards.tsx            # 顶部指标卡片
│   ├── PartitionBar.tsx        # 分区排行柱状图
│   ├── TagCloud.tsx            # 标签词云
│   ├── PartitionBubble.tsx     # 分区竞争力气泡图
│   ├── HotVideoTable.tsx       # 热门视频排行表格
│   ├── InteractionRadar.tsx    # 互动质量雷达图
│   ├── LifecycleChart.tsx      # 生命周期折线图 + 时间轴
│   └── common/
│       ├── ChartWrapper.tsx    # ECharts 容器封装
│       └── Loading.tsx         # 加载状态
├── hooks/
│   └── useApi.ts               # 数据请求 hook
├── types/
│   └── index.ts                # TypeScript 类型定义
└── styles/
    └── dashboard.css           # 大屏样式（深色主题）
```

### 6.5 技术栈

```
React 18 + TypeScript
echarts + echarts-for-react
axios (API 请求)
CSS Modules / Tailwind
```

### 6.6 验证

```bash
cd frontend && npm run dev
# 打开浏览器 http://localhost:5173
# 确认所有图表正常渲染、交互联动正常
```

## 7. Verification Checklist

| 阶段 | 检查项 | 命令/方法 |
|---|---|---|
| Stage 1 | 采集脚本能跑通、输出 NDJSON 文件 | `python main.py` |
| Stage 1 | 文件每行一个合法 JSON，含 aid/cid/snapshot_id/category | `head -1 output/.../rank_video.ndjson \| python3 -c "import sys,json; d=json.load(sys.stdin); assert 'aid' in d and 'snapshot_id' in d"` |
| Stage 2 | HDFS 目录已创建、文件已上传 | `hdfs dfs -ls /data/bilibili/ods/rank_video/` |
| Stage 2 | NDJSON 格式正确 | `hdfs dfs -head /data/bilibili/ods/rank_video/rank_video.ndjson` |
| Stage 3 | Hive 外部表能查到数据 | `hive -e "SELECT COUNT(*) FROM ods_rank_video_raw;"` |
| Stage 3 | 评论/弹幕/UP主表有数据且含 bvid | `hive -e "SELECT bvid FROM ods_video_comment_raw LIMIT 1;"` |
| Stage 4 | DWD 各表建表成功 | `hive -e "SHOW TABLES;" \| grep dwd` |
| Stage 4 | 视频信息按 bvid 唯一 | `hive -e "SELECT bvid, COUNT(*) c FROM dwd_video_info GROUP BY bvid HAVING c > 1;"` — 应返回 0 行 |
| Stage 4 | DWD 清洗后无异常数据 | `hive -e "SELECT * FROM dwd_video_snapshot WHERE view_count < 0;"` — 应返回 0 行 |
| Stage 5 | ADS 各表有数据 | `hive -e "SELECT * FROM ads_screen_overview;"` |
| Stage 5 | 分区竞争力数据完整 | `hive -e "SELECT COUNT(*) FROM ads_partition_compare;"` |
| Stage 5 | 词云有数据 | `hive -e "SELECT source_type, COUNT(*) FROM ads_word_cloud GROUP BY source_type;"` |
| Stage 5 | 生命周期数据有时间序列 | `hive -e "SELECT bvid, COUNT(*) FROM ads_video_lifecycle GROUP BY bvid;"` — 每视频应有多行 |
| Stage 5 | 热度分数 ∈ [0, 1] | `hive -e "SELECT MIN(heat_score), MAX(heat_score) FROM ads_hot_video_rank;"` |
| Stage 6 | MySQL 中有全部 ADS 数据 | `mysql -u root bilibili -e "SHOW TABLES;"` |
| Stage 6 | 各表行数合理 | `mysql -u root bilibili -e "SELECT COUNT(*) FROM ads_hot_video_rank;"` |
| Stage 7 | Rust 后端启动 | `cargo run` |
| Stage 7 | 8 个端点全部可用 | `bash test_api.sh` |
| Stage 7 | 错误参数返回 400 | `curl -s "$BASE/api/hot-videos?sort=x"` → `code=400` |
| Stage 8 | 前端启动 | `cd frontend && npm run dev` |
| Stage 8 | 所有图表渲染 | 浏览器打开确认 |
| Stage 8 | 交互联动正常 | 点击分区/视频，观察图表联动 |

## 8. Deliverables

| 产出 | 内容 | 目录/文件 |
|---|---|---|---|
| 采集脚本 | Python 爬虫，NDJSON 输出，含 WBI 签名、protobuf 解析 | `crawler/` |
| 采集数据示例 | 采集输出的 NDJSON 格式示例 | `output/` |
| HDFS 上传脚本 | 批量上传 ODS 数据的 shell 脚本 | `scripts/upload_hdfs.sh` |
| Hive DDL | ODS 外部表建表语句 | `sql/hive_ods_ddl.sql` |
| Spark SQL ETL | DWD 清洗 + DWS/ADS 分析 SQL 脚本 | `sql/spark_etl.sql` |
| MapReduce 作业 | 标题词频统计 | `mapreduce/` |
| MySQL DDL | ADS 表建表语句 | `sql/mysql_ads_ddl.sql` |
| 数据导出脚本 | PySpark JDBC 导出脚本 | `scripts/export_to_mysql.py` |
| 管线脚本 | 日常 ETL 编排 + 首次初始化 | `scripts/run_pipeline.sh` / `scripts/setup_pipeline.sh` |
| 后端代码 | Rust Axum 项目，8 个只读 API 端点 | `backend/` |
| 前端代码 | React + ECharts 大屏项目 | `frontend/` |
| 测试脚本 | API 端点验证脚本 | `scripts/test_api.sh` |
| 截图 | 大屏全貌 + 各交互截图 | `screenshots/` |
| 课程报告 | Hadoop 流程 + 数据分析结论 + 架构说明 | `report/` |
| PPT | 项目展示 + 演示 | `ppt/` |
