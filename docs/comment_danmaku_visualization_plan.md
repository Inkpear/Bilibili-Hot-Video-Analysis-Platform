# 评论与弹幕内容可视化扩展实施计划

> 目标：让已采集的评论和弹幕内容真正进入分析链路，形成能体现 B 站弹幕流媒体平台特色的可视化结果。
>
> 更新时间：2026-06-02

## 1. 背景与问题

当前平台的视频榜单、分区竞争、生命周期、后端 API、FRP 穿透均已可用。但评论和弹幕内容存在明显利用不足：

| 数据 | 本地累计规模 | 当前 Hive/DWD 规模 | 当前 ADS 贡献 |
|---|---:|---:|---|
| 评论内容 | 约 193,500 行 | 约 6,344 行 | 无 |
| 弹幕内容 | 约 344,680 行 | 约 8,219 行 | 无 |

根因是 `scripts/upload_hdfs.sh` 上传评论/弹幕时会删除 HDFS 旧文件：

```bash
hdfs dfs -rm -r ${HDFS_BASE}/video_comment/* >/dev/null 2>&1 || true
hdfs dfs -rm -r ${HDFS_BASE}/video_danmaku/* >/dev/null 2>&1 || true
```

这导致本地历史评论/弹幕虽然还在，但 HDFS/Hive 当前只保留最新一轮少量新视频的数据。

当前评论/弹幕与最新热门 TOP100 的覆盖情况：

| 覆盖关系 | 当前数量 |
|---|---:|
| 最新快照视频数 | 954 |
| 热门 TOP100 视频数 | 100 |
| DWD 评论覆盖最新快照视频 | 34 |
| DWD 弹幕覆盖最新快照视频 | 32 |
| DWD 评论覆盖热门 TOP100 | 0 |
| DWD 弹幕覆盖热门 TOP100 | 0 |

因此，在修复数据链路前，不能直接基于当前 Hive 评论/弹幕数据做评论词云、弹幕词云或热门评论，否则结论会严重失真。

## 2. 最终目标

形成以下分析闭环：

```text
本地采集评论/弹幕内容
→ HDFS 持久保存全部历史内容
→ Hive DWD 去重清洗
→ ADS 内容分析表
→ Rust API
→ React + ECharts 大屏展示
```

新增可视化重点：

| 可视化 | 数据来源 | 分析含义 |
|---|---|---|
| 标题词云 | 视频标题 | 内容生产侧：视频在讲什么 |
| 评论词云 | 评论内容 | 社区讨论侧：观众在聊什么 |
| 弹幕词云 | 弹幕内容 | 即时反应侧：观众观看时在刷什么 |
| 热门评论榜 | 评论内容 + 点赞/回复 | 社区共鸣样本 |

暂不优先做情绪分析和弹幕时间热力图。它们可以作为后续增强，但第一阶段应先保证数据完整和展示可信。

## 3. 阶段一：修复评论/弹幕数据保留问题

### 3.1 修改 HDFS 上传逻辑

修改 `scripts/upload_hdfs.sh`：

| 当前行为 | 目标行为 |
|---|---|
| 上传评论前删除 `/data/bilibili/ods/video_comment/*` | 不删除旧文件，只覆盖当前快照文件 |
| 上传弹幕前删除 `/data/bilibili/ods/video_danmaku/*` | 不删除旧文件，只覆盖当前快照文件 |

建议逻辑：

```bash
hdfs dfs -rm -f ${HDFS_BASE}/video_comment/${SNAPSHOT_ID}_comments.ndjson >/dev/null 2>&1 || true
hdfs dfs -put -f - ${HDFS_BASE}/video_comment/${SNAPSHOT_ID}_comments.ndjson

hdfs dfs -rm -f ${HDFS_BASE}/video_danmaku/${SNAPSHOT_ID}_danmaku.ndjson >/dev/null 2>&1 || true
hdfs dfs -put -f - ${HDFS_BASE}/video_danmaku/${SNAPSHOT_ID}_danmaku.ndjson
```

### 3.2 补传历史评论/弹幕

从 WSL 本地 `~/output/*/comments/` 和 `~/output/*/danmaku/` 补传所有历史快照。

推荐方式：

```bash
for snapshot in ~/output/*; do
  bash ~/scripts/upload_hdfs.sh "$(basename "$snapshot")"
done
```

注意：必须先完成 3.1，否则循环补传仍会互相覆盖。

### 3.3 修复 DWD 去重逻辑

当前 `dwd_comment_info` 和 `dwd_danmaku_info` 没有显式按 ID 去重，与项目约定不一致。

评论去重建议：

```sql
ROW_NUMBER() OVER (
  PARTITION BY comment_id
  ORDER BY comment_time DESC
) AS rn
```

弹幕去重建议：

```sql
ROW_NUMBER() OVER (
  PARTITION BY danmaku_id
  ORDER BY send_time DESC
) AS rn
```

### 3.4 重跑 ETL 和导出

执行：

```bash
bash ~/scripts/run_pipeline.sh
```

或者至少执行：

```bash
spark-sql --master spark://master:7077 -f /home/hadoop/sql/spark_etl.sql
spark-submit --jars /opt/spark/jars/mysql-connector-j-8.0.33.jar \
  --master spark://master:7077 \
  /home/hadoop/scripts/export_to_mysql.py
```

### 3.5 阶段一验收标准

| 指标 | 合格标准 |
|---|---:|
| DWD 评论行数 | 接近本地累计评论行数，当前约 193,500 |
| DWD 弹幕行数 | 接近本地累计弹幕行数，当前约 344,680 |
| 评论覆盖视频数 | 接近本地评论文件数，当前约 1,106 |
| 弹幕覆盖视频数 | 接近本地弹幕文件数，当前约 1,012 |
| 热门 TOP100 评论覆盖 | 至少 70 个视频 |
| 热门 TOP100 弹幕覆盖 | 至少 70 个视频 |
| `comment_id` 重复 | 0 |
| `danmaku_id` 重复 | 0 |

## 4. 阶段二：新增内容分析 ADS 表

### 4.1 统一词云表

建议将标题词云、评论词云、弹幕词云统一到一张表，而不是新增多张结构重复的表。

MySQL ADS 表：

```sql
CREATE TABLE ads_word_cloud (
    id          INT AUTO_INCREMENT PRIMARY KEY,
    source_type VARCHAR(32),    -- title/comment/danmaku
    category    VARCHAR(64),    -- 全站/游戏/动画/...
    word        VARCHAR(64),
    word_count  INT,
    update_time DATETIME DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_source_category (source_type, category),
    INDEX idx_word (word)
);
```

字段含义：

| 字段 | 说明 |
|---|---|
| `source_type='title'` | 标题词云，表达内容主题 |
| `source_type='comment'` | 评论词云，表达观众讨论主题 |
| `source_type='danmaku'` | 弹幕词云，表达即时反应和梗文化 |
| `category='全站'` | 全局词云 |
| `category='游戏'` 等 | 分区词云 |

### 4.2 热门评论表

MySQL ADS 表：

```sql
CREATE TABLE ads_hot_comments (
    id           INT AUTO_INCREMENT PRIMARY KEY,
    bvid         VARCHAR(32),
    title        VARCHAR(512),
    category     VARCHAR(64),
    up_name      VARCHAR(128),
    user_name    VARCHAR(128),
    content      TEXT,
    like_count   BIGINT,
    reply_count  BIGINT,
    comment_time DATETIME,
    update_time  DATETIME DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_category (category),
    INDEX idx_bvid (bvid),
    INDEX idx_like (like_count)
);
```

推荐口径：

| 场景 | 排序 |
|---|---|
| 全局热门评论 | `like_count DESC, reply_count DESC` |
| 分区热门评论 | 按 `category` 筛选后排序 |
| 单视频热门评论 | 按 `bvid` 筛选后排序 |

### 4.3 不建议第一阶段做的 ADS

暂不新增情绪分析表和弹幕时间热力表。

原因：

| 功能 | 暂缓原因 |
|---|---|
| 评论情绪分析 | 中文短文本、反讽、表情、梗词多，简单词典容易误判 |
| 弹幕情绪分析 | 弹幕过短，语义稀疏，噪声高 |
| 弹幕时间热力图 | 需要按视频时长归一化，前端交互复杂，适合作为后续增强 |

## 5. 阶段三：MapReduce 词云处理扩展

### 5.1 复用现有 jieba 分词能力

继续使用 Hadoop Streaming + Python3 + jieba。

原因：

| 方案 | 评价 |
|---|---|
| Hadoop Streaming + jieba | 已部署、已验证、中文分词效果好 |
| Spark MLlib | 中文分词能力弱，需要额外依赖，不推荐 |

### 5.2 输入生成策略

标题词云输入：

```sql
SELECT 'title' AS source_type, category, title AS content
FROM ads_hot_video_rank
WHERE title IS NOT NULL AND title != '';
```

评论词云输入：

```sql
SELECT 'comment' AS source_type, h.category, c.content
FROM dwd_comment_info c
JOIN ads_hot_video_rank h ON c.bvid = h.bvid
WHERE c.content IS NOT NULL AND c.content != '';
```

弹幕词云输入：

```sql
SELECT 'danmaku' AS source_type, h.category, d.content
FROM dwd_danmaku_info d
JOIN ads_hot_video_rank h ON d.bvid = h.bvid
WHERE d.content IS NOT NULL AND d.content != '';
```

### 5.3 停用词策略

标题、评论、弹幕不应完全使用同一套停用词。

建议：

| source_type | 停用词重点 |
|---|---|
| title | 常见虚词、平台词、标题符号 |
| comment | 表情词、口水词、语气词、常见无意义回复 |
| danmaku | `哈哈哈`、`？`、`？？？` 可保留部分，但极端重复项要限制 |

弹幕词云不应把所有“哈哈哈”“？？？”过滤掉，因为它们本身代表即时反应。但需要限制它们不要完全淹没其他词。

### 5.4 阶段三验收标准

| 指标 | 合格标准 |
|---|---:|
| `ads_word_cloud source=title` | > 20 个词 |
| `ads_word_cloud source=comment` | > 30 个词 |
| `ads_word_cloud source=danmaku` | > 30 个词 |
| 每个 source 全站词云 | 存在 `category='全站'` |
| 分区词云 | 至少覆盖 5 个主要分区 |

## 6. 阶段四：后端 API 扩展

### 6.1 新增 `/api/word-cloud`

请求：

```text
GET /api/word-cloud?source=title&category=游戏
GET /api/word-cloud?source=comment&category=游戏
GET /api/word-cloud?source=danmaku&category=游戏
```

参数：

| 参数 | 必填 | 默认值 | 说明 |
|---|---|---|---|
| `source` | 否 | `title` | `title/comment/danmaku` |
| `category` | 否 | `全站` | 分区名 |

响应：

```json
{
  "code": 0,
  "message": "ok",
  "data": {
    "source": "comment",
    "category": "游戏",
    "words": [
      { "name": "剧情", "value": 42 },
      { "name": "操作", "value": 37 }
    ]
  }
}
```

错误：

| 场景 | HTTP | code |
|---|---:|---:|
| `source` 非法 | 400 | 400 |
| 无数据 | 200 | 0，`words=[]` |

### 6.2 `/api/tag-cloud` 已废弃

旧 `/api/tag-cloud` 接口已删除，统一替换为 `/api/word-cloud?source=title`。

### 6.3 新增 `/api/hot-comments`

请求：

```text
GET /api/hot-comments?category=游戏&limit=20
GET /api/hot-comments?bvid=BVxxx&limit=10
```

参数：

| 参数 | 必填 | 默认值 | 说明 |
|---|---|---|---|
| `category` | 否 | 无 | 分区筛选 |
| `bvid` | 否 | 无 | 单视频筛选 |
| `limit` | 否 | 20 | 0~100 |

响应：

```json
{
  "code": 0,
  "message": "ok",
  "data": {
    "comments": [
      {
        "bvid": "BVxxx",
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

### 6.4 阶段四验收标准

| 接口 | 合格标准 |
|---|---|
| `/api/word-cloud?source=title` | 返回标题词云 |
| `/api/word-cloud?source=comment` | 返回评论词云 |
| `/api/word-cloud?source=danmaku` | 返回弹幕词云 |
| `/api/hot-comments` | 返回热门评论列表 |
| 鉴权 | 未带 token 返回 401 |

## 7. 阶段五：前端可视化扩展

### 7.1 推荐新版大屏布局

```text
┌─────────────────────────────────────────────────────────────┐
│ KPI: 视频数 / 分区数 / UP数 / 播放量 / 互动量 / 数据时间       │
├──────────────────┬──────────────────┬───────────────────────┤
│ 分区排行柱状图    │ 分区竞争力气泡图   │ 热门视频榜             │
├──────────────────┼──────────────────┼───────────────────────┤
│ 标题词云          │ 评论词云           │ 弹幕词云               │
├──────────────────┴──────────────────┴───────────────────────┤
│ 热门评论 / 高赞观点榜                                         │
├─────────────────────────────────────────────────────────────┤
│ 生命周期折线图 + 互动质量雷达图                                │
└─────────────────────────────────────────────────────────────┘
```

### 7.2 新增组件

| 组件 | API | 说明 |
|---|---|---|
| `WordCloudPanel.tsx` | `/api/word-cloud` | 三词云统一组件 |
| `HotComments.tsx` | `/api/hot-comments` | 高赞评论列表 |
| `SourceTabs.tsx` | `/api/word-cloud` | 标题/评论/弹幕切换 |

### 7.3 图表解释口径

| 图表 | 标题建议 | 解释 |
|---|---|---|
| 标题词云 | 内容主题词云 | 视频标题中的热门主题 |
| 评论词云 | 观众讨论词云 | 评论区关注点 |
| 弹幕词云 | 即时反应词云 | 观看过程中的情绪和梗 |
| 热门评论 | 高赞观点榜 | 观众最认同的评论样本 |

### 7.4 交互设计

推荐联动：

| 操作 | 联动结果 |
|---|---|
| 点击分区柱状图 | 刷新三词云、热门评论、热门视频 |
| 点击热门视频 | 刷新生命周期、互动雷达、单视频热门评论 |
| 切换词云 source | 在标题/评论/弹幕词云之间切换 |

### 7.5 阶段五验收标准

| 检查项 | 合格标准 |
|---|---|
| 三词云 | 均能渲染且含义区分清楚 |
| 热门评论榜 | 文本不溢出，支持分区/视频联动 |
| 分区点击 | 能刷新相关图表 |
| 视频点击 | 能刷新生命周期、雷达图、热门评论 |
| 空数据 | 显示“暂无数据”，不崩溃 |

## 8. 阶段六：文档与答辩材料更新

### 8.1 需要更新的文档

| 文件 | 更新内容 |
|---|---|
| `docs/plan.md` | 新增评论/弹幕内容分析阶段和前端布局 |
| `docs/api.md` | 新增 `/api/word-cloud`、`/api/hot-comments` |
| `AGENTS.md` | 更新当前状态和新增约定 |
| `history.md` | 记录修复和扩展过程 |

### 8.2 答辩叙事建议

推荐讲法：

```text
本项目不仅分析播放、点赞、投币等数值指标，还利用 B 站独有的评论区与弹幕文化数据，构建“内容主题—社区讨论—即时反应”三层文本分析。
```

三词云解释：

```text
标题词云：创作者如何包装内容。
评论词云：观众事后如何讨论内容。
弹幕词云：观众观看时如何即时反应。
```

热门评论解释：

```text
高赞评论体现了社区共鸣，是理解热门视频为什么引发讨论的重要补充。
```

## 9. 风险与注意事项

| 风险 | 影响 | 处理建议 |
|---|---|---|
| 评论/弹幕历史数据未补传 | 新增图表失真 | 必须先完成阶段一 |
| 词云噪声过多 | 图表难解释 | 增强停用词，按 source 分开维护 |
| 弹幕“哈哈哈/？？？”过多 | 弹幕词云被口水词淹没 | 保留但限制排名或做停用词权衡 |
| 热门评论文本过长 | 前端布局混乱 | 截断展示，悬停显示全文 |
| 情绪分析误判 | 结论不可信 | 暂不做第一阶段核心功能 |
| 弹幕时间热力复杂 | 工期失控 | 放到后续增强 |

## 10. 推荐优先级

| 优先级 | 任务 | 原因 |
|---|---|---|
| P0 | 修复评论/弹幕 HDFS 覆盖问题 | 不修则内容分析无意义 |
| P0 | 补传历史评论/弹幕 | 恢复真实数据资产 |
| P0 | DWD 评论/弹幕按 ID 去重 | 保证统计口径正确 |
| P1 | 新增统一 `ads_word_cloud` | 支撑三词云 |
| P1 | 新增 `/api/word-cloud` | 支撑前端复用 |
| P1 | 新增热门评论 ADS/API | 直观展示社区讨论 |
| P2 | 前端三词云和热门评论组件 | 提升展示效果 |
| P3 | 弹幕时间热力图 | 加分项，非第一阶段必需 |

## 11. 完整验收清单

### 数据链路

| 检查项 | 合格标准 |
|---|---|
| HDFS 评论文件 | 多个快照文件共存 |
| HDFS 弹幕文件 | 多个快照文件共存 |
| DWD 评论行数 | 接近本地累计行数 |
| DWD 弹幕行数 | 接近本地累计行数 |
| 评论/弹幕 JOIN 热门 TOP100 | 覆盖大部分热门视频 |

### ADS/API

| 检查项 | 合格标准 |
|---|---|
| `ads_word_cloud` | `title/comment/danmaku` 三类均有数据 |
| `ads_hot_comments` | 有全局、分区、单视频热门评论 |
| `/api/word-cloud` | 支持 source/category |
| `/api/hot-comments` | 支持 category/bvid/limit |

### 前端

| 检查项 | 合格标准 |
|---|---|
| 标题词云 | 正常展示 |
| 评论词云 | 正常展示 |
| 弹幕词云 | 正常展示 |
| 热门评论榜 | 正常展示 |
| 分区/视频联动 | 正常工作 |

## 12. 不建议当前立即实施的功能

| 功能 | 原因 |
|---|---|
| 评论情绪分析 | 中文短文本反讽、梗词、表情复杂，简单词典不可靠 |
| 弹幕情绪分析 | 文本过短，语义不足 |
| 弹幕时间热力图 | 需要按视频时长归一化，前端交互复杂 |
| 评论用户画像 | 缺少稳定用户画像字段，且隐私敏感 |

## 13. 最终建议

第一轮扩展只做：

```text
修复数据保留 → 三词云 → 热门评论
```

不要同时引入情绪分析和弹幕热力图。这样能以较低风险，把评论/弹幕内容转化为清晰、有解释力、适合答辩展示的可视化结果。
