use reqwest::Client;
use serde::Deserialize;
use anyhow::Result;
use crate::models::{VideoDetail, UpInfo};

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
    duration: i32,
    pubdate: i64,
    owner: Owner,
    stat: Stat,
    tname: String,
}

#[derive(Debug, Deserialize)]
struct Owner {
    mid: i64,
    name: String,
    face: String,
}

#[derive(Debug, Deserialize)]
struct Stat {
    view: i64,
    like: i64,
    coin: i64,
    favorite: i64,
    share: i64,
    reply: i64,
    danmaku: i64,
}

pub async fn fetch_video_detail(client: &Client, base_url: &str, bvid: &str) -> Result<VideoDetail> {
    let url = format!("{}/x/web-interface/view?bvid={}", base_url, bvid);
    let resp: VideoResponse = client.get(&url).send().await?.json().await?;
    
    if resp.code != 0 {
        anyhow::bail!("API error: code={}", resp.code);
    }
    
    let data = resp.data;
    Ok(VideoDetail {
        bvid: data.bvid,
        aid: data.aid,
        cid: data.cid,
        title: data.title,
        description: data.desc,
        tags: String::new(), // Tags need separate API request
        sub_category: data.tname,
        duration: data.duration,
        publish_time: data.pubdate,
        up_id: data.owner.mid.to_string(),
        up_name: data.owner.name,
        up_avatar: data.owner.face,
        view_count: data.stat.view,
        like_count: data.stat.like,
        coin_count: data.stat.coin,
        favorite_count: data.stat.favorite,
        share_count: data.stat.share,
        reply_count: data.stat.reply,
        danmaku_count: data.stat.danmaku,
    })
}

pub async fn fetch_up_info(client: &Client, base_url: &str, up_id: &str) -> Result<UpInfo> {
    let url = format!("{}/x/space/acc/info?mid={}", base_url, up_id);
    
    #[derive(Debug, Deserialize)]
    struct UpResponse {
        code: i32,
        data: UpData,
    }
    
    #[derive(Debug, Deserialize)]
    struct UpData {
        mid: i64,
        name: String,
        face: String,
        sign: String,
        fans: i64,
    }
    
    let resp: UpResponse = client.get(&url).send().await?.json().await?;
    
    if resp.code != 0 {
        anyhow::bail!("API error: code={}", resp.code);
    }
    
    Ok(UpInfo {
        up_id: resp.data.mid.to_string(),
        up_name: resp.data.name,
        up_avatar: resp.data.face,
        follower_count: resp.data.fans,
        video_count: 0, // Needs separate API request
        sign: resp.data.sign,
        official_verify: String::new(),
    })
}
