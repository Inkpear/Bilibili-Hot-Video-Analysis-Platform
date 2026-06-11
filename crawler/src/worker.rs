use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use tracing::info;

use crate::config::AccountConfig;
use crate::error::Result;
use crate::fingerprint::DeviceFingerprint;
use crate::rate_limit::PerWorkerRateLimiter;
use crate::wbi::WbiSigner;

/// A Worker represents one independent "device" — each owns its own
/// HTTP client, device fingerprint, cookie, WBI signer, and rate limiter.
pub struct Worker {
    pub id: usize,
    pub sessdata: String,
    pub bili_jct: String,
    pub dede_user_id: String,
    pub fingerprint: DeviceFingerprint,
    pub client: reqwest::Client,
    pub wbi: WbiSigner,
    pub rate_limiter: PerWorkerRateLimiter,
    pub fail_streak: u32,
}

impl Worker {
    /// Create a Worker from an AccountConfig.
    ///
    /// Automatically:
    /// 1. Builds a reqwest::Client (with optional fixed proxy)
    /// 2. Calls `/x/frontend/finger/spi` to get buvid3 + buvid4
    /// 3. Generates b_nut, _uuid, and picks a random browser profile
    /// 4. Creates an independent WbiSigner
    pub async fn create(
        id: usize,
        config: &AccountConfig,
        base_url: &str,
        delay_min_ms: u64,
        delay_max_ms: u64,
    ) -> Result<Self> {
        // Build bare client with optional fixed proxy (for SPI/GenWebTicket)
        let mut builder = reqwest::Client::builder().timeout(std::time::Duration::from_secs(30));

        if let Some(ref proxy_url) = config.proxy {
            if !proxy_url.is_empty() {
                let proxy = reqwest::Proxy::all(proxy_url)?;
                builder = builder.proxy(proxy);
                info!(worker = id, proxy = %proxy_url, "Using fixed proxy");
            }
        }

        let bare_client = builder.build()?;

        // Generate device fingerprint via SPI (uses bare client)
        let ua_override = config.user_agent.as_deref();
        let fingerprint = DeviceFingerprint::create(&bare_client, ua_override).await?;

        info!(
            worker = id,
            buvid3 = %fingerprint.buvid3,
            ua = %fingerprint.user_agent,
            "Worker created"
        );

        // Build a client with browser headers as defaults
        let mut default_headers = HeaderMap::new();
        default_headers.insert(
            reqwest::header::USER_AGENT,
            HeaderValue::from_str(&fingerprint.user_agent).unwrap(),
        );
        default_headers.insert(
            HeaderName::from_static("sec-ch-ua"),
            HeaderValue::from_str(&fingerprint.sec_ch_ua).unwrap(),
        );
        default_headers.insert(
            HeaderName::from_static("sec-ch-ua-platform"),
            HeaderValue::from_str(&fingerprint.sec_ch_ua_platform).unwrap(),
        );
        default_headers.insert(
            reqwest::header::ACCEPT_LANGUAGE,
            HeaderValue::from_str(&fingerprint.accept_language).unwrap(),
        );

        let mut builder2 = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .default_headers(default_headers);

        if let Some(ref proxy_url) = config.proxy {
            if !proxy_url.is_empty() {
                let proxy = reqwest::Proxy::all(proxy_url)?;
                builder2 = builder2.proxy(proxy);
            }
        }

        let client = builder2.build()?;

        // Build the full cookie string
        let cookie =
            fingerprint.cookie_string(&config.sessdata, &config.bili_jct, &config.dede_user_id);

        // Create independent WbiSigner (uses client with browser headers)
        let wbi = WbiSigner::new(cookie, base_url.to_string(), client.clone());

        // Create independent rate limiter
        let rate_limiter = PerWorkerRateLimiter::new(delay_min_ms, delay_max_ms);

        Ok(Self {
            id,
            sessdata: config.sessdata.clone(),
            bili_jct: config.bili_jct.clone(),
            dede_user_id: config.dede_user_id.clone(),
            fingerprint,
            client,
            wbi,
            rate_limiter,
            fail_streak: 0,
        })
    }

    /// Get the full cookie string (device fingerprint + session cookies).
    pub fn cookie(&self) -> String {
        self.fingerprint
            .cookie_string(&self.sessdata, &self.bili_jct, &self.dede_user_id)
    }

    /// Get a reference to the WBI signer.
    pub fn wbi(&self) -> &WbiSigner {
        &self.wbi
    }

    /// Record a successful request — resets fail streak and cooldown.
    pub fn record_success(&mut self) {
        self.fail_streak = 0;
        self.rate_limiter.reset_cooldown();
    }

    /// Record a failure — increments fail streak.
    pub fn record_failure(&mut self) {
        self.fail_streak += 1;
    }

    /// Record a 429 rate limit — sets cooldown with exponential backoff.
    pub fn record_rate_limit(&mut self, cooldown_secs: u64) {
        let backoff = cooldown_secs * 2u64.pow(self.fail_streak.min(4));
        self.rate_limiter.set_cooldown(backoff);
        self.fail_streak += 1;
    }
}
