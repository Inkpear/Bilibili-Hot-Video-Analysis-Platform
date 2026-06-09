-- scripts/init_db.sql
CREATE DATABASE IF NOT EXISTS bilibili DEFAULT CHARACTER SET utf8mb4;
USE bilibili;

-- 视频主表
CREATE TABLE hot_video (
    bvid VARCHAR(20) NOT NULL,
    snapshot_date VARCHAR(8) NOT NULL,
    aid BIGINT,
    cid BIGINT,
    title VARCHAR(500),
    description TEXT,
    tags VARCHAR(1000),
    sub_category VARCHAR(50),
    category VARCHAR(100),
    duration INT,
    publish_time BIGINT,
    up_id VARCHAR(20),
    up_name VARCHAR(100),
    up_avatar VARCHAR(500),
    view_count BIGINT DEFAULT 0,
    like_count BIGINT DEFAULT 0,
    coin_count BIGINT DEFAULT 0,
    favorite_count BIGINT DEFAULT 0,
    share_count BIGINT DEFAULT 0,
    reply_count BIGINT DEFAULT 0,
    danmaku_count BIGINT DEFAULT 0,
    rank_name VARCHAR(100),
    rank_position INT,
    rank_source VARCHAR(50),
    url VARCHAR(500),
    snapshot_id VARCHAR(50),
    collect_time BIGINT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (bvid, snapshot_date)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- UP主信息表
CREATE TABLE up_info (
    up_id VARCHAR(50) NOT NULL,
    snapshot_date VARCHAR(8) NOT NULL,
    up_name VARCHAR(200),
    up_avatar VARCHAR(500),
    follower_count INT DEFAULT 0,
    video_count INT DEFAULT 0,
    sign TEXT,
    official_verify VARCHAR(200),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (up_id, snapshot_date)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 互动指标表
CREATE TABLE video_interaction (
    bvid VARCHAR(20) NOT NULL,
    snapshot_date VARCHAR(8) NOT NULL,
    like_rate DECIMAL(10, 6),
    coin_rate DECIMAL(10, 6),
    fav_rate DECIMAL(10, 6),
    reply_rate DECIMAL(10, 6),
    share_rate DECIMAL(10, 6),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (bvid, snapshot_date)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 评论表
CREATE TABLE comment_info (
    comment_id BIGINT NOT NULL,
    snapshot_date VARCHAR(8) NOT NULL,
    bvid VARCHAR(20),
    aid BIGINT,
    user_id BIGINT,
    user_name VARCHAR(100),
    content TEXT,
    like_count INT DEFAULT 0,
    reply_count INT DEFAULT 0,
    comment_time BIGINT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (comment_id, snapshot_date)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 弹幕表
CREATE TABLE danmaku_info (
    danmaku_id BIGINT NOT NULL,
    snapshot_date VARCHAR(8) NOT NULL,
    bvid VARCHAR(20),
    cid BIGINT,
    content VARCHAR(1000),
    video_time DECIMAL(10, 3),
    mode INT,
    fontsize INT,
    color BIGINT,
    mid_hash VARCHAR(50),
    send_time BIGINT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (danmaku_id, snapshot_date)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 词云表（标题）
CREATE TABLE word_cloud_title (
    bvid VARCHAR(20) NOT NULL,
    word VARCHAR(100) NOT NULL,
    snapshot_date VARCHAR(8) NOT NULL,
    word_count INT DEFAULT 1,
    category VARCHAR(50),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (bvid, word, snapshot_date)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 词云表（评论）
CREATE TABLE word_cloud_comment (
    bvid VARCHAR(20) NOT NULL,
    word VARCHAR(100) NOT NULL,
    snapshot_date VARCHAR(8) NOT NULL,
    word_count INT DEFAULT 1,
    category VARCHAR(50),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (bvid, word, snapshot_date)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 词云表（弹幕）
CREATE TABLE word_cloud_danmaku (
    bvid VARCHAR(20) NOT NULL,
    word VARCHAR(100) NOT NULL,
    snapshot_date VARCHAR(8) NOT NULL,
    word_count INT DEFAULT 1,
    category VARCHAR(50),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (bvid, word, snapshot_date)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 词频趋势表
CREATE TABLE word_trend (
    word VARCHAR(100) NOT NULL,
    snapshot_date VARCHAR(8) NOT NULL,
    category VARCHAR(50) NOT NULL,
    source_type VARCHAR(20) NOT NULL,  -- title/comment/danmaku
    total_count BIGINT DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (word, snapshot_date, category, source_type)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 快照日期表
CREATE TABLE snapshot_dates (
    snapshot_date VARCHAR(8) NOT NULL,
    video_count INT DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (snapshot_date)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
