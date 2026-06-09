use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use md5::{Digest, Md5};
use serde_json::Value;
use tracing::{debug, warn};

use crate::error::{CrawlerError, Result};

/// Substring table for mixin key derivation — exact same 64-entry table as the
/// Python reference implementation.  Only the first 32 indices are used.
const MIXIN_KEY_ENC_TAB: [usize; 64] = [
    46, 47, 18, 2, 53, 8, 23, 32, 15, 50, 10, 31, 58, 3, 45, 35, 27, 43, 5, 49, 33, 9, 42, 19,
    29, 28, 14, 39, 12, 38, 41, 13, 37, 48, 7, 16, 24, 55, 40, 61, 26, 17, 0, 1, 60, 51, 30, 4,
    22, 25, 54, 21, 56, 59, 6, 63, 57, 62, 11, 36, 20, 34, 44, 52,
];

/// Characters stripped from parameter values when computing the signature.
/// The actual params HashMap is **not** mutated — stripping is local to the
/// signature computation only.
const CHARS_TO_STRIP: &[char] = &['!', '\'', '(', ')', '*'];

/// Default TTL for cached WBI keys (4 hours).
const DEFAULT_KEY_TTL_SECS: u64 = 14_400;

// ---------------------------------------------------------------------------
// Pure helpers (testable)
// ---------------------------------------------------------------------------

/// Derive the 32-char mixin key from the 64-char concatenation of
/// `img_key + sub_key` by selecting characters at the positions given by
/// `MIXIN_KEY_ENC_TAB`.
fn get_mixin_key(orig: &str) -> String {
    MIXIN_KEY_ENC_TAB
        .iter()
        .filter_map(|&i| orig.chars().nth(i))
        .take(32)
        .collect()
}

/// Extract the filename **stem** (the part between the last `/` and the first
/// `.` after it) from a Bilibili `wbi_img` URL.
///
/// Example:
///   `"https://i0.hdslb.com/bfs/wbi/7cd084941338484aae1ad9425b84077c.png"`
///   → `"7cd084941338484aae1ad9425b84077c"`
fn extract_key_from_url(url: &str) -> Result<String> {
    let filename = url
        .rsplit('/')
        .next()
        .ok_or_else(|| CrawlerError::Wbi(format!("cannot extract filename from URL: {url}")))?;

    let stem = filename
        .split('.')
        .next()
        .ok_or_else(|| CrawlerError::Wbi(format!("cannot extract stem from filename: {filename}")))?;

    if stem.is_empty() {
        return Err(CrawlerError::Wbi(format!(
            "empty key extracted from URL: {url}"
        )));
    }

    Ok(stem.to_string())
}

// ---------------------------------------------------------------------------
// WbiSigner
// ---------------------------------------------------------------------------

/// Signs Bilibili API parameters with the WBI v3 scheme.
///
/// On first use (or after the TTL expires) the signer fetches the current
/// `img_key` / `sub_key` pair from `/x/web-interface/nav` and derives the
/// mixin key used for MD5 signing.
pub struct WbiSigner {
    cookie: String,
    base_url: String,
    client: reqwest::Client,

    /// Cached mixin key (32 chars).
    mixin_key: Option<String>,
    /// Unix timestamp at which the cached key expires.
    key_expires_at: u64,
    /// TTL in seconds for cached keys.
    key_ttl_secs: u64,
}

impl WbiSigner {
    /// Create a new signer.
    ///
    /// * `cookie` – full cookie header value (must include at least `SESSDATA`).
    /// * `base_url` – Bilibili API origin, e.g. `https://api.bilibili.com`.
    /// * `client` – a shared `reqwest::Client`.
    pub fn new(cookie: String, base_url: String, client: reqwest::Client) -> Self {
        Self {
            cookie,
            base_url,
            client,
            mixin_key: None,
            key_expires_at: 0,
            key_ttl_secs: DEFAULT_KEY_TTL_SECS,
        }
    }

    // -- public API ---------------------------------------------------------

    /// Sign `params` in-place by adding `wts` and `w_rid`.
    ///
    /// The original parameter values are **not** modified — character stripping
    /// happens only inside the signature computation.
    pub async fn sign_params(
        &mut self,
        params: &mut HashMap<String, String>,
    ) -> Result<()> {
        self.ensure_keys().await?;

        let mixin_key = self
            .mixin_key
            .as_ref()
            .ok_or_else(|| CrawlerError::Wbi("mixin key unavailable after fetch".into()))?;

        // 1. Add current unix timestamp.
        let wts = now_unix().to_string();
        params.insert("wts".to_string(), wts.clone());

        // 2. Sort params by key.
        let mut sorted: Vec<(&String, &String)> = params.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(b.0));

        // 3. Build query string — strip CHARS_TO_STRIP from values, URL-encode,
        //    join with `&`.
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

    // -- key management -----------------------------------------------------

    /// Fetch or re-fetch WBI keys from `/x/web-interface/nav`.
    async fn fetch_wbi_keys(&mut self) -> Result<()> {
        let url = format!("{}/x/web-interface/nav", self.base_url);

        let resp: Value = self
            .client
            .get(&url)
            .header("Cookie", &self.cookie)
            .header("Referer", "https://www.bilibili.com/")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36")
            .send()
            .await?
            .json()
            .await?;

        // Check for -352 (risk control) or other non-zero codes.
        let code = resp
            .get("code")
            .and_then(|v| v.as_i64())
            .unwrap_or(-1);

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
        self.mixin_key = Some(mixin_key);
        self.key_expires_at = now + self.key_ttl_secs;

        debug!(
            img_key = %img_key,
            sub_key = %sub_key,
            expires_at = self.key_expires_at,
            "WBI keys refreshed"
        );

        Ok(())
    }

    /// Ensure cached keys are available and not expired.
    async fn ensure_keys(&mut self) -> Result<()> {
        let now = now_unix();
        if self.mixin_key.is_none() || now >= self.key_expires_at {
            warn!("WBI keys missing or expired — fetching");
            self.fetch_wbi_keys().await?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

/// Current unix timestamp (seconds).
fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before UNIX epoch")
        .as_secs()
}

/// Minimal percent-encoding that covers the characters Bilibili expects
/// encoded in query strings.
fn url_encode(s: &str) -> String {
    let mut encoded = String::with_capacity(s.len() * 3);
    for byte in s.bytes() {
        match byte {
            // Unreserved characters — pass through.
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            b' ' => {
                // Bilibili uses %20, not `+`.
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
        let orig = "abc123def456ghi789jkl012mno345pqr678stu901vwx234yz";
        let key = get_mixin_key(orig);
        assert_eq!(key.len(), 32);
    }
}
