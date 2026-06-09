use serde::Deserialize;

use crate::config::Config;
use crate::error::{CrawlerError, Result};
use crate::models::VideoDetail;

// ── Bilibili API response types ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct VideoResponse {
    code: i32,
    data: Option<VideoData>,
}

#[derive(Debug, Deserialize)]
struct VideoData {
    bvid: String,
    aid: i64,
    cid: i64,
    title: String,
    desc: String,
    duration: i32,
    pubdate: i64,
    owner: VideoOwner,
    stat: VideoStat,
    #[serde(rename = "tname")]
    #[allow(dead_code)]
    tname: String,
    tags: Option<Vec<TagItem>>,
}

#[derive(Debug, Deserialize)]
struct VideoOwner {
    mid: i64,
    name: String,
    face: String,
}

#[derive(Debug, Deserialize)]
struct VideoStat {
    view: i64,
    like: i64,
    coin: i64,
    favorite: i64,
    share: i64,
    reply: i64,
    danmaku: i64,
}

#[derive(Debug, Deserialize)]
struct TagItem {
    tag_name: String,
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Fetches video detail from Bilibili `/x/web-interface/view?bvid={bvid}`.
///
/// Requires `config.api.cookie_env_name` to point to an env var containing a
/// valid Bilibili cookie (SESSDATA).  Returns a [`VideoDetail`] with all
/// available fields populated.  Fields that are **not** provided by this
/// endpoint (`category`, `sub_category`, `url`, `rank_source`, `rank_name`,
/// `rank_position`, `snapshot_id`, `collect_time`) are left at their defaults;
/// the caller is expected to fill them in.
pub async fn fetch_video_detail(
    client: &reqwest::Client,
    config: &Config,
    bvid: &str,
) -> Result<VideoDetail> {
    let url = format!(
        "{}/x/web-interface/view?bvid={}",
        config.api.base_url, bvid
    );

    let cookie = std::env::var(&config.api.cookie_env_name).map_err(|_| {
        CrawlerError::Config(format!(
            "environment variable '{}' is not set",
            config.api.cookie_env_name
        ))
    })?;

    let request_builder = client
        .get(&url)
        .header("User-Agent", &config.api.user_agent)
        .header("Cookie", &cookie)
        .header("Referer", "https://www.bilibili.com");

    tokio::time::sleep(std::time::Duration::from_millis(config.crawler.request_delay_ms)).await;

    let response = request_builder.send().await?;

    let resp: VideoResponse = response.json().await?;

    if resp.code != 0 {
        return Err(CrawlerError::Api {
            code: resp.code,
            message: format!("video detail API returned code {} for bvid {}", resp.code, bvid),
        });
    }

    let data = resp.data.ok_or_else(|| CrawlerError::Api {
        code: -1,
        message: format!("empty data in video detail response for bvid {}", bvid),
    })?;

    // Join all tag names into a comma-separated string.
    let tags = data
        .tags
        .unwrap_or_default()
        .iter()
        .map(|t| t.tag_name.clone())
        .collect::<Vec<_>>()
        .join(",");

    Ok(VideoDetail {
        bvid: data.bvid,
        aid: data.aid,
        cid: data.cid,
        title: data.title,
        description: data.desc,
        tags,
        category: String::new(),
        sub_category: String::new(),
        duration: data.duration,
        publish_time: data.pubdate,
        up_id: data.owner.mid.to_string(),
        up_name: data.owner.name,
        up_avatar: data.owner.face,
        url: String::new(),
        view_count: data.stat.view,
        like_count: data.stat.like,
        coin_count: data.stat.coin,
        favorite_count: data.stat.favorite,
        share_count: data.stat.share,
        reply_count: data.stat.reply,
        danmaku_count: data.stat.danmaku,
        rank_source: String::new(),
        rank_name: String::new(),
        rank_position: 0,
        snapshot_id: String::new(),
        collect_time: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that a minimal valid JSON response deserialises correctly and
    /// produces the expected `VideoDetail` fields.
    #[test]
    fn test_deserialize_video_response() {
        let body = serde_json::json!({
            "code": 0,
            "data": {
                "bvid": "BV1xx411c7mD",
                "aid": 12345678,
                "cid": 87654321,
                "title": "Test Video",
                "desc": "A test video description",
                "duration": 300,
                "pubdate": 1700000000,
                "owner": {
                    "mid": 100200300,
                    "name": "TestUploader",
                    "face": "https://example.com/avatar.jpg"
                },
                "stat": {
                    "view": 1000,
                    "like": 200,
                    "coin": 50,
                    "favorite": 30,
                    "share": 10,
                    "reply": 40,
                    "danmaku": 500
                },
                "tname": "知识",
                "tags": [
                    { "tag_name": "科技" },
                    { "tag_name": "教程" },
                    { "tag_name": "编程" }
                ]
            }
        });

        let resp: VideoResponse = serde_json::from_value(body).unwrap();
        assert_eq!(resp.code, 0);

        let data = resp.data.unwrap();
        assert_eq!(data.bvid, "BV1xx411c7mD");
        assert_eq!(data.title, "Test Video");
        assert_eq!(data.desc, "A test video description");
        assert_eq!(data.tname, "知识");
        assert_eq!(data.tags.as_ref().unwrap().len(), 3);
        assert_eq!(data.tags.as_ref().unwrap()[0].tag_name, "科技");
    }

    /// Verify that missing `tags` field (or null) is handled gracefully.
    #[test]
    fn test_deserialize_no_tags() {
        let body = serde_json::json!({
            "code": 0,
            "data": {
                "bvid": "BV1xx411c7mD",
                "aid": 12345678,
                "cid": 87654321,
                "title": "No Tags",
                "desc": "",
                "duration": 120,
                "pubdate": 1700000000,
                "owner": {
                    "mid": 1,
                    "name": "Uploader",
                    "face": ""
                },
                "stat": {
                    "view": 0, "like": 0, "coin": 0,
                    "favorite": 0, "share": 0,
                    "reply": 0, "danmaku": 0
                },
                "tname": ""
            }
        });

        let resp: VideoResponse = serde_json::from_value(body).unwrap();
        let data = resp.data.unwrap();
        assert!(data.tags.is_none());
    }

    /// Verify that a non-zero code is detected.
    #[test]
    fn test_deserialize_error_code() {
        let body = serde_json::json!({
            "code": -400,
            "data": null
        });

        let resp: VideoResponse = serde_json::from_value(body).unwrap();
        assert_eq!(resp.code, -400);
        assert!(resp.data.is_none());
    }
}
