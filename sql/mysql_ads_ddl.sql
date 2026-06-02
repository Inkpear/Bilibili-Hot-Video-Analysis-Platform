-- ============================================================================
-- MySQL ADS 表 DDL
-- 与 API 响应字段一一对应，存储 Spark ETL 最终结果
-- ============================================================================

CREATE DATABASE IF NOT EXISTS bilibili DEFAULT CHARSET utf8mb4;
USE bilibili;

-- ----------------------------------------------------------------------------
-- ads_screen_overview: 大屏总览（单行表）
-- ----------------------------------------------------------------------------
DROP TABLE IF EXISTS ads_screen_overview;
CREATE TABLE ads_screen_overview (
    video_count          INT,
    category_count       INT,
    up_count             INT,
    total_views          BIGINT,
    total_interactions   BIGINT,
    latest_snapshot_date VARCHAR(8),
    update_time          DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- ----------------------------------------------------------------------------
-- ads_hot_video_rank: 热门视频排行
-- ----------------------------------------------------------------------------
DROP TABLE IF EXISTS ads_hot_video_rank;
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

-- ----------------------------------------------------------------------------
-- ads_partition_compare: 分区竞争力
-- ----------------------------------------------------------------------------
DROP TABLE IF EXISTS ads_partition_compare;
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

-- ----------------------------------------------------------------------------
-- ads_video_lifecycle: 视频生命周期
-- ----------------------------------------------------------------------------
DROP TABLE IF EXISTS ads_video_lifecycle;
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

-- ----------------------------------------------------------------------------
-- ads_partition_daily: 分区每日汇总
-- ----------------------------------------------------------------------------
DROP TABLE IF EXISTS ads_partition_daily;
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

-- ----------------------------------------------------------------------------
-- ads_up_summary: UP主汇总
-- ----------------------------------------------------------------------------
DROP TABLE IF EXISTS ads_up_summary;
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

-- ----------------------------------------------------------------------------
-- ads_word_cloud: 统一词云（标题/评论/弹幕）
-- ----------------------------------------------------------------------------
DROP TABLE IF EXISTS ads_word_cloud;
CREATE TABLE ads_word_cloud (
    id          INT AUTO_INCREMENT PRIMARY KEY,
    source_type VARCHAR(32),
    category    VARCHAR(64),
    word        VARCHAR(64),
    word_count  INT,
    update_time DATETIME DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_source_category (source_type, category),
    INDEX idx_word (word)
);

-- ----------------------------------------------------------------------------
-- ads_hot_comments: 热门评论
-- ----------------------------------------------------------------------------
DROP TABLE IF EXISTS ads_hot_comments;
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

-- 验证
SHOW TABLES;
