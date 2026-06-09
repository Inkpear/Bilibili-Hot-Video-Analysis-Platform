use std::collections::{HashMap, HashSet};

use serde::Deserialize;
use tokio::time::{sleep, Duration};
use tracing::info;

use crate::config::Config;
use crate::error::{CrawlerError, Result};
use crate::wbi::WbiSigner;

// ---------------------------------------------------------------------------
// Private response types (Bilibili ranking/v2 API)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct RankingResponse {
    code: i32,
    message: String,
    data: Option<RankingData>,
}

#[derive(Debug, Deserialize)]
struct RankingData {
    list: Vec<RankingItem>,
}

#[derive(Debug, Deserialize)]
struct RankingItem {
    bvid: String,
    aid: i64,
    cid: i64,
    title: String,
    pubdate: i64,
    duration: i32,
    owner: RankingOwner,
    stat: RankingStat,
    tname: String,
}

#[derive(Debug, Deserialize)]
struct RankingOwner {
    mid: i64,
    name: String,
    face: String,
}

#[derive(Debug, Deserialize)]
struct RankingStat {
    view: i64,
    #[serde(rename = "like")]
    like_count: i64,
    coin: i64,
    favorite: i64,
    share: i64,
    reply: i64,
    danmaku: i64,
}

// ---------------------------------------------------------------------------
// Public output type
// ---------------------------------------------------------------------------

/// A video from a Bilibili ranking list, carrying both video metadata and
/// ranking provenance.
#[derive(Debug, Clone)]
pub struct RankedVideo {
    pub bvid: String,
    pub aid: i64,
    pub cid: i64,
    pub title: String,
    pub category: String,
    pub sub_category: String,
    pub duration: i32,
    pub publish_time: i64,
    pub up_id: String,
    pub up_name: String,
    pub up_avatar: String,
    pub url: String,
    pub view_count: i64,
    pub like_count: i64,
    pub coin_count: i64,
    pub favorite_count: i64,
    pub share_count: i64,
    pub reply_count: i64,
    pub danmaku_count: i64,
    pub rank_source: String,
    pub rank_name: String,
    pub rank_position: i32,
}

// ---------------------------------------------------------------------------
// Category lookup
// ---------------------------------------------------------------------------

fn rid_to_category(rid: u32) -> &'static str {
    match rid {
        1001 => "影视",
        1002 => "娱乐",
        1003 => "音乐",
        1004 => "舞蹈",
        1005 => "动画",
        1007 => "鬼畜",
        1008 => "游戏",
        1010 => "知识",
        1012 => "科技",
        1013 => "汽车",
        1014 => "时尚美妆",
        1018 => "体育运动",
        1020 => "美食",
        1024 => "动物",
        _ => "其他",
    }
}

// ---------------------------------------------------------------------------
// Main fetch function
// ---------------------------------------------------------------------------

/// Fetch rankings for all configured partitions and return a deduplicated
/// vector of ranked videos (first occurrence wins).
pub async fn fetch_all_rankings(
    client: &reqwest::Client,
    config: &Config,
    wbi_signer: &mut WbiSigner,
) -> Result<Vec<RankedVideo>> {
    let cookie = std::env::var(&config.api.cookie_env_name).map_err(|e| {
        CrawlerError::Config(format!(
            "failed to read cookie from env var '{}': {}",
            config.api.cookie_env_name, e
        ))
    })?;

    let base_url = &config.api.base_url;
    let ranking_url = format!("{}/x/web-interface/ranking/v2", base_url);

    let mut all_videos: Vec<RankedVideo> = Vec::new();
    let mut seen_bvids: HashSet<String> = HashSet::new();

    for (idx, &rid) in config.partitions.rids.iter().enumerate() {
        let category = rid_to_category(rid);

        info!(
            partition = idx + 1,
            total = config.partitions.rids.len(),
            rid = rid,
            category = category,
            "fetching ranking"
        );

        // Build and sign parameters.
        let mut params: HashMap<String, String> = HashMap::new();
        params.insert("rid".to_string(), rid.to_string());
        params.insert("type".to_string(), "all".to_string());
        wbi_signer.sign_params(&mut params).await?;

        // Execute request.
        let resp = client
            .get(&ranking_url)
            .query(&params)
            .header("User-Agent", &config.api.user_agent)
            .header("Cookie", &cookie)
            .header("Referer", "https://www.bilibili.com/")
            .send()
            .await?
            .json::<RankingResponse>()
            .await?;

        if resp.code != 0 {
            return Err(CrawlerError::Api {
                code: resp.code,
                message: resp.message,
            });
        }

        let data = resp.data.ok_or_else(|| CrawlerError::Api {
            code: resp.code,
            message: "ranking response missing data field".to_string(),
        })?;

        let count_before = seen_bvids.len();

        for (rank_position, item) in data.list.into_iter().enumerate() {
            if seen_bvids.contains(&item.bvid) {
                continue;
            }

            let url = format!("https://www.bilibili.com/video/{}", &item.bvid);
            seen_bvids.insert(item.bvid.clone());

            all_videos.push(RankedVideo {
                bvid: item.bvid,
                aid: item.aid,
                cid: item.cid,
                title: item.title,
                category: category.to_string(),
                sub_category: item.tname,
                duration: item.duration,
                publish_time: item.pubdate,
                up_id: item.owner.mid.to_string(),
                up_name: item.owner.name,
                up_avatar: item.owner.face,
                url,
                view_count: item.stat.view,
                like_count: item.stat.like_count,
                coin_count: item.stat.coin,
                favorite_count: item.stat.favorite,
                share_count: item.stat.share,
                reply_count: item.stat.reply,
                danmaku_count: item.stat.danmaku,
                rank_source: "ranking".to_string(),
                rank_name: format!("{}榜", category),
                rank_position: (rank_position + 1) as i32,
            });
        }

        let new_count = seen_bvids.len() - count_before;
        info!(
            category = category,
            new_items = new_count,
            unique_so_far = seen_bvids.len(),
            "partition fetched"
        );

        // Delay between requests (skip after last partition).
        if idx + 1 < config.partitions.rids.len() {
            sleep(Duration::from_millis(config.crawler.request_delay_ms)).await;
        }
    }

    info!(
        total_unique = all_videos.len(),
        partitions_scanned = config.partitions.rids.len(),
        "all rankings fetched"
    );

    Ok(all_videos)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rid_to_category_known_partitions() {
        assert_eq!(rid_to_category(1001), "影视");
        assert_eq!(rid_to_category(1002), "娱乐");
        assert_eq!(rid_to_category(1003), "音乐");
        assert_eq!(rid_to_category(1004), "舞蹈");
        assert_eq!(rid_to_category(1005), "动画");
        assert_eq!(rid_to_category(1007), "鬼畜");
        assert_eq!(rid_to_category(1008), "游戏");
        assert_eq!(rid_to_category(1010), "知识");
        assert_eq!(rid_to_category(1012), "科技");
        assert_eq!(rid_to_category(1013), "汽车");
        assert_eq!(rid_to_category(1014), "时尚美妆");
        assert_eq!(rid_to_category(1018), "体育运动");
        assert_eq!(rid_to_category(1020), "美食");
        assert_eq!(rid_to_category(1024), "动物");
    }

    #[test]
    fn test_rid_to_category_unknown() {
        assert_eq!(rid_to_category(0), "其他");
        assert_eq!(rid_to_category(9999), "其他");
    }

    #[test]
    fn test_ranking_response_deserialization() {
        let json = r#"{
            "code": 0,
            "message": "0",
            "data": {
                "list": [
                    {
                        "bvid": "BV1xx411c7mD",
                        "aid": 171049,
                        "cid": 2811658,
                        "title": "Test Video",
                        "pubdate": 1609459200,
                        "duration": 120,
                        "owner": {
                            "mid": 12345,
                            "name": "TestUP",
                            "face": "https://example.com/face.jpg"
                        },
                        "stat": {
                            "view": 100000,
                            "like": 5000,
                            "coin": 2000,
                            "favorite": 3000,
                            "share": 500,
                            "reply": 100,
                            "danmaku": 800
                        },
                        "tname": "综合"
                    }
                ]
            }
        }"#;

        let resp: RankingResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.code, 0);
        let data = resp.data.unwrap();
        assert_eq!(data.list.len(), 1);
        assert_eq!(data.list[0].bvid, "BV1xx411c7mD");
        assert_eq!(data.list[0].stat.view, 100000);
        assert_eq!(data.list[0].tname, "综合");
        assert_eq!(data.list[0].stat.like_count, 5000);
    }

    #[test]
    fn test_ranking_response_error_code() {
        let json = r#"{
            "code": -352,
            "message": "risk control",
            "data": null
        }"#;

        let resp: RankingResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.code, -352);
        assert!(resp.data.is_none());
    }

    #[test]
    fn test_ranked_video_url_format() {
        let bvid = "BV1xx411c7mD";
        let url = format!("https://www.bilibili.com/video/{}", bvid);
        assert_eq!(url, "https://www.bilibili.com/video/BV1xx411c7mD");
    }

    #[test]
    fn test_rank_name_format() {
        let category = rid_to_category(1003);
        let rank_name = format!("{}榜", category);
        assert_eq!(rank_name, "音乐榜");
    }
}
