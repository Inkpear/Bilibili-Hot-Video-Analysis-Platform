-- scripts/hive_ddl.sql
CREATE DATABASE IF NOT EXISTS bilibili;

-- ODS层：原始视频数据
CREATE EXTERNAL TABLE IF NOT EXISTS bilibili.ods_video (
    bvid STRING,
    aid BIGINT,
    cid BIGINT,
    title STRING,
    description STRING,
    tags STRING,
    sub_category STRING,
    duration INT,
    publish_time BIGINT,
    up_id STRING,
    up_name STRING,
    up_avatar STRING,
    view_count BIGINT,
    like_count BIGINT,
    coin_count BIGINT,
    favorite_count BIGINT,
    share_count BIGINT,
    reply_count BIGINT,
    danmaku_count BIGINT
)
PARTITIONED BY (date STRING)
ROW FORMAT SERDE 'org.apache.hive.hcatalog.data.JsonSerDe'
LOCATION '/bilibili/raw/video';

-- ODS层：原始评论数据
CREATE EXTERNAL TABLE IF NOT EXISTS bilibili.ods_comment (
    comment_id BIGINT,
    bvid STRING,
    aid BIGINT,
    user_id BIGINT,
    user_name STRING,
    content STRING,
    like_count INT,
    reply_count INT,
    comment_time BIGINT
)
PARTITIONED BY (date STRING)
ROW FORMAT SERDE 'org.apache.hive.hcatalog.data.JsonSerDe'
LOCATION '/bilibili/raw/comment';

-- DWD层：清洗后的视频数据
CREATE TABLE IF NOT EXISTS bilibili.dwd_video_clean
LIKE bilibili.ods_video;

-- DWS层：分区统计
CREATE TABLE IF NOT EXISTS bilibili.dws_partition_stats (
    sub_category STRING,
    video_count INT,
    total_views BIGINT,
    total_likes BIGINT,
    avg_like_rate DOUBLE
)
PARTITIONED BY (date STRING);

-- ADS层：供API查询
CREATE TABLE IF NOT EXISTS bilibili.ads_hot_video
LIKE bilibili.dwd_video_clean;
