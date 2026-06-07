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
