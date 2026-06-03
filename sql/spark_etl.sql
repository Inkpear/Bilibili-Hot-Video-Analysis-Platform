-- ============================================================================
-- Spark SQL ETL: ODS → DWD → DWS → ADS
-- 运行方式: spark-sql --master spark://master:7077 -f sql/spark_etl.sql
--     或: spark-submit + Python 脚本逐段执行
-- ============================================================================

USE bilibili;

-- ============================================================================
-- Stage 4: DWD 清洗层
-- ============================================================================

-- 4.1 视频快照表（按 snapshot_date 分区，去重）
DROP TABLE IF EXISTS dwd_video_snapshot;
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

-- 4.2 视频基础信息表（按 bvid 唯一）
DROP TABLE IF EXISTS dwd_video_info;
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
            ORDER BY CASE WHEN category != '其他' AND category IS NOT NULL THEN 0 ELSE 1 END,
                     CASE WHEN COALESCE(description, '') != '' THEN 0 ELSE 1 END
        ) AS rn
    FROM ods_rank_video_raw
    WHERE bvid IS NOT NULL
      AND title IS NOT NULL
) t
WHERE t.rn = 1;

-- 4.3 评论明细表
DROP TABLE IF EXISTS dwd_comment_info;
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
    like_count,
    reply_count,
    comment_time
FROM (
    SELECT
        c.comment_id,
        c.bvid,
        c.aid,
        c.user_id,
        c.user_name,
        c.content,
        COALESCE(c.like_count, 0) AS like_count,
        COALESCE(c.reply_count, 0) AS reply_count,
        CAST(FROM_UNIXTIME(CAST(c.comment_time AS BIGINT)) AS TIMESTAMP) AS comment_time,
        ROW_NUMBER() OVER (
            PARTITION BY c.comment_id
            ORDER BY CAST(c.comment_time AS BIGINT) DESC
        ) AS rn
    FROM ods_video_comment_raw c
    WHERE c.content IS NOT NULL
      AND c.content != ''
      AND c.bvid IS NOT NULL
) t
WHERE t.rn = 1;

-- 4.4 弹幕明细表
DROP TABLE IF EXISTS dwd_danmaku_info;
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
    send_time
FROM (
    SELECT
        d.danmaku_id,
        d.bvid,
        d.cid,
        d.content,
        d.video_time,
        d.mode,
        d.fontsize,
        d.color,
        d.mid_hash,
        CAST(FROM_UNIXTIME(CAST(d.send_time AS BIGINT)) AS TIMESTAMP) AS send_time,
        ROW_NUMBER() OVER (
            PARTITION BY d.danmaku_id
            ORDER BY CAST(d.send_time AS BIGINT) DESC
        ) AS rn
    FROM ods_video_danmaku_raw d
    WHERE d.content IS NOT NULL
      AND d.content != ''
      AND d.bvid IS NOT NULL
) t
WHERE t.rn = 1;

-- 4.5 UP 主信息表
DROP TABLE IF EXISTS dwd_up_info;
CREATE TABLE dwd_up_info (
    up_id           STRING,
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

-- ============================================================================
-- Stage 5: DWS / ADS 分析层
-- ============================================================================

-- 5.1 分区每日汇总 (DWS)
DROP TABLE IF EXISTS dws_partition_daily_stats;
CREATE TABLE dws_partition_daily_stats AS
WITH canonical AS (
    SELECT *
    FROM (
        SELECT *,
            ROW_NUMBER() OVER (
                PARTITION BY bvid, snapshot_id
                ORDER BY rank_position
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

-- 5.2 视频生命周期 (DWS)
DROP TABLE IF EXISTS dws_video_lifecycle_stats;
CREATE TABLE dws_video_lifecycle_stats AS
WITH latest_collect AS (
    SELECT MAX(snapshot_id) AS max_sid FROM dwd_video_snapshot
),
top_videos AS (
    SELECT s.bvid
    FROM dwd_video_snapshot s
    JOIN latest_collect lc ON s.snapshot_id = lc.max_sid
    ORDER BY s.view_count DESC
    LIMIT 100
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
    CONCAT(SUBSTR(s.snapshot_id, 10, 2), ':', SUBSTR(s.snapshot_id, 12, 2)) AS snapshot_time,
    s.view_count,
    s.like_count,
    s.rank_position,
    s.view_count - LAG(s.view_count) OVER (
        PARTITION BY s.bvid ORDER BY s.snapshot_id
    ) AS view_growth,
    LAG(s.rank_position) OVER (
        PARTITION BY s.bvid ORDER BY s.snapshot_id
    ) - s.rank_position AS rank_change
FROM dwd_video_snapshot s
JOIN dwd_video_info v ON s.bvid = v.bvid
WHERE s.bvid IN (SELECT bvid FROM top_videos);

-- 5.3 UP 主汇总 (DWS)
DROP TABLE IF EXISTS dws_up_summary_stats;
CREATE TABLE dws_up_summary_stats AS
WITH canonical AS (
    SELECT *
    FROM (
        SELECT *,
            ROW_NUMBER() OVER (
                PARTITION BY bvid, snapshot_id
                ORDER BY rank_position
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
LEFT JOIN dwd_video_info v ON u.up_id = v.up_id
LEFT JOIN canonical s ON v.bvid = s.bvid
GROUP BY u.up_id, u.up_name, u.up_avatar, u.follower_count, u.video_count;

-- ============================================================================
-- ADS 最终层 (导出到 MySQL)
-- ============================================================================

-- 5.4 大屏总览
DROP TABLE IF EXISTS ads_screen_overview;
CREATE TABLE ads_screen_overview AS
WITH canonical AS (
    SELECT *
    FROM (
        SELECT *,
            ROW_NUMBER() OVER (
                PARTITION BY bvid, snapshot_id
                ORDER BY rank_position
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

-- 5.5 热门视频排行（含热度分数）
DROP TABLE IF EXISTS ads_hot_video_rank;
CREATE TABLE ads_hot_video_rank AS
WITH latest AS (
    SELECT *
    FROM (
        SELECT *,
            ROW_NUMBER() OVER (
                PARTITION BY bvid, snapshot_id
                ORDER BY rank_position
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
ORDER BY heat_score DESC;

-- 5.6 分区竞争力
DROP TABLE IF EXISTS ads_partition_compare;
CREATE TABLE ads_partition_compare AS
WITH canonical AS (
    SELECT *
    FROM (
        SELECT *,
            ROW_NUMBER() OVER (
                PARTITION BY bvid, snapshot_id
                ORDER BY rank_position
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

-- 5.7 统一词云（标题/评论/弹幕）
DROP TABLE IF EXISTS ads_word_cloud;
CREATE TABLE ads_word_cloud (
    source_type  STRING,
    category     STRING,
    word         STRING,
    word_count   INT
)
STORED AS PARQUET;

-- 5.8 热门评论
DROP TABLE IF EXISTS ads_hot_comments;
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

INSERT OVERWRITE TABLE ads_hot_comments
SELECT
    c.bvid,
    v.title,
    v.category,
    v.up_name,
    c.user_name,
    c.content,
    c.like_count,
    c.reply_count,
    c.comment_time
FROM dwd_comment_info c
JOIN dwd_video_info v ON c.bvid = v.bvid
WHERE c.content IS NOT NULL AND c.content != ''
ORDER BY c.like_count DESC
LIMIT 5000;

INSERT OVERWRITE TABLE ads_word_cloud
-- 标题词云数据（由 MapReduce 生成，此处仅建空表框架）
SELECT source_type, category, word, word_count
FROM (
    SELECT 'title' AS source_type, '__ALL__' AS category, '__placeholder__' AS word, 0 AS word_count
) t
WHERE 1 = 0;

-- 5.9 视频生命周期 ADS
DROP TABLE IF EXISTS ads_video_lifecycle;
CREATE TABLE ads_video_lifecycle AS
SELECT
    bvid,
    title,
    category,
    snapshot_date,
    snapshot_time,
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

-- 5.10 分区每日 ADS
DROP TABLE IF EXISTS ads_partition_daily;
CREATE TABLE ads_partition_daily AS
SELECT * FROM dws_partition_daily_stats;

-- 5.11 UP 主汇总 ADS
DROP TABLE IF EXISTS ads_up_summary;
CREATE TABLE ads_up_summary AS
SELECT * FROM dws_up_summary_stats;

-- ============================================================================
-- 验证
-- ============================================================================
SELECT 'ads_screen_overview' AS tbl, COUNT(*) FROM ads_screen_overview;
SELECT 'ads_hot_video_rank'  AS tbl, COUNT(*) FROM ads_hot_video_rank;
SELECT 'ads_partition_compare' AS tbl, COUNT(*) FROM ads_partition_compare;
SELECT 'ads_video_lifecycle' AS tbl, COUNT(*) FROM ads_video_lifecycle;
SELECT 'ads_partition_daily' AS tbl, COUNT(*) FROM ads_partition_daily;
SELECT 'ads_up_summary'      AS tbl, COUNT(*) FROM ads_up_summary;
SELECT 'ads_word_cloud'      AS tbl, COUNT(*) FROM ads_word_cloud;
SELECT 'ads_hot_comments'    AS tbl, COUNT(*) FROM ads_hot_comments;
