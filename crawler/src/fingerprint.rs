use std::time::{SystemTime, UNIX_EPOCH};

use hmac::{Hmac, Mac};
use rand::seq::SliceRandom;
use serde::Deserialize;
use sha2::Sha256;
use tracing::info;

use crate::error::{CrawlerError, Result};

type HmacSha256 = Hmac<Sha256>;

// ---------------------------------------------------------------------------
// Pre-defined browser profiles
// ---------------------------------------------------------------------------

struct BrowserProfile {
    user_agent: &'static str,
    sec_ch_ua: &'static str,
    sec_ch_ua_platform: &'static str,
    accept_language: &'static str,
}

const BROWSER_PROFILES: &[BrowserProfile] = &[
    BrowserProfile {
        user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
        sec_ch_ua: r#""Google Chrome";v="131", "Chromium";v="131", "Not_A Brand";v="24""#,
        sec_ch_ua_platform: r#""Windows""#,
        accept_language: "zh-CN,zh;q=0.9,en;q=0.8",
    },
    BrowserProfile {
        user_agent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
        sec_ch_ua: r#""Google Chrome";v="131", "Chromium";v="131", "Not_A Brand";v="24""#,
        sec_ch_ua_platform: r#""macOS""#,
        accept_language: "zh-CN,zh;q=0.9,en;q=0.8",
    },
    BrowserProfile {
        user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:133.0) Gecko/20100101 Firefox/133.0",
        sec_ch_ua: r#""Firefox";v="133", "Not_A Brand";v="24""#,
        sec_ch_ua_platform: r#""Windows""#,
        accept_language: "zh-CN,zh;q=0.8,en-US;q=0.5,en;q=0.3",
    },
    BrowserProfile {
        user_agent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.2 Safari/605.1.15",
        sec_ch_ua: r#""Safari";v="18", "Not_A Brand";v="24""#,
        sec_ch_ua_platform: r#""macOS""#,
        accept_language: "zh-CN,zh-Hans;q=0.9",
    },
    BrowserProfile {
        user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36 Edg/131.0.0.0",
        sec_ch_ua: r#""Microsoft Edge";v="131", "Chromium";v="131", "Not_A Brand";v="24""#,
        sec_ch_ua_platform: r#""Windows""#,
        accept_language: "zh-CN,zh;q=0.9,en;q=0.8",
    },
];

// ---------------------------------------------------------------------------
// DeviceFingerprint
// ---------------------------------------------------------------------------

pub struct DeviceFingerprint {
    /// Server-issued device identifier from `/x/frontend/finger/spi`.
    pub buvid3: String,
    /// Server-issued buvid4 from `/x/frontend/finger/spi` (b_4 field).
    pub buvid4: String,
    /// Session initialization timestamp.
    pub b_nut: String,
    /// Random UUID4.
    pub uuid: String,
    /// bili_ticket JWT — reduces risk control probability.
    pub bili_ticket: String,
    /// User-Agent string.
    pub user_agent: String,
    /// Sec-CH-UA header value.
    pub sec_ch_ua: String,
    /// Sec-CH-UA-Platform header value.
    pub sec_ch_ua_platform: String,
    /// Accept-Language header value.
    pub accept_language: String,
}

#[derive(Deserialize)]
struct SpiResponse {
    code: i32,
    data: Option<SpiData>,
}

#[derive(Deserialize)]
struct SpiData {
    b_3: String,
    b_4: String,
}

#[derive(Deserialize)]
struct TicketResponse {
    code: i32,
    data: Option<TicketData>,
}

#[derive(Deserialize)]
struct TicketData {
    ticket: String,
}

impl DeviceFingerprint {
    /// Create a new device fingerprint by calling the SPI and GenWebTicket endpoints.
    pub async fn create(client: &reqwest::Client, ua_override: Option<&str>) -> Result<Self> {
        let profile = match ua_override {
            Some(_) => None,
            None => {
                let mut rng = rand::thread_rng();
                Some(BROWSER_PROFILES.choose(&mut rng).unwrap())
            }
        };

        let user_agent = ua_override
            .or_else(|| profile.map(|p| p.user_agent))
            .unwrap();

        let sec_ch_ua = profile.map(|p| p.sec_ch_ua.to_string()).unwrap_or_else(|| {
            r#""Google Chrome";v="131", "Chromium";v="131", "Not_A Brand";v="24""#.to_string()
        });

        let sec_ch_ua_platform = profile
            .map(|p| p.sec_ch_ua_platform.to_string())
            .unwrap_or_else(|| r#""Windows""#.to_string());

        let accept_language = profile
            .map(|p| p.accept_language.to_string())
            .unwrap_or_else(|| "zh-CN,zh;q=0.9,en;q=0.8".to_string());

        // 1. Call SPI endpoint to get buvid3 + buvid4
        let spi_url = "https://api.bilibili.com/x/frontend/finger/spi";
        let resp: SpiResponse = client
            .get(spi_url)
            .header("User-Agent", user_agent)
            .send()
            .await?
            .json()
            .await?;

        if resp.code != 0 {
            return Err(CrawlerError::Api {
                code: resp.code,
                message: "SPI endpoint returned non-zero code".to_string(),
            });
        }

        let data = resp.data.ok_or_else(|| CrawlerError::Api {
            code: resp.code,
            message: "SPI response missing data field".to_string(),
        })?;

        let b_nut = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before UNIX epoch")
            .as_secs()
            .to_string();

        let uuid_str = uuid::Uuid::new_v4().to_string();

        // 2. Fetch bili_ticket from GenWebTicket
        let bili_ticket = Self::fetch_bili_ticket(client, user_agent)
            .await
            .unwrap_or_default();

        info!(
            buvid3 = %data.b_3,
            buvid4 = %data.b_4,
            has_ticket = !bili_ticket.is_empty(),
            ua = %user_agent,
            "Device fingerprint created"
        );

        Ok(Self {
            buvid3: data.b_3,
            buvid4: data.b_4,
            b_nut,
            uuid: uuid_str,
            bili_ticket,
            user_agent: user_agent.to_string(),
            sec_ch_ua,
            sec_ch_ua_platform,
            accept_language,
        })
    }

    /// Fetch bili_ticket from GenWebTicket endpoint using HMAC-SHA256.
    async fn fetch_bili_ticket(client: &reqwest::Client, user_agent: &str) -> Result<String> {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before UNIX epoch")
            .as_secs();

        // HMAC-SHA256 with key "XgwSnGZ1p", message "ts{timestamp}"
        let mut mac = HmacSha256::new_from_slice(b"XgwSnGZ1p")
            .map_err(|e| CrawlerError::Config(format!("HMAC init failed: {}", e)))?;
        mac.update(format!("ts{}", ts).as_bytes());
        let hexsign = hex::encode(mac.finalize().into_bytes());

        let url = "https://api.bilibili.com/bapis/bilibili.api.ticket.v1.Ticket/GenWebTicket";
        let resp: TicketResponse = client
            .post(url)
            .query(&[
                ("key_id", "ec02"),
                ("hexsign", &hexsign),
                ("context[ts]", &ts.to_string()),
                ("csrf", ""),
            ])
            .header("User-Agent", user_agent)
            .header("Referer", "https://www.bilibili.com/")
            .send()
            .await?
            .json()
            .await?;

        if resp.code != 0 {
            return Err(CrawlerError::Api {
                code: resp.code,
                message: "GenWebTicket returned non-zero code".to_string(),
            });
        }

        let ticket = resp
            .data
            .ok_or_else(|| CrawlerError::Api {
                code: resp.code,
                message: "GenWebTicket response missing data".to_string(),
            })?
            .ticket;

        info!(ticket_len = ticket.len(), "bili_ticket fetched");
        Ok(ticket)
    }

    /// Build a cookie string with all required cookies for Bilibili API requests.
    pub fn cookie_string(&self, sessdata: &str, bili_jct: &str, dede_user_id: &str) -> String {
        let mut parts = vec![
            format!("buvid3={}", self.buvid3),
            format!("buvid4={}", self.buvid4),
            format!("b_nut={}", self.b_nut),
            format!("_uuid={}", self.uuid),
            format!("SESSDATA={}", sessdata),
            format!("bili_jct={}", bili_jct),
            format!("DedeUserID={}", dede_user_id),
        ];
        if !self.bili_ticket.is_empty() {
            parts.push(format!("bili_ticket={}", self.bili_ticket));
        }
        parts.join("; ")
    }

    /// Apply device-specific headers to a request builder.
    pub fn apply_headers(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        req.header("User-Agent", &self.user_agent)
            .header("Sec-CH-UA", &self.sec_ch_ua)
            .header("Sec-CH-UA-Platform", &self.sec_ch_ua_platform)
            .header("Accept-Language", &self.accept_language)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cookie_string_format() {
        let fp = DeviceFingerprint {
            buvid3: "test-b3".to_string(),
            buvid4: "test-b4".to_string(),
            b_nut: "12345".to_string(),
            uuid: "test-uuid".to_string(),
            bili_ticket: "test-ticket".to_string(),
            user_agent: "test-ua".to_string(),
            sec_ch_ua: "test-ch-ua".to_string(),
            sec_ch_ua_platform: "test-platform".to_string(),
            accept_language: "zh-CN".to_string(),
        };

        let cookie = fp.cookie_string("sd", "jt", "123");
        assert!(cookie.contains("buvid3=test-b3"));
        assert!(cookie.contains("buvid4=test-b4"));
        assert!(cookie.contains("b_nut=12345"));
        assert!(cookie.contains("_uuid=test-uuid"));
        assert!(cookie.contains("SESSDATA=sd"));
        assert!(cookie.contains("bili_jct=jt"));
        assert!(cookie.contains("DedeUserID=123"));
        assert!(cookie.contains("bili_ticket=test-ticket"));
    }

    #[test]
    fn test_cookie_string_without_ticket() {
        let fp = DeviceFingerprint {
            buvid3: "test-b3".to_string(),
            buvid4: "test-b4".to_string(),
            b_nut: "12345".to_string(),
            uuid: "test-uuid".to_string(),
            bili_ticket: String::new(), // empty
            user_agent: "test-ua".to_string(),
            sec_ch_ua: "test-ch-ua".to_string(),
            sec_ch_ua_platform: "test-platform".to_string(),
            accept_language: "zh-CN".to_string(),
        };

        let cookie = fp.cookie_string("sd", "jt", "123");
        assert!(!cookie.contains("bili_ticket="));
    }
}
