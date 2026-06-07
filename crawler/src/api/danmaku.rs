use reqwest::Client;
use anyhow::Result;
use crate::models::Danmaku;

// Danmaku API returns protobuf format, needs special handling
// Simplified version — actual implementation should use prost to parse protobuf

pub async fn fetch_danmaku(
    _client: &Client, 
    _base_url: &str, 
    cid: i64,
    _limit: usize
) -> Result<Vec<Danmaku>> {
    // Danmaku API: https://api.bilibili.com/x/v1/dm/list.so?oid={cid}
    // Returns protobuf format, needs parsing
    // Returning empty vec for now — actual implementation needs protobuf parsing
    tracing::warn!("Danmaku API not fully implemented yet, cid={}", cid);
    Ok(vec![])
}
