use std::collections::HashMap;

use serde::Deserialize;

use crate::error::{CrawlerError, Result};
use crate::models::UpInfo;
use crate::wbi::WbiSigner;

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

#[derive(Debug, Deserialize)]
struct RelationResponse {
    code: i32,
    data: Option<RelationData>,
}

#[derive(Debug, Deserialize)]
struct RelationData {
    follower: i64,
}

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

/// Raw response used only to extract the `code` field for error detection.
#[derive(Debug, Deserialize)]
struct CodeResponse {
    code: i32,
}

/// Check if a raw JSON response is a -352 risk control rejection.
/// Returns Ok(true) if -352, Ok(false) if not, Err if JSON is unparseable.
fn is_risk_control(raw: &str) -> Result<bool> {
    let resp: CodeResponse = serde_json::from_str(raw).map_err(CrawlerError::Json)?;
    Ok(resp.code == -352)
}

/// Fetch UP info from three Bilibili endpoints.
///
/// Returns `Err(RateLimited)` on -352 risk control rejections so the task can be re-queued.
/// Non-critical failures (relation, archive) use defaults.
pub async fn fetch_up_info(
    client: &reqwest::Client,
    cookie: &str,
    base_url: &str,
    wbi_signer: &WbiSigner,
    up_id: &str,
) -> Result<UpInfo> {
    let mid: i64 = up_id.parse().map_err(|_| {
        CrawlerError::Parse(format!("invalid up_id (not a valid integer): {up_id}"))
    })?;

    let mut up_name = String::new();
    let mut up_avatar = String::new();
    let mut sign = String::new();
    let mut official_verify = String::new();
    let mut follower_count: i64 = 0;
    let mut video_count: i64 = 0;

    // 1. Profile: /x/space/wbi/acc/info (critical — WBI signed)
    {
        let mut params: HashMap<String, String> = HashMap::new();
        params.insert("mid".to_string(), mid.to_string());

        wbi_signer.sign_params(&mut params).await?;

        let url = format!("{}/x/space/wbi/acc/info", base_url);
        let resp = client
            .get(&url)
            .query(&params)
            .header("Cookie", cookie)
            .header("Referer", "https://www.bilibili.com/")
            .send()
            .await?;

        let raw = resp.text().await.unwrap_or_default();

        // Check for -352 risk control rejection first
        if is_risk_control(&raw)? {
            return Err(CrawlerError::RateLimited {
                retry_after: Some(60),
            });
        }

        match serde_json::from_str::<UpResponse>(&raw) {
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
                tracing::warn!(code = body.code, "up info API non-zero code");
            }
            Err(e) => {
                let preview: String = raw.chars().take(300).collect();
                tracing::warn!(error = %e, body = %preview, "failed to parse up info response");
            }
        }
    }

    // 2. Follower count: /x/relation/stat (no WBI — non-critical)
    {
        let url = format!("{}/x/relation/stat?vmid={}", base_url, mid);
        match client
            .get(&url)
            .header("Cookie", cookie)
            .header("Referer", "https://www.bilibili.com/")
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
                        "relation API non-zero code, using defaults"
                    );
                }
                Err(e) => {
                    tracing::warn!(error = %e, "failed to deserialize relation, using defaults");
                }
            },
            Err(e) => {
                tracing::warn!(error = %e, "HTTP request failed for relation, using defaults");
            }
        }
    }

    // 3. Video count: /x/space/wbi/arc/search (WBI signed — -352 is fatal)
    {
        let mut params: HashMap<String, String> = HashMap::new();
        params.insert("mid".to_string(), mid.to_string());
        params.insert("pn".to_string(), "1".to_string());
        params.insert("ps".to_string(), "1".to_string());

        wbi_signer.sign_params(&mut params).await?;

        let url = format!("{}/x/space/wbi/arc/search", base_url);
        let referer = format!("https://space.bilibili.com/{}/video", mid);
        let resp = client
            .get(&url)
            .query(&params)
            .header("Cookie", cookie)
            .header("Referer", &referer)
            .send()
            .await?;

        let raw = resp.text().await.unwrap_or_default();

        // Check for -352 risk control rejection
        if is_risk_control(&raw)? {
            return Err(CrawlerError::RateLimited {
                retry_after: Some(60),
            });
        }

        match serde_json::from_str::<ArchiveResponse>(&raw) {
            Ok(body) if body.code == 0 => {
                if let Some(data) = body.data {
                    video_count = data.page.map(|p| p.count).unwrap_or(0);
                }
            }
            Ok(body) => {
                tracing::warn!(code = body.code, "archive search non-zero code");
            }
            Err(e) => {
                let preview: String = raw.chars().take(300).collect();
                tracing::warn!(error = %e, body = %preview, "failed to parse archive search response");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_up_response() {
        let json = r#"{"code":0,"data":{"mid":123,"name":"test","face":"","sign":""}}"#;
        let resp: UpResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.code, 0);
        assert_eq!(resp.data.unwrap().name, "test");
    }

    #[test]
    fn test_deserialize_relation_response() {
        let json = r#"{"code":0,"data":{"follower":12345}}"#;
        let resp: RelationResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.data.unwrap().follower, 12345);
    }

    #[test]
    fn test_detect_risk_control() {
        let json =
            r#"{"code":-352,"message":"风控校验失败","ttl":1,"data":{"v_voucher":"voucher_abc"}}"#;
        assert!(is_risk_control(json).unwrap());

        let json_ok = r#"{"code":0,"data":{"mid":123,"name":"test","face":"","sign":""}}"#;
        assert!(!is_risk_control(json_ok).unwrap());
    }
}
