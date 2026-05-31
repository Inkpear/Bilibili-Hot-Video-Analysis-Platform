-- ============================================================================
-- Hive ODS 外部表 DDL
-- 指向 HDFS /data/bilibili/ods/ 目录，使用 JsonSerDe 解析 NDJSON
-- ============================================================================

-- 创建数据库
CREATE DATABASE IF NOT EXISTS bilibili;
USE bilibili;

-- ----------------------------------------------------------------------------
-- ods_rank_video_raw: 榜单视频快照（含互动指标 + 上下文字段）
-- ----------------------------------------------------------------------------
DROP TABLE IF EXISTS ods_rank_video_raw;
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

-- ----------------------------------------------------------------------------
-- ods_video_comment_raw: 评论原始数据（采集时已关联 bvid）
-- ----------------------------------------------------------------------------
DROP TABLE IF EXISTS ods_video_comment_raw;
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

-- ----------------------------------------------------------------------------
-- ods_video_danmaku_raw: 弹幕原始数据（采集时已关联 bvid 和 cid）
-- ----------------------------------------------------------------------------
DROP TABLE IF EXISTS ods_video_danmaku_raw;
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

-- ----------------------------------------------------------------------------
-- ods_up_raw: UP 主原始数据
-- ----------------------------------------------------------------------------
DROP TABLE IF EXISTS ods_up_raw;
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
