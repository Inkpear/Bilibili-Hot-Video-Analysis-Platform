use serde::Deserialize;

use crate::error::{CrawlerError, Result};
use crate::models::VideoDetail;

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

#[derive(Debug, Deserialize)]
struct TagResponse {
    code: i32,
    data: Option<Vec<TagItem>>,
}

/// Fetch tags for a video from `/x/tag/archive/tags`.
pub async fn fetch_tags(
    client: &reqwest::Client,
    cookie: &str,
    base_url: &str,
    bvid: &str,
) -> Result<String> {
    let url = format!("{}/x/tag/archive/tags?bvid={}", base_url, bvid);

    let response = client
        .get(&url)
        .header("Cookie", cookie)
        .header("Referer", "https://www.bilibili.com")
        .send()
        .await?;

    if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(CrawlerError::RateLimited {
            retry_after: Some(60),
        });
    }

    let resp: TagResponse = response.json().await?;
    if resp.code != 0 {
        return Ok(String::new());
    }

    let tags = resp
        .data
        .unwrap_or_default()
        .iter()
        .map(|t| t.tag_name.clone())
        .collect::<Vec<_>>()
        .join(",");

    Ok(tags)
}

/// Fetch video detail from `/x/web-interface/view`.
///
/// Returns `Err` on non-zero API code or network failure.
/// Returns `Err(RateLimited)` on HTTP 429.
pub async fn fetch_video_detail(
    client: &reqwest::Client,
    cookie: &str,
    base_url: &str,
    bvid: &str,
) -> Result<VideoDetail> {
    let url = format!("{}/x/web-interface/view?bvid={}", base_url, bvid);

    let response = client
        .get(&url)
        .header("Cookie", cookie)
        .header("Referer", "https://www.bilibili.com")
        .send()
        .await?;

    if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(CrawlerError::RateLimited {
            retry_after: Some(60),
        });
    }

    let resp: VideoResponse = response.json().await?;

    if resp.code != 0 {
        // -400 = bad parameter, -404 = video deleted — no point retrying
        if resp.code == -400 || resp.code == -404 {
            return Err(CrawlerError::Permanent(format!(
                "video {} unavailable (code {})",
                bvid, resp.code
            )));
        }
        return Err(CrawlerError::Api {
            code: resp.code,
            message: format!(
                "video detail API returned code {} for bvid {}",
                resp.code, bvid
            ),
        });
    }

    let data = resp.data.ok_or_else(|| CrawlerError::Api {
        code: -1,
        message: format!("empty data in video detail response for bvid {}", bvid),
    })?;

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
                    "view": 1000, "like": 200, "coin": 50,
                    "favorite": 30, "share": 10, "reply": 40, "danmaku": 500
                },
                "tname": "知识",
                "tags": [{"tag_name": "科技"}, {"tag_name": "教程"}]
            }
        });
        let resp: VideoResponse = serde_json::from_value(body).unwrap();
        assert_eq!(resp.code, 0);
        let data = resp.data.unwrap();
        assert_eq!(data.tags.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_deserialize_tag_response() {
        let body = serde_json::json!({
            "code": 0,
            "data": [
                {"tag_name": "科技"},
                {"tag_name": "教程"},
                {"tag_name": "编程"}
            ]
        });
        let resp: TagResponse = serde_json::from_value(body).unwrap();
        assert_eq!(resp.code, 0);
        let tags = resp.data.unwrap();
        assert_eq!(tags.len(), 3);
        assert_eq!(tags[0].tag_name, "科技");
    }

    #[test]
    fn test_deserialize_tag_response_empty() {
        let body = serde_json::json!({
            "code": 0,
            "data": []
        });
        let resp: TagResponse = serde_json::from_value(body).unwrap();
        assert_eq!(resp.code, 0);
        assert!(resp.data.unwrap().is_empty());
    }
}
