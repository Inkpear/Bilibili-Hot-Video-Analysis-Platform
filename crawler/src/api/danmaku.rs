use std::collections::HashSet;
use std::sync::Mutex;
use std::time::Instant;

use prost::Message;

use crate::config::Config;
use crate::error::Result;
use crate::models::Danmaku;
use crate::protos::DmSegMobileReply;

// ── Global cooldown state ────────────────────────────────────────────────────
// When too many 429s arrive in a row, all danmaku collection pauses for a
// configurable duration so the crawler can back off gracefully.

/// Timestamp when the cooldown expires. `None` = not in cooldown.
static COOLDOWN_STATE: Mutex<Option<Instant>> = Mutex::new(None);

/// Number of consecutive 429 responses seen so far.
static CONSECUTIVE_429_COUNT: Mutex<u32> = Mutex::new(0);

/// Returns `true` if the global danmaku cooldown is currently active.
///
/// Also clears the cooldown flag (and resets the 429 counter) once the
/// cooldown period has elapsed, so subsequent calls are cheap.
pub fn is_in_cooldown(_cooldown_secs: u64) -> bool {
    let mut guard = COOLDOWN_STATE.lock().unwrap();
    match *guard {
        Some(expiry) => {
            if Instant::now() >= expiry {
                // Cooldown expired — clear it.
                *guard = None;
                *CONSECUTIVE_429_COUNT.lock().unwrap() = 0;
                false
            } else {
                true
            }
        }
        None => false,
    }
}

/// Record a 429 response.  When the consecutive count hits
/// `max_consecutive_429` the global cooldown is activated for
/// `cooldown_secs` seconds.
fn record_429(max_consecutive_429: u32, cooldown_secs: u64) {
    let mut count = CONSECUTIVE_429_COUNT.lock().unwrap();
    *count += 1;
    if *count >= max_consecutive_429 {
        let mut guard = COOLDOWN_STATE.lock().unwrap();
        *guard = Some(
            Instant::now() + std::time::Duration::from_secs(cooldown_secs),
        );
    }
}

/// Reset the consecutive-429 counter (called after a successful request).
fn reset_429_count() {
    *CONSECUTIVE_429_COUNT.lock().unwrap() = 0;
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Fetches danmaku (bullet comments) for a video via the `seg.so` segmented
/// protobuf endpoint.
///
/// # Arguments
/// * `client`   — shared HTTP client
/// * `config`   — application config (holds limits / user-agent)
/// * `bvid`     — BV id of the video
/// * `cid`      — cid of the video page
/// * `existing_ids` — set of danmaku IDs already persisted; these are skipped
///
/// # Returns
/// `(new_danmaku, success_flag)` where `success_flag` is `false` when the
/// request was blocked by the global 429 cooldown (the vec will be empty).
pub async fn collect_danmaku(
    client: &reqwest::Client,
    config: &Config,
    bvid: &str,
    cid: i64,
    existing_ids: &HashSet<i64>,
) -> Result<(Vec<Danmaku>, bool)> {
    // ── Global cooldown gate ─────────────────────────────────────────────
    if is_in_cooldown(config.limits.danmaku_429_cooldown_secs) {
        tracing::debug!("danmaku collection skipped — global 429 cooldown active");
        return Ok((Vec::new(), false));
    }

    let max_danmaku = config.limits.max_danmaku_per_video;
    let mut danmaku_list = Vec::with_capacity(max_danmaku.min(500));
    let mut success = true;

    for seg in 1..=500_u32 {
        if danmaku_list.len() >= max_danmaku {
            break;
        }

        let url = format!(
            "https://api.bilibili.com/x/v2/dm/web/seg.so?type=1&oid={}&segment_index={}",
            cid, seg,
        );

        tokio::time::sleep(std::time::Duration::from_millis(config.crawler.request_delay_ms)).await;
        let response = client
            .get(&url)
            .header("User-Agent", &config.api.user_agent)
            .send()
            .await?;

        let status = response.status();

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            record_429(
                config.limits.max_consecutive_danmaku_429,
                config.limits.danmaku_429_cooldown_secs,
            );
            if is_in_cooldown(config.limits.danmaku_429_cooldown_secs) {
                tracing::warn!(
                    "danmaku 429 cooldown triggered after {} consecutive 429s",
                    config.limits.max_consecutive_danmaku_429,
                );
                success = false;
                break;
            }
            // Not yet in cooldown — skip this segment and try the next.
            continue;
        }

        if !status.is_success() {
            tracing::warn!(
                "danmaku seg {} returned HTTP {} for cid={}",
                seg,
                status,
                cid,
            );
            // Treat non-success as end-of-stream for this video.
            break;
        }

        // Successful response — reset the consecutive-429 counter.
        reset_429_count();

        let body = response.bytes().await?;

        // seg.so returns very short bodies when there are no more segments
        // or when the IP is limited (typically ≤ 39 bytes of framing).
        if body.len() <= 39 {
            break;
        }

        let reply = DmSegMobileReply::decode(&body[..])?;

        if reply.elems.is_empty() {
            break;
        }

        for elem in reply.elems {
            // Skip already-collected danmaku.
            if existing_ids.contains(&elem.id) {
                continue;
            }
            if danmaku_list.len() >= max_danmaku {
                break;
            }

            danmaku_list.push(Danmaku {
                danmaku_id: elem.id,
                bvid: bvid.to_string(),
                cid,
                content: elem.content,
                // `progress` is in milliseconds; convert to seconds.
                video_time: elem.progress as f64 / 1000.0,
                mode: elem.mode,
                fontsize: elem.fontsize,
                color: elem.color as i64,
                mid_hash: elem.mid_hash,
                send_time: elem.ctime,
            });
        }
    }

    Ok((danmaku_list, success))
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cooldown_state_starts_inactive() {
        // Ensure the cooldown is not active by default.
        assert!(!is_in_cooldown(60));
    }

    #[test]
    fn test_record_429_activates_cooldown() {
        // Reset global state for a clean test.
        *COOLDOWN_STATE.lock().unwrap() = None;
        *CONSECUTIVE_429_COUNT.lock().unwrap() = 0;

        // With max_consecutive = 2, the first 429 should NOT trigger cooldown.
        record_429(2, 60);
        assert!(!is_in_cooldown(60));

        // The second 429 should trigger it.
        record_429(2, 60);
        assert!(is_in_cooldown(60));

        // Cleanup.
        *COOLDOWN_STATE.lock().unwrap() = None;
        *CONSECUTIVE_429_COUNT.lock().unwrap() = 0;
    }

    #[test]
    fn test_cooldown_expires() {
        // Set a cooldown that already expired (0-second duration from "now"
        // is effectively instant on fast machines, so we set a 0-second
        // cooldown in the past).
        {
            let mut guard = COOLDOWN_STATE.lock().unwrap();
            // Set expiry to 1 nanosecond ago.
            *guard = Some(
                Instant::now() - std::time::Duration::from_nanos(1),
            );
        }
        *CONSECUTIVE_429_COUNT.lock().unwrap() = 5;

        // Should report expired and clear state.
        assert!(!is_in_cooldown(60));
        assert!(COOLDOWN_STATE.lock().unwrap().is_none());
        assert_eq!(*CONSECUTIVE_429_COUNT.lock().unwrap(), 0);
    }

    #[test]
    fn test_reset_429_count() {
        *CONSECUTIVE_429_COUNT.lock().unwrap() = 10;
        reset_429_count();
        assert_eq!(*CONSECUTIVE_429_COUNT.lock().unwrap(), 0);
    }
}
