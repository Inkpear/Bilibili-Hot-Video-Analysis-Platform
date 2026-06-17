use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use md5::{Digest, Md5};
use serde_json::Value;
use tokio::sync::RwLock;
use tracing::{debug, warn};

use crate::error::{CrawlerError, Result};

/// Substring table for mixin key derivation — exact same 64-entry table as the
/// Python reference implementation.  Only the first 32 indices are used.
const MIXIN_KEY_ENC_TAB: [usize; 64] = [
    46, 47, 18, 2, 53, 8, 23, 32, 15, 50, 10, 31, 58, 3, 45, 35, 27, 43, 5, 49, 33, 9, 42, 19, 29,
    28, 14, 39, 12, 38, 41, 13, 37, 48, 7, 16, 24, 55, 40, 61, 26, 17, 0, 1, 60, 51, 30, 4, 22, 25,
    54, 21, 56, 59, 6, 63, 57, 62, 11, 36, 20, 34, 44, 52,
];

/// Characters stripped from parameter values when computing the signature.
const CHARS_TO_STRIP: &[char] = &['!', '\'', '(', ')', '*'];

/// Default TTL for cached WBI keys (4 hours).
const DEFAULT_KEY_TTL_SECS: u64 = 14_400;

// ---------------------------------------------------------------------------
// Pure helpers (testable)
// ---------------------------------------------------------------------------

fn get_mixin_key(orig: &str) -> String {
    let bytes = orig.as_bytes();
    MIXIN_KEY_ENC_TAB
        .iter()
        .take(32)
        .map(|&i| bytes[i] as char)
        .collect()
}

fn extract_key_from_url(url: &str) -> Result<String> {
    let filename = url
        .rsplit('/')
        .next()
        .ok_or_else(|| CrawlerError::Wbi(format!("cannot extract filename from URL: {url}")))?;

    let stem = filename.split('.').next().ok_or_else(|| {
        CrawlerError::Wbi(format!("cannot extract stem from filename: {filename}"))
    })?;

    if stem.is_empty() {
        return Err(CrawlerError::Wbi(format!(
            "empty key extracted from URL: {url}"
        )));
    }

    Ok(stem.to_string())
}

// ---------------------------------------------------------------------------
// WbiInner (mutable state behind the lock)
// ---------------------------------------------------------------------------

struct WbiInner {
    mixin_key: Option<String>,
    key_expires_at: u64,
}

// ---------------------------------------------------------------------------
// WbiSigner
// ---------------------------------------------------------------------------

/// Signs Bilibili API parameters with the WBI v3 scheme.
///
/// Uses `tokio::sync::RwLock` for interior mutability so `sign_params` can
/// take `&self`, allowing concurrent use from multiple Workers (each with
/// their own WbiSigner instance).
pub struct WbiSigner {
    cookie: String,
    base_url: String,
    client: reqwest::Client,
    key_ttl_secs: u64,
    inner: RwLock<WbiInner>,
}

impl WbiSigner {
    pub fn new(cookie: String, base_url: String, client: reqwest::Client) -> Self {
        Self {
            cookie,
            base_url,
            client,
            key_ttl_secs: DEFAULT_KEY_TTL_SECS,
            inner: RwLock::new(WbiInner {
                mixin_key: None,
                key_expires_at: 0,
            }),
        }
    }

    /// Sign `params` in-place by adding `wts` and `w_rid`.
    pub async fn sign_params(&self, params: &mut HashMap<String, String>) -> Result<()> {
        self.ensure_keys().await?;

        let inner = self.inner.read().await;
        let mixin_key = inner
            .mixin_key
            .as_ref()
            .ok_or_else(|| CrawlerError::Wbi("mixin key unavailable after fetch".into()))?;

        // 1. Add current unix timestamp.
        let wts = now_unix().to_string();
        params.insert("wts".to_string(), wts.clone());

        // 2. Sort params by key.
        let mut sorted: Vec<(&String, &String)> = params.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(b.0));

        // 3. Build query string.
        let query = sorted
            .iter()
            .map(|(k, v)| {
                let cleaned: String = v.chars().filter(|c| !CHARS_TO_STRIP.contains(c)).collect();
                format!("{}={}", url_encode(k), url_encode(&cleaned))
            })
            .collect::<Vec<_>>()
            .join("&");

        // 4. Append mixin key, compute MD5.
        let mut hasher = Md5::new();
        hasher.update(query.as_bytes());
        hasher.update(mixin_key.as_bytes());
        let w_rid = hex::encode(hasher.finalize());

        debug!(wts = %wts, w_rid = %w_rid, "WBI signature computed");

        // 5. Inject w_rid into params.
        params.insert("w_rid".to_string(), w_rid);

        Ok(())
    }

    /// Fetch or re-fetch WBI keys from `/x/web-interface/nav`.
    async fn fetch_wbi_keys(&self) -> Result<()> {
        let url = format!("{}/x/web-interface/nav", self.base_url);

        let resp: Value = self
            .client
            .get(&url)
            .header("Cookie", &self.cookie)
            .header("Referer", "https://www.bilibili.com/")
            .send()
            .await?
            .json()
            .await?;

        let code = resp.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);

        if code == -352 {
            return Err(CrawlerError::Wbi(
                "WBI key fetch rejected by risk control (code -352)".into(),
            ));
        }

        if code != 0 {
            let msg = resp
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            return Err(CrawlerError::Wbi(format!(
                "WBI key fetch failed: code={code}, message={msg}"
            )));
        }

        let wbi_img = resp
            .pointer("/data/wbi_img")
            .ok_or_else(|| CrawlerError::Wbi("missing data.wbi_img in nav response".into()))?;

        let img_url = wbi_img
            .get("img_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CrawlerError::Wbi("missing wbi_img.img_url".into()))?;

        let sub_url = wbi_img
            .get("sub_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CrawlerError::Wbi("missing wbi_img.sub_url".into()))?;

        let img_key = extract_key_from_url(img_url)?;
        let sub_key = extract_key_from_url(sub_url)?;

        let combined = format!("{img_key}{sub_key}");
        let mixin_key = get_mixin_key(&combined);

        if mixin_key.len() != 32 {
            return Err(CrawlerError::Wbi(format!(
                "derived mixin key has unexpected length {} (expected 32)",
                mixin_key.len()
            )));
        }

        let now = now_unix();
        let mut inner = self.inner.write().await;
        inner.mixin_key = Some(mixin_key);
        inner.key_expires_at = now + self.key_ttl_secs;

        debug!(
            img_key = %img_key,
            sub_key = %sub_key,
            expires_at = inner.key_expires_at,
            "WBI keys refreshed"
        );

        Ok(())
    }

    /// Ensure cached keys are available and not expired.
    async fn ensure_keys(&self) -> Result<()> {
        {
            let inner = self.inner.read().await;
            let now = now_unix();
            if inner.mixin_key.is_some() && now < inner.key_expires_at {
                return Ok(());
            }
        }
        warn!("WBI keys missing or expired — fetching");
        self.fetch_wbi_keys().await
    }
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before UNIX epoch")
        .as_secs()
}

fn url_encode(s: &str) -> String {
    let mut encoded = String::with_capacity(s.len() * 3);
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            b' ' => {
                encoded.push_str("%20");
            }
            _ => {
                encoded.push_str(&format!("%{byte:02X}"));
            }
        }
    }
    encoded
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mixin_key() {
        // Need at least 64 bytes since MIXIN_KEY_ENC_TAB max index is 63
        let orig = "7cd084941338484aae1ad9425b84077c4932caff0ff746eab6f01bf08b70ac45";
        let key = get_mixin_key(orig);
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_mixin_key_reference() {
        // From Bilibili WBI reference implementation
        let concat_key =
            "7cd084941338484aae1ad9425b84077c".to_string() + "4932caff0ff746eab6f01bf08b70ac45";
        assert_eq!(
            get_mixin_key(&concat_key),
            "ea1db124af3c7062474693fa704f4ff8"
        );
    }
}
