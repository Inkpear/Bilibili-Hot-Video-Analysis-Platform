-- scripts/hive_ddl_full.sql
-- Hive Data Warehouse DDL for Bilibili Hot Video Analysis Platform v2
-- Layers: ODS (raw) → DWD (clean) → DWS (aggregate) → ADS (application)
-- Storage: ODS → JSON on HDFS; DWD/DWS/ADS → PARQUET
-- ============================================================

CREATE DATABASE IF NOT EXISTS bilibili;
USE bilibili;

-- ============================================================
-- ODS Layer — External tables pointing to HDFS raw JSON data
-- ============================================================

-- ODS: Raw video data ingested from Bilibili API
CREATE EXTERNAL TABLE IF NOT EXISTS ods_video (
    bvid          STRING,
    aid           BIGINT,
    cid           BIGINT,
    title         STRING,
    description   STRING,
    tags          STRING,
    sub_category  STRING,
    duration      INT,
    publish_time  BIGINT,
    up_id         STRING,
    up_name       STRING,
    up_avatar     STRING,
    view_count    BIGINT,
    like_count    BIGINT,
    coin_count    BIGINT,
    favorite_count BIGINT,
    share_count   BIGINT,
    reply_count   BIGINT,
    danmaku_count BIGINT
)
PARTITIONED BY (`date` STRING)
ROW FORMAT SERDE 'org.apache.hive.hcatalog.data.JsonSerDe'
LOCATION '/bilibili/raw/video';

-- ODS: Raw comment data ingested from Bilibili API
CREATE EXTERNAL TABLE IF NOT EXISTS ods_comment (
    comment_id    BIGINT,
    bvid          STRING,
    aid           BIGINT,
    user_id       BIGINT,
    user_name     STRING,
    content       STRING,
    like_count    INT,
    reply_count   INT,
    comment_time  BIGINT
)
PARTITIONED BY (`date` STRING)
ROW FORMAT SERDE 'org.apache.hive.hcatalog.data.JsonSerDe'
LOCATION '/bilibili/raw/comment';

-- ODS: Raw danmaku data ingested from Bilibili API
CREATE EXTERNAL TABLE IF NOT EXISTS ods_danmaku (
    danmaku_id    BIGINT,
    bvid          STRING,
    cid           BIGINT,
    content       STRING,
    video_time    DOUBLE,
    mode          INT,
    fontsize      INT,
    color         BIGINT,
    mid_hash      STRING,
    send_time     BIGINT
)
PARTITIONED BY (`date` STRING)
ROW FORMAT SERDE 'org.apache.hive.hcatalog.data.JsonSerDe'
LOCATION '/bilibili/raw/danmaku';

-- ============================================================
-- DWD Layer — Cleaned, filtered, deduplicated data (PARQUET)
-- ============================================================

-- DWD: Cleaned video with derived interaction rates
CREATE TABLE IF NOT EXISTS dwd_video_clean (
    bvid          STRING,
    aid           BIGINT,
    cid           BIGINT,
    title         STRING,
    description   STRING,
    tags          STRING,
    sub_category  STRING,
    duration      INT,
    publish_time  BIGINT,
    up_id         STRING,
    up_name       STRING,
    up_avatar     STRING,
    view_count    BIGINT,
    like_count    BIGINT,
    coin_count    BIGINT,
    favorite_count BIGINT,
    share_count   BIGINT,
    reply_count   BIGINT,
    danmaku_count BIGINT,
    like_rate     DOUBLE,
    coin_rate     DOUBLE,
    fav_rate      DOUBLE,
    reply_rate    DOUBLE,
    share_rate    DOUBLE
)
PARTITIONED BY (`date` STRING)
STORED AS PARQUET;

-- DWD: Cleaned comment data
CREATE TABLE IF NOT EXISTS dwd_comment_clean (
    comment_id    BIGINT,
    bvid          STRING,
    aid           BIGINT,
    user_id       BIGINT,
    user_name     STRING,
    content       STRING,
    like_count    INT,
    reply_count   INT,
    comment_time  BIGINT
)
PARTITIONED BY (`date` STRING)
STORED AS PARQUET;

-- DWD: Cleaned danmaku data
CREATE TABLE IF NOT EXISTS dwd_danmaku_clean (
    danmaku_id    BIGINT,
    bvid          STRING,
    cid           BIGINT,
    content       STRING,
    video_time    DOUBLE,
    mode          INT,
    fontsize      INT,
    color         BIGINT,
    mid_hash      STRING,
    send_time     BIGINT
)
PARTITIONED BY (`date` STRING)
STORED AS PARQUET;

-- ============================================================
-- DWS Layer — Lightweight aggregated statistics (PARQUET)
-- ============================================================

-- DWS: Per-category / per-partition rollup
CREATE TABLE IF NOT EXISTS dws_partition_stats (
    sub_category   STRING,
    video_count    BIGINT,
    total_views    BIGINT,
    total_likes    BIGINT,
    total_coins    BIGINT,
    total_favorites BIGINT,
    avg_like_rate  DOUBLE,
    avg_coin_rate  DOUBLE,
    avg_fav_rate   DOUBLE
)
PARTITIONED BY (`date` STRING)
STORED AS PARQUET;

-- ============================================================
-- ADS Layer — Application tables consumed by API / MySQL export
-- ============================================================

-- ADS: Full hot-video record set (denormalized for API)
CREATE TABLE IF NOT EXISTS ads_hot_video (
    bvid          STRING,
    aid           BIGINT,
    cid           BIGINT,
    title         STRING,
    description   STRING,
    tags          STRING,
    sub_category  STRING,
    duration      INT,
    publish_time  BIGINT,
    up_id         STRING,
    up_name       STRING,
    up_avatar     STRING,
    view_count    BIGINT,
    like_count    BIGINT,
    coin_count    BIGINT,
    favorite_count BIGINT,
    share_count   BIGINT,
    reply_count   BIGINT,
    danmaku_count BIGINT,
    snapshot_date STRING
)
STORED AS PARQUET;

-- ADS: Per-video interaction rates for ranking
CREATE TABLE IF NOT EXISTS ads_video_interaction (
    bvid          STRING,
    snapshot_date STRING,
    like_rate     DOUBLE,
    coin_rate     DOUBLE,
    fav_rate      DOUBLE,
    reply_rate    DOUBLE,
    share_rate    DOUBLE
)
STORED AS PARQUET;

-- ADS: Comment data for application layer
CREATE TABLE IF NOT EXISTS ads_comment_info (
    comment_id    BIGINT,
    snapshot_date STRING,
    bvid          STRING,
    aid           BIGINT,
    user_id       BIGINT,
    user_name     STRING,
    content       STRING,
    like_count    INT,
    reply_count   INT,
    comment_time  BIGINT
)
STORED AS PARQUET;

-- ADS: Danmaku data for application layer
CREATE TABLE IF NOT EXISTS ads_danmaku_info (
    danmaku_id    BIGINT,
    snapshot_date STRING,
    bvid          STRING,
    cid           BIGINT,
    content       STRING,
    video_time    DOUBLE,
    mode          INT,
    fontsize      INT,
    color         BIGINT,
    mid_hash      STRING,
    send_time     BIGINT
)
STORED AS PARQUET;

-- ADS: Snapshot reference table for batch tracking
CREATE TABLE IF NOT EXISTS ads_snapshot_dates (
    snapshot_date  STRING,
    video_count    BIGINT,
    comment_count  BIGINT,
    danmaku_count  BIGINT
)
STORED AS PARQUET;
