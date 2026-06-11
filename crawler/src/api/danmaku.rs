use std::collections::HashSet;

use prost::Message;

use crate::error::Result;
use crate::models::Danmaku;
use crate::protos::DmSegMobileReply;

/// Fetch danmaku (bullet comments) via the `seg.so` segmented protobuf endpoint.
///
/// - `danmaku_count`: total danmaku count from video detail, used to calculate max segments.
/// - `max_danmaku`: cap on how many danmaku to collect (e.g. 3000).
/// - `start_seg`: segment to resume from (0 = start from beginning).
///
/// Returns `(new_danmaku, last_segment, success_flag)`.
/// Returns `Err(RateLimited)` on HTTP 429 or Bilibili's `-304` anti-bot signal.
#[allow(clippy::too_many_arguments)]
pub async fn collect_danmaku(
    client: &reqwest::Client,
    cookie: &str,
    bvid: &str,
    cid: i64,
    danmaku_count: i64,
    max_danmaku: usize,
    start_seg: u32,
    existing_ids: &HashSet<i64>,
) -> Result<(Vec<Danmaku>, u32)> {
    // Each segment holds ~1000 danmaku. Calculate how many we actually need.
    let needed = (max_danmaku as i64).min(danmaku_count).max(1);
    let max_segments = ((needed + 999) / 1000).min(500) as u32;
    let first_seg = start_seg.max(1);

    tracing::debug!(
        cid = cid,
        danmaku_count = danmaku_count,
        max_danmaku = max_danmaku,
        max_segments = max_segments,
        start_seg = first_seg,
        "danmaku fetch plan"
    );

    let mut danmaku_list = Vec::with_capacity(max_danmaku.min(1000));
    let mut last_seg = first_seg.saturating_sub(1);

    for seg in first_seg..=max_segments {
        if danmaku_list.len() >= max_danmaku {
            break;
        }

        // Small delay between segments to avoid triggering anti-bot (-304)
        if seg > 1 {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        }

        let url = format!(
            "https://api.bilibili.com/x/v2/dm/web/seg.so?type=1&oid={}&segment_index={}",
            cid, seg,
        );

        let response = client
            .get(&url)
            .header("Cookie", cookie)
            .header("Referer", "https://www.bilibili.com/")
            .send()
            .await?;

        let status = response.status();

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            tracing::warn!(
                cid = cid,
                segment = seg,
                collected = danmaku_list.len(),
                last_seg = last_seg,
                "danmaku HTTP 429"
            );
            return Ok((danmaku_list, last_seg));
        }

        // Check for Bilibili's anti-bot 304 signal.
        // The server returns HTTP 304 with `bili-status-code: -304` header when it
        // considers the request suspicious (rate limiting / anti-bot).
        // A genuine 304 (without bili-status-code or with positive value) means
        // the segment genuinely has no new content.
        if status == reqwest::StatusCode::NOT_MODIFIED {
            let bili_code = response
                .headers()
                .get("bili-status-code")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("0");

            if bili_code.starts_with('-') {
                // Negative bili-status-code = anti-bot / rate limiting
                tracing::warn!(
                    cid = cid,
                    segment = seg,
                    bili_status_code = bili_code,
                    collected = danmaku_list.len(),
                    last_seg = last_seg,
                    "danmaku rate-limited by Bilibili anti-bot"
                );
                // Return partial results — caller decides if it's enough
                return Ok((danmaku_list, last_seg));
            }

            // Genuine 304 = no more content for this segment range
            tracing::debug!(
                cid = cid,
                segment = seg,
                "danmaku segment empty (genuine 304)"
            );
            break;
        }

        if !status.is_success() {
            // Non-200 (not 429, not 304) likely means danmaku disabled for this video
            tracing::info!(
                "danmaku unavailable (HTTP {}) for cid={}, marking as done",
                status,
                cid
            );
            return Ok((danmaku_list, last_seg));
        }

        let body = response.bytes().await?;

        // seg.so returns very short bodies when there are no more segments
        if body.len() <= 39 {
            break;
        }

        let reply = DmSegMobileReply::decode(&body[..])?;

        if reply.elems.is_empty() {
            break;
        }

        last_seg = seg;

        for elem in reply.elems {
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
                video_time: elem.progress as f64 / 1000.0,
                mode: elem.mode,
                fontsize: elem.fontsize,
                color: elem.color as i64,
                mid_hash: elem.mid_hash,
                send_time: elem.ctime,
            });
        }
    }

    Ok((danmaku_list, last_seg))
}
