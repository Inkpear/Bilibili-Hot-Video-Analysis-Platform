# B站热门视频分析大屏 v2 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 构建一个B站热门视频分析大屏，包含Rust爬虫、大数据ETL、Rust后端API、React前端大屏

**Architecture:** 爬虫采集NDJSON → HDFS存储 → Spark/Hive ETL → MySQL服务层 → Rust Axum API → React+ECharts前端

**Tech Stack:** Rust, React, TypeScript, ECharts, Tailwind CSS, MySQL, Hadoop, Spark, Hive

---

## 阶段概览

| 阶段 | 内容 | 依赖 |
|------|------|------|
| 1 | 项目结构 + MySQL初始化 | 无 |
| 2 | Rust爬虫 | 阶段1 |
| 3 | ETL数据处理 | 阶段2 |
| 4 | Rust后端API | 阶段3 |
| 5 | React前端 | 阶段4 |
| 6 | 大数据集成 | 阶段3 |
| 7 | 部署 | 阶段4+5 |

---

## 阶段1：项目结构 + MySQL初始化

### Task 1.1: 创建项目目录结构

**Files:**
- Create: `crawler/` (Rust爬虫)
- Create: `backend/` (Rust后端)
- Create: `frontend/` (React前端)
- Create: `etl/` (ETL脚本)
- Create: `docs/` (文档)
- Create: `scripts/` (部署脚本)

- [ ] **Step 1: 创建目录结构**

```bash
mkdir -p crawler backend frontend etl docs scripts
```

- [ ] **Step 2: 初始化Git仓库**

```bash
git init
echo "node_modules/
target/
dist/
*.log
.DS_Store" > .gitignore
```

- [ ] **Step 3: 提交**

```bash
git add .
git commit -m "chore: init project structure"
```

### Task 1.2: MySQL数据库初始化

**Files:**
- Create: `scripts/init_db.sql`

- [ ] **Step 1: 创建数据库DDL**

```sql
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
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (bvid, snapshot_date)
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
```

- [ ] **Step 2: 执行DDL**

```bash
mysql -u root -p < scripts/init_db.sql
```

- [ ] **Step 3: 提交**

```bash
git add scripts/init_db.sql
git commit -m "feat: add MySQL DDL for all tables"
```

---

## 阶段2：Rust爬虫

### Task 2.1: 爬虫项目初始化

**Files:**
- Create: `crawler/Cargo.toml`
- Create: `crawler/src/main.rs`

- [ ] **Step 1: 创建Cargo.toml**

```toml
# crawler/Cargo.toml
[package]
name = "bilibili-crawler"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = "0.4"
tracing = "0.1"
tracing-subscriber = "0.3"
anyhow = "1"
toml = "0.8"
```

- [ ] **Step 2: 创建main.rs骨架**

```rust
// crawler/src/main.rs
use anyhow::Result;
use tracing::{info, error};

mod config;
mod api;
mod models;
mod storage;
mod scheduler;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::init();
    info!("Bilibili Crawler starting...");
    
    let config = config::Config::load()?;
    
    // TODO: 根据参数决定全量/增量模式
    scheduler::run_full_crawl(&config).await?;
    
    Ok(())
}
```

- [ ] **Step 3: 提交**

```bash
git add crawler/
git commit -m "feat: init Rust crawler project"
```

### Task 2.2: 配置模块

**Files:**
- Create: `crawler/src/config.rs`
- Create: `crawler/config.toml`

- [ ] **Step 1: 创建配置结构体**

```rust
// crawler/src/config.rs
use serde::Deserialize;
use anyhow::Result;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub crawler: CrawlerConfig,
    pub api: ApiConfig,
}

#[derive(Debug, Deserialize)]
pub struct CrawlerConfig {
    pub interval: u64,
    pub concurrency: usize,
    pub max_retries: u32,
    pub output_dir: String,
}

#[derive(Debug, Deserialize)]
pub struct ApiConfig {
    pub base_url: String,
    pub user_agent: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        let content = std::fs::read_to_string("config.toml")?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}
```

- [ ] **Step 2: 创建配置文件**

```toml
# crawler/config.toml
[crawler]
interval = 600
concurrency = 5
max_retries = 3
output_dir = "~/output"

[api]
base_url = "https://api.bilibili.com"
user_agent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
```

- [ ] **Step 3: 提交**

```bash
git add crawler/src/config.rs crawler/config.toml
git commit -m "feat: add crawler config module"
```

### Task 2.3: 数据模型

**Files:**
- Create: `crawler/src/models/mod.rs`
- Create: `crawler/src/models/video.rs`
- Create: `crawler/src/models/comment.rs`
- Create: `crawler/src/models/danmaku.rs`

- [ ] **Step 1: 创建视频模型**

```rust
// crawler/src/models/video.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoDetail {
    pub bvid: String,
    pub aid: i64,
    pub cid: i64,
    pub title: String,
    pub description: String,
    pub tags: String,
    pub sub_category: String,
    pub duration: i32,
    pub publish_time: i64,
    pub up_id: String,
    pub up_name: String,
    pub up_avatar: String,
    pub view_count: i64,
    pub like_count: i64,
    pub coin_count: i64,
    pub favorite_count: i64,
    pub share_count: i64,
    pub reply_count: i64,
    pub danmaku_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpInfo {
    pub up_id: String,
    pub up_name: String,
    pub up_avatar: String,
    pub follower_count: i64,
    pub video_count: i64,
    pub sign: String,
    pub official_verify: String,
}
```

- [ ] **Step 2: 创建评论模型**

```rust
// crawler/src/models/comment.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub comment_id: i64,
    pub bvid: String,
    pub aid: i64,
    pub user_id: i64,
    pub user_name: String,
    pub content: String,
    pub like_count: i32,
    pub reply_count: i32,
    pub comment_time: i64,
}
```

- [ ] **Step 3: 创建弹幕模型**

```rust
// crawler/src/models/danmaku.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Danmaku {
    pub danmaku_id: i64,
    pub bvid: String,
    pub cid: i64,
    pub content: String,
    pub video_time: f64,
    pub mode: i32,
    pub fontsize: i32,
    pub color: i64,
    pub mid_hash: String,
    pub send_time: i64,
}
```

- [ ] **Step 4: 创建mod.rs**

```rust
// crawler/src/models/mod.rs
pub mod video;
pub mod comment;
pub mod danmaku;

pub use video::{VideoDetail, UpInfo};
pub use comment::Comment;
pub use danmaku::Danmaku;
```

- [ ] **Step 5: 提交**

```bash
git add crawler/src/models/
git commit -m "feat: add crawler data models"
```

### Task 2.4: API客户端

**Files:**
- Create: `crawler/src/api/mod.rs`
- Create: `crawler/src/api/hot.rs`
- Create: `crawler/src/api/video.rs`
- Create: `crawler/src/api/comment.rs`
- Create: `crawler/src/api/danmaku.rs`

- [ ] **Step 1: 创建热门视频API**

```rust
// crawler/src/api/hot.rs
use reqwest::Client;
use serde::Deserialize;
use anyhow::Result;
use tracing::info;

#[derive(Debug, Deserialize)]
struct HotResponse {
    code: i32,
    data: HotData,
}

#[derive(Debug, Deserialize)]
struct HotData {
    list: Vec<HotVideo>,
}

#[derive(Debug, Deserialize)]
struct HotVideo {
    bvid: String,
}

pub async fn fetch_hot_videos(client: &Client, base_url: &str) -> Result<Vec<String>> {
    let url = format!("{}/x/web-interface/popular", base_url);
    let resp: HotResponse = client.get(&url).send().await?.json().await?;
    
    if resp.code != 0 {
        anyhow::bail!("API error: code={}", resp.code);
    }
    
    let bvids: Vec<String> = resp.data.list.iter().map(|v| v.bvid.clone()).collect();
    info!("Fetched {} hot videos", bvids.len());
    
    Ok(bvids)
}
```

- [ ] **Step 2: 创建视频详情API**

```rust
// crawler/src/api/video.rs
use reqwest::Client;
use serde::Deserialize;
use anyhow::Result;
use crate::models::VideoDetail;

#[derive(Debug, Deserialize)]
struct VideoResponse {
    code: i32,
    data: VideoData,
}

#[derive(Debug, Deserialize)]
struct VideoData {
    bvid: String,
    aid: i64,
    cid: i64,
    title: String,
    desc: String,
    // ... 其他字段
}

pub async fn fetch_video_detail(client: &Client, base_url: &str, bvid: &str) -> Result<VideoDetail> {
    let url = format!("{}/x/web-interface/view?bvid={}", base_url, bvid);
    let resp: VideoResponse = client.get(&url).send().await?.json().await?;
    
    if resp.code != 0 {
        anyhow::bail!("API error: code={}", resp.code);
    }
    
    // 转换为VideoDetail
    // TODO: 完整实现
    todo!()
}
```

- [ ] **Step 3: 创建评论API**

```rust
// crawler/src/api/comment.rs
use reqwest::Client;
use serde::Deserialize;
use anyhow::Result;
use crate::models::Comment;

pub async fn fetch_comments(
    client: &Client, 
    base_url: &str, 
    bvid: &str, 
    aid: i64,
    limit: usize
) -> Result<Vec<Comment>> {
    let url = format!(
        "{}/x/v2/reply?type=1&oid={}&pn=1&ps={}", 
        base_url, aid, limit
    );
    
    // TODO: 完整实现
    todo!()
}
```

- [ ] **Step 4: 创建弹幕API**

```rust
// crawler/src/api/danmaku.rs
use reqwest::Client;
use anyhow::Result;
use crate::models::Danmaku;

pub async fn fetch_danmaku(
    client: &Client, 
    base_url: &str, 
    cid: i64,
    limit: usize
) -> Result<Vec<Danmaku>> {
    // 弹幕API需要先获取oid，然后请求protobuf格式
    // TODO: 完整实现
    todo!()
}
```

- [ ] **Step 5: 创建mod.rs**

```rust
// crawler/src/api/mod.rs
pub mod hot;
pub mod video;
pub mod comment;
pub mod danmaku;
```

- [ ] **Step 6: 提交**

```bash
git add crawler/src/api/
git commit -m "feat: add crawler API clients"
```

### Task 2.5: 存储模块

**Files:**
- Create: `crawler/src/storage/mod.rs`
- Create: `crawler/src/storage/ndjson.rs`

- [ ] **Step 1: 创建NDJSON写入器**

```rust
// crawler/src/storage/ndjson.rs
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;
use serde::Serialize;
use anyhow::Result;
use chrono::Local;

pub struct NdjsonWriter {
    output_dir: String,
}

impl NdjsonWriter {
    pub fn new(output_dir: &str) -> Self {
        Self {
            output_dir: output_dir.to_string(),
        }
    }
    
    pub fn create_snapshot_dir(&self) -> Result<String> {
        let now = Local::now();
        let dir_name = now.format("%Y%m%d_%H%M").to_string();
        let dir_path = format!("{}/{}", self.output_dir, dir_name);
        fs::create_dir_all(&dir_path)?;
        fs::create_dir_all(format!("{}/comments", dir_path))?;
        fs::create_dir_all(format!("{}/danmaku", dir_path))?;
        Ok(dir_path)
    }
    
    pub fn write_ndjson<T: Serialize>(&self, path: &str, data: &[T]) -> Result<()> {
        let mut file = File::create(path)?;
        for item in data {
            let json = serde_json::to_string(item)?;
            writeln!(file, "{}", json)?;
        }
        Ok(())
    }
    
    pub fn append_ndjson<T: Serialize>(&self, path: &str, data: &[T]) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        for item in data {
            let json = serde_json::to_string(item)?;
            writeln!(file, "{}", json)?;
        }
        Ok(())
    }
}
```

- [ ] **Step 2: 提交**

```bash
git add crawler/src/storage/
git commit -m "feat: add NDJSON storage module"
```

### Task 2.6: 调度器

**Files:**
- Create: `crawler/src/scheduler/mod.rs`
- Create: `crawler/src/scheduler/full.rs`
- Create: `crawler/src/scheduler/incremental.rs`

- [ ] **Step 1: 创建全量采集调度器**

```rust
// crawler/src/scheduler/full.rs
use crate::config::Config;
use crate::api::{hot, video, comment, danmaku};
use crate::storage::ndjson::NdjsonWriter;
use crate::models::*;
use anyhow::Result;
use tracing::{info, error};
use std::sync::Arc;
use tokio::sync::Semaphore;

pub async fn run_full_crawl(config: &Config) -> Result<()> {
    info!("Starting full crawl...");
    
    let client = reqwest::Client::new();
    let writer = NdjsonWriter::new(&config.crawler.output_dir);
    let snapshot_dir = writer.create_snapshot_dir()?;
    let semaphore = Arc::new(Semaphore::new(config.crawler.concurrency));
    
    // 1. 获取热门视频列表
    let bvids = hot::fetch_hot_videos(&client, &config.api.base_url).await?;
    info!("Found {} hot videos", bvids.len());
    
    // 2. 并发采集每个视频
    let mut handles = vec![];
    for bvid in bvids {
        let client = client.clone();
        let base_url = config.api.base_url.clone();
        let snapshot_dir = snapshot_dir.clone();
        let sem = semaphore.clone();
        
        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            process_video(&client, &base_url, &snapshot_dir, &bvid).await
        });
        handles.push(handle);
    }
    
    // 等待所有任务完成
    for handle in handles {
        if let Err(e) = handle.await? {
            error!("Task failed: {}", e);
        }
    }
    
    info!("Full crawl completed!");
    Ok(())
}

async fn process_video(
    client: &reqwest::Client,
    base_url: &str,
    snapshot_dir: &str,
    bvid: &str,
) -> Result<()> {
    // 采集视频详情
    let video_detail = video::fetch_video_detail(client, base_url, bvid).await?;
    
    // 采集UP主信息
    let up_info = video::fetch_up_info(client, base_url, &video_detail.up_id).await?;
    
    // 采集评论
    let comments = comment::fetch_comments(client, base_url, bvid, video_detail.aid, 200).await?;
    
    // 采集弹幕
    let danmaku_list = danmaku::fetch_danmaku(client, base_url, video_detail.cid, 3000).await?;
    
    // 写入文件
    let writer = NdjsonWriter::new(snapshot_dir);
    // TODO: 写入逻辑
    
    Ok(())
}
```

- [ ] **Step 2: 提交**

```bash
git add crawler/src/scheduler/
git commit -m "feat: add crawler scheduler"
```

- [ ] **Step 3: 添加错误处理模块**

```rust
// crawler/src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CrawlerError {
    #[error("API error: {0}")]
    ApiError(String),
    
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}
```

- [ ] **Step 4: 提交**

```bash
git add crawler/src/error.rs
git commit -m "feat: add error handling module"
```

---

## 阶段3：ETL数据处理

### Task 3.1: ETL项目初始化

**Files:**
- Create: `etl/requirements.txt`
- Create: `etl/main.py`

- [ ] **Step 1: 创建requirements.txt**

```
# etl/requirements.txt
pymysql>=1.1.0
jieba>=0.42.1
pandas>=2.0.0
```

- [ ] **Step 2: 创建主入口**

```python
# etl/main.py
import sys
from datetime import datetime, timedelta
from processor import DataProcessor
from word_segment import WordSegmenter
from db import Database

def main():
    # 默认处理前一天的数据
    if len(sys.argv) > 1:
        date_str = sys.argv[1]
    else:
        yesterday = datetime.now() - timedelta(days=1)
        date_str = yesterday.strftime("%Y%m%d")
    
    print(f"Processing data for {date_str}...")
    
    # 1. 找到最新的快照目录
    # 2. 读取并清洗数据
    # 3. 计算衍生指标
    # 4. 分词计算词云
    # 5. 写入MySQL
    
    processor = DataProcessor(date_str)
    processor.run()

if __name__ == "__main__":
    main()
```

- [ ] **Step 3: 提交**

```bash
git add etl/
git commit -m "feat: init ETL project"
```

### Task 3.2: 数据处理器

**Files:**
- Create: `etl/processor.py`

- [ ] **Step 1: 创建数据处理器**

```python
# etl/processor.py
import json
import os
import glob
from datetime import datetime
from db import Database

class DataProcessor:
    def __init__(self, date_str: str):
        self.date_str = date_str
        self.db = Database()
        self.output_dir = os.path.expanduser("~/output")
        
    def find_latest_snapshot(self) -> str:
        """找到指定日期的最新快照目录"""
        pattern = f"{self.output_dir}/{self.date_str}_*"
        dirs = sorted(glob.glob(pattern))
        if not dirs:
            raise FileNotFoundError(f"No snapshot found for {self.date_str}")
        return dirs[-1]
    
    def load_video_details(self, snapshot_dir: str) -> list:
        """加载视频详情"""
        path = f"{snapshot_dir}/video_detail.ndjson"
        videos = []
        with open(path, 'r') as f:
            for line in f:
                if line.strip():
                    videos.append(json.loads(line))
        return videos
    
    def calculate_interaction_rates(self, video: dict) -> dict:
        """计算互动指标"""
        view_count = video.get('view_count', 0)
        if view_count == 0:
            return {
                'like_rate': 0, 'coin_rate': 0, 'fav_rate': 0,
                'reply_rate': 0, 'share_rate': 0
            }
        return {
            'like_rate': video['like_count'] / view_count,
            'coin_rate': video['coin_count'] / view_count,
            'fav_rate': video['favorite_count'] / view_count,
            'reply_rate': video['reply_count'] / view_count,
            'share_rate': video['share_count'] / view_count,
        }
    
    def run(self):
        """执行ETL流程"""
        snapshot_dir = self.find_latest_snapshot()
        print(f"Processing snapshot: {snapshot_dir}")
        
        # 1. 加载视频数据
        videos = self.load_video_details(snapshot_dir)
        print(f"Loaded {len(videos)} videos")
        
        # 2. 写入hot_video表
        for video in videos:
            self.db.insert_video(video, self.date_str)
        
        # 3. 计算并写入互动指标
        for video in videos:
            rates = self.calculate_interaction_rates(video)
            self.db.insert_interaction(video['bvid'], self.date_str, rates)
        
        # 4. 处理评论和弹幕
        # TODO
        
        print("ETL completed!")
```

- [ ] **Step 2: 提交**

```bash
git add etl/processor.py
git commit -m "feat: add data processor"
```

### Task 3.3: 词云分词

**Files:**
- Create: `etl/word_segment.py`

- [ ] **Step 1: 创建分词器**

```python
# etl/word_segment.py
import jieba
import re
from collections import Counter

class WordSegmenter:
    def __init__(self):
        # 停用词表
        self.stopwords = set([
            '的', '了', '在', '是', '我', '有', '和', '就', '不', '人',
            '都', '一', '一个', '上', '也', '很', '到', '说', '要', '去',
            '你', '会', '着', '没有', '看', '好', '自己', '这', '他', '她',
            '哈哈哈', '666', '来了', '好的', '可以', '什么', '怎么',
            '视频', '播放', '投稿', '热门', '合集', '完整版', '全程',
        ])
        
    def clean_text(self, text: str) -> str:
        """清洗文本"""
        # 移除emoji
        text = re.sub(r'[\U00010000-\U0010ffff]', '', text)
        # 移除特殊字符
        text = re.sub(r'[^\w\s]', '', text)
        return text.strip()
    
    def segment(self, text: str) -> list[str]:
        """分词并过滤"""
        text = self.clean_text(text)
        words = jieba.lcut(text)
        # 过滤停用词和短词
        words = [w for w in words if w not in self.stopwords and len(w) > 1]
        return words
    
    def segment_batch(self, texts: list[str]) -> Counter:
        """批量分词并统计词频"""
        counter = Counter()
        for text in texts:
            words = self.segment(text)
            counter.update(words)
        return counter
    
    def get_top_words(self, texts: list[str], top_n: int = 100) -> list[tuple[str, int]]:
        """获取TOP N高频词"""
        counter = self.segment_batch(texts)
        return counter.most_common(top_n)
```

- [ ] **Step 2: 提交**

```bash
git add etl/word_segment.py
git commit -m "feat: add word segmentation module"
```

### Task 3.4: 数据库操作

**Files:**
- Create: `etl/db.py`

- [ ] **Step 1: 创建数据库操作类**

```python
# etl/db.py
import pymysql
import os

class Database:
    def __init__(self):
        self.conn = pymysql.connect(
            host=os.getenv('DB_HOST', 'localhost'),
            user=os.getenv('DB_USER', 'root'),
            password=os.getenv('DB_PASSWORD', ''),
            database=os.getenv('DB_NAME', 'bilibili'),
            charset='utf8mb4'
        )
    
    def insert_video(self, video: dict, snapshot_date: str):
        """插入视频数据"""
        sql = """
        INSERT INTO hot_video 
        (bvid, snapshot_date, aid, cid, title, description, tags, sub_category,
         duration, publish_time, up_id, up_name, up_avatar,
         view_count, like_count, coin_count, favorite_count, 
         share_count, reply_count, danmaku_count)
        VALUES (%s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s)
        ON DUPLICATE KEY UPDATE
        view_count=VALUES(view_count), like_count=VALUES(like_count),
        coin_count=VALUES(coin_count), favorite_count=VALUES(favorite_count),
        share_count=VALUES(share_count), reply_count=VALUES(reply_count),
        danmaku_count=VALUES(danmaku_count)
        """
        # TODO: 执行SQL
    
    def insert_interaction(self, bvid: str, snapshot_date: str, rates: dict):
        """插入互动指标"""
        sql = """
        INSERT INTO video_interaction 
        (bvid, snapshot_date, like_rate, coin_rate, fav_rate, reply_rate, share_rate)
        VALUES (%s, %s, %s, %s, %s, %s, %s)
        ON DUPLICATE KEY UPDATE
        like_rate=VALUES(like_rate), coin_rate=VALUES(coin_rate),
        fav_rate=VALUES(fav_rate), reply_rate=VALUES(reply_rate),
        share_rate=VALUES(share_rate)
        """
        # TODO: 执行SQL
    
    def insert_word_cloud(self, table: str, bvid: str, word: str, 
                          snapshot_date: str, count: int, category: str):
        """插入词云数据"""
        sql = f"""
        INSERT INTO {table} 
        (bvid, word, snapshot_date, word_count, category)
        VALUES (%s, %s, %s, %s, %s)
        ON DUPLICATE KEY UPDATE word_count=VALUES(word_count)
        """
        # TODO: 执行SQL
    
    def close(self):
        self.conn.close()
```

- [ ] **Step 2: 提交**

```bash
git add etl/db.py
git commit -m "feat: add database operations module"
```

---

## 阶段4：Rust后端API

### Task 4.1: 后端项目初始化

**Files:**
- Create: `backend/Cargo.toml`
- Create: `backend/src/main.rs`

- [ ] **Step 1: 创建Cargo.toml**

```toml
# backend/Cargo.toml
[package]
name = "bilibili-backend"
version = "0.1.0"
edition = "2021"

[dependencies]
axum = "0.7"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sqlx = { version = "0.7", features = ["runtime-tokio", "mysql"] }
tower-http = { version = "0.5", features = ["cors"] }
tracing = "0.1"
tracing-subscriber = "0.3"
anyhow = "1"
```

- [ ] **Step 2: 创建main.rs**

```rust
// backend/src/main.rs
use axum::{Router, routing::get};
use tower_http::cors::CorsLayer;
use sqlx::MySqlPool;
use std::sync::Arc;

mod handlers;
mod models;

pub struct AppState {
    db: MySqlPool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::init();
    
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "mysql://root:password@localhost/bilibili".to_string());
    
    let pool = MySqlPool::connect(&database_url).await?;
    
    let state = Arc::new(AppState { db: pool });
    
    let app = Router::new()
        .route("/api/overview", get(handlers::overview))
        .route("/api/partitions", get(handlers::partitions))
        .route("/api/partition-heatmap", get(handlers::partition_heatmap))
        .route("/api/trend", get(handlers::trend))
        .route("/api/word-cloud", get(handlers::word_cloud))
        .route("/api/videos", get(handlers::videos))
        .route("/api/video/{bvid}", get(handlers::video_detail))
        .route("/api/word-trend", get(handlers::word_trend))
        .route("/api/snapshot-dates", get(handlers::snapshot_dates))
        .layer(CorsLayer::permissive())
        .with_state(state);
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:9090").await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}
```

- [ ] **Step 3: 提交**

```bash
git add backend/
git commit -m "feat: init Rust backend project"
```

### Task 4.2: 数据模型

**Files:**
- Create: `backend/src/models.rs`

- [ ] **Step 1: 创建API响应模型**

```rust
// backend/src/models.rs
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub code: i32,
    pub message: String,
    pub data: T,
}

#[derive(Debug, Serialize)]
pub struct OverviewData {
    pub total_views: i64,
    pub total_likes: i64,
    pub partition_count: i32,
    pub hot_title: String,
    pub new_video_count: i32,
}

#[derive(Debug, Serialize)]
pub struct PartitionHeatmapData {
    pub categories: Vec<String>,
    pub dates: Vec<String>,
    pub values: Vec<Vec<f64>>,
}

#[derive(Debug, Serialize)]
pub struct VideoListItem {
    pub bvid: String,
    pub title: String,
    pub up_name: String,
    pub category: String,
    pub view_count: i64,
    pub like_count: i64,
}

#[derive(Debug, Serialize)]
pub struct WordCloudItem {
    pub word: String,
    pub count: i32,
}
```

- [ ] **Step 2: 提交**

```bash
git add backend/src/models.rs
git commit -m "feat: add API response models"
```

### Task 4.3: API处理器

**Files:**
- Create: `backend/src/handlers.rs`

- [ ] **Step 1: 创建处理器**

```rust
// backend/src/handlers.rs
use axum::extract::{State, Query};
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;
use crate::AppState;
use crate::models::*;

#[derive(Debug, Deserialize)]
pub struct SnapshotDateQuery {
    pub snapshot_date: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CategoryQuery {
    pub category: Option<String>,
    pub snapshot_date: Option<String>,
}

pub async fn overview(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SnapshotDateQuery>,
) -> Json<ApiResponse<OverviewData>> {
    // TODO: 查询数据库
    Json(ApiResponse {
        code: 0,
        message: "ok".to_string(),
        data: OverviewData {
            total_views: 0,
            total_likes: 0,
            partition_count: 0,
            hot_title: "".to_string(),
            new_video_count: 0,
        },
    })
}

pub async fn partition_heatmap(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SnapshotDateQuery>,
) -> Json<ApiResponse<PartitionHeatmapData>> {
    // TODO: 查询数据库
    Json(ApiResponse {
        code: 0,
        message: "ok".to_string(),
        data: PartitionHeatmapData {
            categories: vec![],
            dates: vec![],
            values: vec![],
        },
    })
}

// TODO: 实现其他处理器
```

- [ ] **Step 2: 提交**

```bash
git add backend/src/handlers.rs
git commit -m "feat: add API handlers"
```

---

## 阶段5：React前端

### Task 5.1: 前端项目初始化

**Files:**
- Create: `frontend/package.json`
- Create: `frontend/vite.config.ts`
- Create: `frontend/tsconfig.json`

- [ ] **Step 1: 初始化项目**

```bash
cd frontend
npm create vite@latest . -- --template react-ts
npm install
npm install echarts echarts-for-react tailwindcss @tailwindcss/vite
```

- [ ] **Step 2: 配置Tailwind CSS**

```typescript
// frontend/vite.config.ts
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    proxy: {
      '/api': 'http://localhost:9090'
    }
  }
})
```

```css
/* frontend/src/index.css */
@import "tailwindcss";
```

- [ ] **Step 3: 提交**

```bash
git add frontend/
git commit -m "feat: init React frontend project"
```

### Task 5.2: 类型定义

**Files:**
- Create: `frontend/src/types/index.ts`

- [ ] **Step 1: 创建类型定义**

```typescript
// frontend/src/types/index.ts
export interface ApiResponse<T> {
  code: number;
  message: string;
  data: T;
}

export interface OverviewData {
  total_views: number;
  total_likes: number;
  partition_count: number;
  hot_title: string;
  new_video_count: number;
}

export interface PartitionHeatmapData {
  categories: string[];
  dates: string[];
  values: number[][];
}

export interface VideoListItem {
  bvid: string;
  title: string;
  up_name: string;
  category: string;
  view_count: number;
  like_count: number;
}

export interface WordCloudItem {
  word: string;
  count: number;
}

export interface DashboardState {
  selectedCategory: string | null;
  selectedBvid: string | null;
  snapshotDate: string | null;
}
```

- [ ] **Step 2: 提交**

```bash
git add frontend/src/types/
git commit -m "feat: add TypeScript type definitions"
```

### Task 5.3: 状态管理

**Files:**
- Create: `frontend/src/context/DashboardContext.tsx`

- [ ] **Step 1: 创建Context**

```typescript
// frontend/src/context/DashboardContext.tsx
import { createContext, useContext, useState, ReactNode } from 'react';
import { DashboardState } from '../types';

interface DashboardContextType {
  state: DashboardState;
  setSelectedCategory: (category: string | null) => void;
  setSelectedBvid: (bvid: string | null) => void;
  setSnapshotDate: (date: string | null) => void;
}

const DashboardContext = createContext<DashboardContextType | null>(null);

export function DashboardProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<DashboardState>({
    selectedCategory: null,
    selectedBvid: null,
    snapshotDate: null,
  });

  const setSelectedCategory = (category: string | null) => {
    setState(prev => ({ ...prev, selectedCategory: category }));
  };

  const setSelectedBvid = (bvid: string | null) => {
    setState(prev => ({ ...prev, selectedBvid: bvid }));
  };

  const setSnapshotDate = (date: string | null) => {
    setState(prev => ({ ...prev, snapshotDate: date }));
  };

  return (
    <DashboardContext.Provider value={{
      state,
      setSelectedCategory,
      setSelectedBvid,
      setSnapshotDate,
    }}>
      {children}
    </DashboardContext.Provider>
  );
}

export function useDashboard() {
  const context = useContext(DashboardContext);
  if (!context) {
    throw new Error('useDashboard must be used within DashboardProvider');
  }
  return context;
}
```

- [ ] **Step 2: 提交**

```bash
git add frontend/src/context/
git commit -m "feat: add dashboard context for state management"
```

### Task 5.4: API请求Hook

**Files:**
- Create: `frontend/src/hooks/useApi.ts`
- Create: `frontend/src/api/index.ts`

- [ ] **Step 1: 创建API模块**

```typescript
// frontend/src/api/index.ts
import { ApiResponse, OverviewData, PartitionHeatmapData, VideoListItem, WordCloudItem } from '../types';

const BASE_URL = '/api';

async function fetchApi<T>(path: string, params?: Record<string, string>): Promise<T> {
  const url = new URL(path, window.location.origin);
  if (params) {
    Object.entries(params).forEach(([key, value]) => {
      if (value) url.searchParams.append(key, value);
    });
  }
  
  const response = await fetch(url.toString());
  const data: ApiResponse<T> = await response.json();
  
  if (data.code !== 0) {
    throw new Error(data.message);
  }
  
  return data.data;
}

export async function fetchOverview(snapshotDate?: string): Promise<OverviewData> {
  return fetchApi(`${BASE_URL}/overview`, { snapshot_date: snapshotDate || '' });
}

export async function fetchPartitionHeatmap(snapshotDate?: string): Promise<PartitionHeatmapData> {
  return fetchApi(`${BASE_URL}/partition-heatmap`, { snapshot_date: snapshotDate || '' });
}

export async function fetchVideos(category?: string, snapshotDate?: string): Promise<VideoListItem[]> {
  return fetchApi(`${BASE_URL}/videos`, { 
    category: category || '', 
    snapshot_date: snapshotDate || '' 
  });
}

export async function fetchWordCloud(
  sourceType: string, 
  category?: string, 
  snapshotDate?: string
): Promise<WordCloudItem[]> {
  return fetchApi(`${BASE_URL}/word-cloud`, { 
    source_type: sourceType,
    category: category || '', 
    snapshot_date: snapshotDate || '' 
  });
}
```

- [ ] **Step 2: 创建useApi Hook**

```typescript
// frontend/src/hooks/useApi.ts
import { useState, useEffect } from 'react';

interface UseApiResult<T> {
  data: T | null;
  loading: boolean;
  error: Error | null;
  refetch: () => void;
}

export function useApi<T>(fetcher: () => Promise<T>, deps: unknown[]): UseApiResult<T> {
  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  const fetchData = async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await fetcher();
      setData(result);
    } catch (err) {
      setError(err as Error);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchData();
  }, deps);

  return { data, loading, error, refetch: fetchData };
}
```

- [ ] **Step 3: 提交**

```bash
git add frontend/src/api/ frontend/src/hooks/
git commit -m "feat: add API module and useApi hook"
```

### Task 5.5: 主布局组件

**Files:**
- Create: `frontend/src/App.tsx`
- Create: `frontend/src/components/Header.tsx`
- Create: `frontend/src/components/KPIStrip.tsx`

- [ ] **Step 1: 创建主布局**

```tsx
// frontend/src/App.tsx
import { DashboardProvider } from './context/DashboardContext';
import Header from './components/Header';
import KPIStrip from './components/KPIStrip';
import PartitionHeatmap from './components/PartitionHeatmap';
import TrendChart from './components/TrendChart';
import WordCloud from './components/WordCloud';
import VideoTable from './components/VideoTable';
import RadarChart from './components/RadarChart';
import WordTrendChart from './components/WordTrendChart';
import VideoDetailPanel from './components/VideoDetailPanel';
import { useDashboard } from './context/DashboardContext';

function DashboardContent() {
  const { state } = useDashboard();
  
  return (
    <div className="min-h-screen bg-gray-900 text-white">
      <Header />
      <KPIStrip />
      
      <div className="grid grid-cols-3 gap-4 p-4">
        <PartitionHeatmap />
        <TrendChart />
        <WordCloud />
        <VideoTable />
        <RadarChart />
        <WordTrendChart />
      </div>
      
      {state.selectedBvid && <VideoDetailPanel />}
    </div>
  );
}

export default function App() {
  return (
    <DashboardProvider>
      <DashboardContent />
    </DashboardProvider>
  );
}
```

- [ ] **Step 2: 创建Header组件**

```tsx
// frontend/src/components/Header.tsx
import { useDashboard } from '../context/DashboardContext';

export default function Header() {
  const { state, setSnapshotDate } = useDashboard();
  
  return (
    <header className="flex items-center justify-between px-6 py-3 bg-gray-800">
      <div className="flex items-center gap-3">
        <h1 className="text-xl font-bold bg-gradient-to-r from-pink-500 to-blue-500 bg-clip-text text-transparent">
          B站热门视频分析大屏
        </h1>
      </div>
      
      <div className="flex items-center gap-4">
        <input
          type="date"
          value={state.snapshotDate || ''}
          onChange={(e) => setSnapshotDate(e.target.value)}
          className="bg-gray-700 text-white px-3 py-1 rounded"
        />
      </div>
    </header>
  );
}
```

- [ ] **Step 3: 创建KPI横幅组件**

```tsx
// frontend/src/components/KPIStrip.tsx
import { useApi } from '../hooks/useApi';
import { fetchOverview } from '../api';
import { useDashboard } from '../context/DashboardContext';

export default function KPIStrip() {
  const { state } = useDashboard();
  const { data } = useApi(() => fetchOverview(state.snapshotDate || undefined), [state.snapshotDate]);
  
  if (!data) return <div className="h-24 bg-gray-800 animate-pulse" />;
  
  const items = [
    { label: '总播放量', value: formatNumber(data.total_views), color: 'text-blue-400' },
    { label: '总点赞数', value: formatNumber(data.total_likes), color: 'text-pink-400' },
    { label: '活跃分区', value: `${data.partition_count}个`, color: 'text-white' },
    { label: '今日热点', value: data.hot_title, color: 'text-yellow-400' },
    { label: '新增视频', value: `${data.new_video_count}个`, color: 'text-green-400' },
  ];
  
  return (
    <div className="grid grid-cols-5 gap-4 px-6 py-4 bg-gray-800/50">
      {items.map((item, i) => (
        <div key={i} className="text-center">
          <div className="text-sm text-gray-400">{item.label}</div>
          <div className={`text-2xl font-bold ${item.color}`}>{item.value}</div>
        </div>
      ))}
    </div>
  );
}

function formatNumber(num: number): string {
  if (num >= 100000000) return (num / 100000000).toFixed(1) + '亿';
  if (num >= 10000) return (num / 10000).toFixed(1) + '万';
  return num.toString();
}
```

- [ ] **Step 4: 提交**

```bash
git add frontend/src/App.tsx frontend/src/components/Header.tsx frontend/src/components/KPIStrip.tsx
git commit -m "feat: add main layout and KPI components"
```

### Task 5.6: 分区热力图组件

**Files:**
- Create: `frontend/src/components/PartitionHeatmap.tsx`

- [ ] **Step 1: 创建热力图组件**

```tsx
// frontend/src/components/PartitionHeatmap.tsx
import { useApi } from '../hooks/useApi';
import { fetchPartitionHeatmap } from '../api';
import { useDashboard } from '../context/DashboardContext';
import ReactECharts from 'echarts-for-react';

export default function PartitionHeatmap() {
  const { state, setSelectedCategory } = useDashboard();
  const { data } = useApi(
    () => fetchPartitionHeatmap(state.snapshotDate || undefined), 
    [state.snapshotDate]
  );
  
  if (!data) return <div className="h-64 bg-gray-800 animate-pulse rounded-lg" />;
  
  const option = {
    tooltip: {
      position: 'top',
      formatter: (params: any) => {
        return `${data.categories[params.value[1]]}<br/>${data.dates[params.value[0]]}<br/>播放量: ${params.value[2]}`;
      }
    },
    grid: {
      left: 80,
      right: 20,
      top: 20,
      bottom: 60,
    },
    xAxis: {
      type: 'category',
      data: data.dates,
      axisLabel: { color: '#9CA3AF' }
    },
    yAxis: {
      type: 'category',
      data: data.categories,
      axisLabel: { color: '#9CA3AF' }
    },
    visualMap: {
      min: 0,
      max: Math.max(...data.values.flat()),
      calculable: true,
      orient: 'horizontal',
      left: 'center',
      bottom: 0,
      inRange: {
        color: ['#1E3A5F', '#00A1D6', '#FB7299']
      },
      textStyle: { color: '#9CA3AF' }
    },
    series: [{
      type: 'heatmap',
      data: data.values.map((row, y) => row.map((val, x) => [x, y, val])).flat(),
      emphasis: {
        itemStyle: {
          shadowBlur: 10,
          shadowColor: 'rgba(0, 0, 0, 0.5)'
        }
      }
    }]
  };
  
  const handleClick = (params: any) => {
    const category = data.categories[params.value[1]];
    setSelectedCategory(category === state.selectedCategory ? null : category);
  };
  
  return (
    <div className="bg-gray-800 rounded-lg p-4">
      <h3 className="text-sm text-gray-400 mb-2">分区热力图</h3>
      <ReactECharts 
        option={option} 
        style={{ height: '250px' }}
        onEvents={{ click: handleClick }}
      />
    </div>
  );
}
```

- [ ] **Step 2: 提交**

```bash
git add frontend/src/components/PartitionHeatmap.tsx
git commit -m "feat: add partition heatmap component"
```

### Task 5.7: 其他图表组件

**Files:**
- Create: `frontend/src/components/TrendChart.tsx`
- Create: `frontend/src/components/WordCloud.tsx`
- Create: `frontend/src/components/VideoTable.tsx`
- Create: `frontend/src/components/RadarChart.tsx`
- Create: `frontend/src/components/WordTrendChart.tsx`
- Create: `frontend/src/components/VideoDetailPanel.tsx`

- [ ] **Step 1: 创建趋势折线图**

```tsx
// frontend/src/components/TrendChart.tsx
import ReactECharts from 'echarts-for-react';
import { useDashboard } from '../context/DashboardContext';

export default function TrendChart() {
  const { state } = useDashboard();
  
  // TODO: 实现趋势图
  
  return (
    <div className="bg-gray-800 rounded-lg p-4">
      <h3 className="text-sm text-gray-400 mb-2">趋势折线图</h3>
      <div className="h-64 flex items-center justify-center text-gray-500">
        TODO: 趋势图实现
      </div>
    </div>
  );
}
```

- [ ] **Step 2: 创建其他组件骨架**

```bash
# 创建所有组件的骨架代码
```

- [ ] **Step 3: 提交**

```bash
git add frontend/src/components/
git commit -m "feat: add chart component skeletons"
```

---

## 阶段6：大数据集成

### Task 6.1: HDFS数据上传

**Files:**
- Create: `scripts/upload_to_hdfs.sh`

- [ ] **Step 1: 创建上传脚本**

```bash
#!/bin/bash
# scripts/upload_to_hdfs.sh

DATE=${1:-$(date -d "yesterday" +%Y%m%d)}
HDFS_BASE="/bilibili/raw"
LOCAL_BASE="$HOME/output"

# 找到最新的快照目录
SNAPSHOT_DIR=$(ls -d ${LOCAL_BASE}/${DATE}_* | tail -1)

if [ -z "$SNAPSHOT_DIR" ]; then
    echo "No snapshot found for $DATE"
    exit 1
fi

echo "Uploading snapshot: $SNAPSHOT_DIR"

# 上传视频详情
hdfs dfs -mkdir -p ${HDFS_BASE}/video/date=${DATE}
hdfs dfs -put ${SNAPSHOT_DIR}/video_detail.ndjson ${HDFS_BASE}/video/date=${DATE}/

# 上传评论
hdfs dfs -mkdir -p ${HDFS_BASE}/comment/date=${DATE}
hdfs dfs -put ${SNAPSHOT_DIR}/comments/*.ndjson ${HDFS_BASE}/comment/date=${DATE}/

# 上传弹幕
hdfs dfs -mkdir -p ${HDFS_BASE}/danmaku/date=${DATE}
hdfs dfs -put ${SNAPSHOT_DIR}/danmaku/*.ndjson ${HDFS_BASE}/danmaku/date=${DATE}/

echo "Upload completed!"
```

- [ ] **Step 2: 提交**

```bash
git add scripts/upload_to_hdfs.sh
git commit -m "feat: add HDFS upload script"
```

### Task 6.2: Hive建表

**Files:**
- Create: `scripts/hive_ddl.sql`

- [ ] **Step 1: 创建Hive DDL**

```sql
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
```

- [ ] **Step 2: 提交**

```bash
git add scripts/hive_ddl.sql
git commit -m "feat: add Hive DDL for data warehouse"
```

### Task 6.3: Spark ETL作业

**Files:**
- Create: `etl/spark_etl.py`

- [ ] **Step 1: 创建Spark作业**

```python
# etl/spark_etl.py
from pyspark.sql import SparkSession
from pyspark.sql.functions import *
from pyspark.sql.types import *

def main():
    spark = SparkSession.builder \
        .appName("Bilibili ETL") \
        .enableHiveSupport() \
        .getOrCreate()
    
    date = spark.conf.get("spark.etl.date")
    
    # 1. 读取ODS层数据
    video_df = spark.table(f"bilibili.ods_video").where(col("date") == date)
    comment_df = spark.table(f"bilibili.ods_comment").where(col("date") == date)
    
    # 2. 数据清洗
    video_clean = video_df.dropDuplicates(["bvid"]) \
        .filter(col("view_count") > 0)
    
    # 3. 计算互动指标
    video_with_rates = video_clean \
        .withColumn("like_rate", col("like_count") / col("view_count")) \
        .withColumn("coin_rate", col("coin_count") / col("view_count")) \
        .withColumn("fav_rate", col("favorite_count") / col("view_count")) \
        .withColumn("reply_rate", col("reply_count") / col("view_count")) \
        .withColumn("share_rate", col("share_count") / col("view_count"))
    
    # 4. 写入DWD层
    video_with_rates.write \
        .mode("overwrite") \
        .partitionBy("date") \
        .saveAsTable("bilibili.dwd_video_clean")
    
    # 5. 计算分区统计
    partition_stats = video_clean.groupBy("sub_category") \
        .agg(
            count("*").alias("video_count"),
            sum("view_count").alias("total_views"),
            sum("like_count").alias("total_likes"),
            avg(col("like_count") / col("view_count")).alias("avg_like_rate")
        )
    
    partition_stats.write \
        .mode("overwrite") \
        .saveAsTable("bilibili.dws_partition_stats")
    
    # 6. 同步到ADS层（供MySQL查询）
    video_with_rates.write \
        .mode("overwrite") \
        .partitionBy("date") \
        .saveAsTable("bilibili.ads_hot_video")
    
    spark.stop()

if __name__ == "__main__":
    main()
```

- [ ] **Step 2: 提交**

```bash
git add etl/spark_etl.py
git commit -m "feat: add Spark ETL job"
```

---

## 阶段7：部署

### Task 7.1: systemd服务配置

**Files:**
- Create: `scripts/bilibili-crawler.service`
- Create: `scripts/bilibili-backend.service`

- [ ] **Step 1: 创建爬虫服务**

```ini
# scripts/bilibili-crawler.service
[Unit]
Description=Bilibili Hot Video Crawler
After=network.target

[Service]
Type=simple
User=inkpear
WorkingDirectory=/home/inkpear/crawler
ExecStart=/home/inkpear/crawler/target/release/bilibili-crawler
Restart=always
RestartSec=10

[Install]
WantedBy=default.target
```

- [ ] **Step 2: 创建后端服务**

```ini
# scripts/bilibili-backend.service
[Unit]
Description=Bilibili Backend API
After=network.target mysql.service

[Service]
Type=simple
User=inkpear
WorkingDirectory=/home/inkpear/backend
ExecStart=/home/inkpear/backend/target/release/bilibili-backend
Restart=always
RestartSec=10
Environment=DATABASE_URL=mysql://root:password@localhost/bilibili

[Install]
WantedBy=default.target
```

- [ ] **Step 3: 提交**

```bash
git add scripts/*.service
git commit -m "feat: add systemd service configs"
```

### Task 7.2: Nginx配置

**Files:**
- Create: `scripts/nginx.conf`

- [ ] **Step 1: 创建Nginx配置**

```nginx
# scripts/nginx.conf
server {
    listen 80;
    server_name _;
    
    root /home/inkpear/frontend/dist;
    index index.html;
    
    location / {
        try_files $uri $uri/ /index.html;
    }
    
    location /api {
        proxy_pass http://localhost:9090;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

- [ ] **Step 2: 提交**

```bash
git add scripts/nginx.conf
git commit -m "feat: add Nginx config"
```

### Task 7.3: 部署脚本

**Files:**
- Create: `scripts/deploy.sh`

- [ ] **Step 1: 创建部署脚本**

```bash
#!/bin/bash
# scripts/deploy.sh

set -e

echo "=== Deploying Bilibili Hot Video Analysis Platform ==="

# 1. 构建爬虫
echo "Building crawler..."
cd crawler
cargo build --release
cd ..

# 2. 构建后端
echo "Building backend..."
cd backend
cargo build --release
cd ..

# 3. 构建前端
echo "Building frontend..."
cd frontend
npm run build
cd ..

# 4. 部署前端
echo "Deploying frontend..."
sudo cp -r frontend/dist/* /var/www/html/

# 5. 部署服务
echo "Deploying services..."
sudo cp scripts/bilibili-crawler.service /etc/systemd/system/
sudo cp scripts/bilibili-backend.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable bilibili-crawler bilibili-backend
sudo systemctl restart bilibili-crawler bilibili-backend

# 6. 部署Nginx配置
echo "Deploying Nginx config..."
sudo cp scripts/nginx.conf /etc/nginx/sites-available/bilibili
sudo ln -sf /etc/nginx/sites-available/bilibili /etc/nginx/sites-enabled/
sudo nginx -t && sudo systemctl reload nginx

echo "=== Deployment completed! ==="
```

- [ ] **Step 2: 提交**

```bash
git add scripts/deploy.sh
git commit -m "feat: add deployment script"
```

---

## 完成

计划已完成，保存到 `docs/superpowers/plans/2026-06-07-dashboard-v2-implementation.md`

**两种执行方式：**

1. **Subagent-Driven（推荐）** - 我为每个task分发独立的subagent，task之间进行review，快速迭代

2. **Inline Execution** - 在当前session中执行task，批量执行并设置检查点

你选择哪种方式？
