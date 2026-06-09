use std::collections::HashMap;

use serde::Deserialize;

use crate::config::Config;
use crate::error::{CrawlerError, Result};
use crate::models::UpInfo;
use crate::wbi::WbiSigner;

// ── Bilibili API response types ──────────────────────────────────────────────

// /x/space/wbi/acc/info
#[derive(Debug, Deserialize)]
struct UpResponse {
    code: i32,
    data: Option<UpData>,
}

#[derive(Debug, Deserialize)]
struct UpData {
    #[allow(dead_code)]
    mid: i64,
    name: String,
    face: String,
    sign: String,
    #[serde(rename = "official_verify")]
    official_verify: Option<OfficialVerify>,
}

#[derive(Debug, Deserialize)]
struct OfficialVerify {
    #[serde(rename = "type")]
    verify_type: i32,
    desc: String,
}

// /x/relation/stat
#[derive(Debug, Deserialize)]
struct RelationResponse {
    code: i32,
    data: Option<RelationData>,
}

#[derive(Debug, Deserialize)]
struct RelationData {
    follower: i64,
}

// /x/space/wbi/arc/search
#[derive(Debug, Deserialize)]
struct ArchiveResponse {
    code: i32,
    data: Option<ArchiveData>,
}

#[derive(Debug, Deserialize)]
struct ArchiveData {
    page: Option<PageData>,
}

#[derive(Debug, Deserialize)]
struct PageData {
    count: i64,
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Fetches UP (uploader) information from Bilibili.
///
/// Makes three API calls:
/// 1. `/x/space/wbi/acc/info` — basic profile info (name, avatar, sign, verify)
/// 2. `/x/relation/stat`     — follower count (no WBI signing needed)
/// 3. `/x/space/wbi/arc/search` — video count (WBI signed)
///
/// If any individual request fails, defaults are used for missing fields
/// rather than failing the entire operation. The `up_id` must be a valid
/// integer (the Bilibili `mid`).
pub async fn fetch_up_info(
    client: &reqwest::Client,
    config: &Config,
    wbi_signer: &mut WbiSigner,
    up_id: &str,
) -> Result<UpInfo> {
    let mid: i64 = up_id.parse().map_err(|_| {
        CrawlerError::Parse(format!("invalid up_id (not a valid integer): {up_id}"))
    })?;

    let cookie = std::env::var(&config.api.cookie_env_name).map_err(|_| {
        CrawlerError::Config(format!(
            "environment variable '{}' is not set",
            config.api.cookie_env_name
        ))
    })?;

    // Defaults in case individual requests fail
    let mut up_name = String::new();
    let mut up_avatar = String::new();
    let mut sign = String::new();
    let mut official_verify = String::new();
    let mut follower_count: i64 = 0;
    let mut video_count: i64 = 0;

    // ── 1. Profile info: /x/space/wbi/acc/info ───────────────────────────
    {
        let mut params: HashMap<String, String> = HashMap::new();
        params.insert("mid".to_string(), mid.to_string());

        match wbi_signer.sign_params(&mut params).await {
            Ok(()) => {
                let url = format!("{}/x/space/wbi/acc/info", config.api.base_url);
                tokio::time::sleep(std::time::Duration::from_millis(config.crawler.request_delay_ms)).await;
                match client
                    .get(&url)
                    .query(&params)
                    .header("Cookie", &cookie)
                    .header("Referer", "https://www.bilibili.com/")
                    .send()
                    .await
                {
                    Ok(resp) => match resp.json::<UpResponse>().await {
                        Ok(body) if body.code == 0 => {
                            if let Some(data) = body.data {
                                up_name = data.name;
                                up_avatar = data.face;
                                sign = data.sign;
                                official_verify = match data.official_verify {
                                    Some(v) if v.verify_type != 0 => {
                                        format!("{}:{}", v.verify_type, v.desc)
                                    }
                                    _ => String::new(),
                                };
                            }
                        }
                        Ok(body) => {
                            tracing::warn!(
                                code = body.code,
                                "up info API returned non-zero code, using defaults"
                            );
                        }
                        Err(e) => {
                            tracing::warn!(
                                error = %e,
                                "failed to deserialize up info response, using defaults"
                            );
                        }
                    },
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            "HTTP request failed for up info, using defaults"
                        );
                    }
                }
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "WBI signing failed for up info request, using defaults"
                );
            }
        }
    }

    // ── 2. Follower count: /x/relation/stat (no WBI) ─────────────────────
    {
        let url = format!("{}/x/relation/stat?vmid={}", config.api.base_url, mid);
        tokio::time::sleep(std::time::Duration::from_millis(config.crawler.request_delay_ms)).await;
        match client
            .get(&url)
            .header("Cookie", &cookie)
            .header("Referer", &config.api.base_url)
            .send()
            .await
        {
            Ok(resp) => match resp.json::<RelationResponse>().await {
                Ok(body) if body.code == 0 => {
                    if let Some(data) = body.data {
                        follower_count = data.follower;
                    }
                }
                Ok(body) => {
                    tracing::warn!(
                        code = body.code,
                        "relation API returned non-zero code, using defaults"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "failed to deserialize relation response, using defaults"
                    );
                }
            },
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "HTTP request failed for relation stat, using defaults"
                );
            }
        }
    }

    // ── 3. Video count: /x/space/wbi/arc/search ──────────────────────────
    {
        let mut params: HashMap<String, String> = HashMap::new();
        params.insert("mid".to_string(), mid.to_string());
        params.insert("pn".to_string(), "1".to_string());
        params.insert("ps".to_string(), "1".to_string());

        match wbi_signer.sign_params(&mut params).await {
            Ok(()) => {
                let url = format!("{}/x/space/wbi/arc/search", config.api.base_url);
                tokio::time::sleep(std::time::Duration::from_millis(config.crawler.request_delay_ms)).await;
                match client
                    .get(&url)
                    .query(&params)
                    .header("Cookie", &cookie)
                    .header("Referer", "https://www.bilibili.com/")
                    .send()
                    .await
                {
                    Ok(resp) => match resp.json::<ArchiveResponse>().await {
                        Ok(body) if body.code == 0 => {
                            if let Some(data) = body.data {
                                video_count = data.page.map(|p| p.count).unwrap_or(0);
                            }
                        }
                        Ok(body) => {
                            tracing::warn!(
                                code = body.code,
                                "archive search API returned non-zero code, using defaults"
                            );
                        }
                        Err(e) => {
                            tracing::warn!(
                                error = %e,
                                "failed to deserialize archive search response, using defaults"
                            );
                        }
                    },
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            "HTTP request failed for archive search, using defaults"
                        );
                    }
                }
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "WBI signing failed for archive search request, using defaults"
                );
            }
        }
    }

    Ok(UpInfo {
        up_id: up_id.to_string(),
        up_name,
        up_avatar,
        follower_count,
        video_count,
        sign,
        official_verify,
    })
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_up_id_parse_fails() {
        // A non-numeric up_id should fail to parse as i64
        let bad_id = "not-a-number";
        let result: Result<i64> = bad_id.parse().map_err(|_| {
            CrawlerError::Parse(format!("invalid up_id: {bad_id}"))
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_up_response() {
        let json = r#"{"code":0,"data":{"mid":123,"name":"test","face":"","sign":""}}"#;
        let resp: UpResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.code, 0);
        let data = resp.data.unwrap();
        assert_eq!(data.mid, 123);
        assert_eq!(data.name, "test");
        assert!(data.official_verify.is_none());
    }

    #[test]
    fn test_deserialize_up_response_with_official_verify() {
        let json = r#"{
            "code": 0,
            "data": {
                "mid": 456,
                "name": "verified_up",
                "face": "https://example.com/face.jpg",
                "sign": "official account",
                "official_verify": {
                    "type": 1,
                    "desc": "Bilibili 知名UP主"
                }
            }
        }"#;
        let resp: UpResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.code, 0);
        let data = resp.data.unwrap();
        assert_eq!(data.name, "verified_up");
        let verify = data.official_verify.unwrap();
        assert_eq!(verify.verify_type, 1);
        assert_eq!(verify.desc, "Bilibili 知名UP主");
    }

    #[test]
    fn test_deserialize_up_response_error_code() {
        let json = r#"{"code":-400,"data":null}"#;
        let resp: UpResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.code, -400);
        assert!(resp.data.is_none());
    }

    #[test]
    fn test_deserialize_relation_response() {
        let json = r#"{"code":0,"data":{"follower":12345}}"#;
        let resp: RelationResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.code, 0);
        assert_eq!(resp.data.unwrap().follower, 12345);
    }

    #[test]
    fn test_deserialize_archive_response() {
        let json = r#"{"code":0,"data":{"page":{"count":42}}}"#;
        let resp: ArchiveResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.code, 0);
        let count = resp.data.unwrap().page.unwrap().count;
        assert_eq!(count, 42);
    }

    #[test]
    fn test_deserialize_archive_response_no_page() {
        let json = r#"{"code":0,"data":{"page":null}}"#;
        let resp: ArchiveResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.code, 0);
        assert!(resp.data.unwrap().page.is_none());
    }

    #[test]
    fn test_up_info_defaults_on_empty_fields() {
        // Verify that UpInfo default construction works (the function uses
        // String::new() and 0 for defaults on failure)
        let info = UpInfo {
            up_id: "123".to_string(),
            up_name: String::new(),
            up_avatar: String::new(),
            follower_count: 0,
            video_count: 0,
            sign: String::new(),
            official_verify: String::new(),
        };
        assert_eq!(info.up_id, "123");
        assert_eq!(info.follower_count, 0);
        assert_eq!(info.video_count, 0);
    }
}
