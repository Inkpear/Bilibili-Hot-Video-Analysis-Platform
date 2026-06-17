use std::collections::HashSet;
use std::fs;
use std::path::Path;

use tracing::info;

use crate::error::Result;
use crate::models::{Comment, Danmaku, UpInfo, VideoDetail};

fn read_ndjson_dir<T: serde::de::DeserializeOwned>(dir: &Path) -> Result<Vec<T>> {
    let mut items = Vec::new();
    if !dir.exists() {
        return Ok(items);
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "ndjson") {
            let content = fs::read_to_string(&path)?;
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                match serde_json::from_str::<T>(line) {
                    Ok(item) => items.push(item),
                    Err(e) => {
                        tracing::warn!(error = %e, path = %path.display(), "skipping malformed line");
                    }
                }
            }
        }
    }
    Ok(items)
}

pub fn load_videos(dir: &Path) -> Result<Vec<VideoDetail>> {
    let videos = read_ndjson_dir(dir)?;
    info!(count = videos.len(), "loaded video details");
    Ok(videos)
}

pub fn load_ups(dir: &Path) -> Result<Vec<UpInfo>> {
    let all: Vec<UpInfo> = read_ndjson_dir(dir)?;
    let mut seen = HashSet::new();
    let mut unique = Vec::new();
    for up in &all {
        if seen.insert(&up.up_id) {
            unique.push(up.clone());
        }
    }
    info!(total = all.len(), unique = unique.len(), "loaded up info");
    Ok(unique)
}

pub fn load_comments(dir: &Path) -> Result<Vec<Comment>> {
    let comments = read_ndjson_dir(dir)?;
    info!(count = comments.len(), "loaded comments");
    Ok(comments)
}

pub fn load_danmaku(dir: &Path) -> Result<Vec<Danmaku>> {
    let danmaku = read_ndjson_dir(dir)?;
    info!(count = danmaku.len(), "loaded danmaku");
    Ok(danmaku)
}
