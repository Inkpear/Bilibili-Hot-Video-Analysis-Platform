use serde::Deserialize;
use std::collections::HashSet;

use crate::config::Config;
use crate::error::{CrawlerError, Result};
use crate::models::Comment;

// ── Bilibili API response types ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct CommentResponse {
    code: i32,
    data: Option<CommentData>,
}

#[derive(Debug, Deserialize)]
struct CommentData {
    replies: Option<Vec<ReplyItem>>,
}

#[derive(Debug, Deserialize)]
struct ReplyItem {
    rpid: i64,
    mid: i64,
    like: i32,
    rcount: i32,
    ctime: i64,
    member: ReplyMember,
    content: ReplyContent,
}

#[derive(Debug, Deserialize)]
struct ReplyMember {
    uname: String,
}

#[derive(Debug, Deserialize)]
struct ReplyContent {
    message: String,
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Fetches hot comments for a single video from Bilibili `/x/v2/reply`.
///
/// Uses `sort=1` (hot order), `ps=20` per page, and iterates up to
/// `config.limits.max_comments_per_video` / 20 pages (capped at 10).
///
/// * `existing_ids` — comment IDs already persisted; those are skipped so
///   the returned vector contains only **new** comments.
///
/// Returns `(Vec<Comment>, bool)` where the bool is:
/// * `true`  — all pages fetched successfully (or comments closed).
/// * `false` — a network/API error caused early pagination termination.
pub async fn collect_comments(
    client: &reqwest::Client,
    config: &Config,
    bvid: &str,
    aid: i64,
    existing_ids: &HashSet<i64>,
) -> Result<(Vec<Comment>, bool)> {
    let page_size: usize = 20;
    let max_pages = std::cmp::min(
        config.limits.max_comments_per_video / page_size,
        10,
    )
    .max(1);

    let cookie = std::env::var(&config.api.cookie_env_name).map_err(|_| {
        CrawlerError::Config(format!(
            "environment variable '{}' is not set",
            config.api.cookie_env_name
        ))
    })?;

    let mut new_comments: Vec<Comment> = Vec::new();
    let mut success = true;

    for page in 1..=max_pages {
        let url = format!(
            "{}/x/v2/reply?type=1&oid={}&sort=1&ps={}&pn={}",
            config.api.base_url, aid, page_size, page,
        );

        let request_builder = client
            .get(&url)
            .header("User-Agent", &config.api.user_agent)
            .header("Cookie", &cookie)
            .header("Referer", "https://www.bilibili.com");

        tokio::time::sleep(std::time::Duration::from_millis(config.crawler.request_delay_ms)).await;

        let response = request_builder.send().await;

        let resp = match response {
            Ok(r) => match r.json::<CommentResponse>().await {
                Ok(body) => body,
                Err(e) => {
                    tracing::debug!(
                        "Failed to parse comment page {} for bvid {}: {}",
                        page,
                        bvid,
                        e
                    );
                    success = false;
                    break;
                }
            },
            Err(e) => {
                tracing::debug!(
                    "Failed to fetch comment page {} for bvid {}: {}",
                    page,
                    bvid,
                    e
                );
                success = false;
                break;
            }
        };

        // Error code 12061 = comments closed for this video — not a failure.
        if resp.code == 12061 {
            tracing::debug!("Comments closed for bvid {} (code 12061)", bvid);
            return Ok((Vec::new(), true));
        }

        if resp.code != 0 {
            tracing::debug!(
                "Comment API returned code {} for bvid {} page {}",
                resp.code,
                bvid,
                page,
            );
            success = false;
            break;
        }

        let replies = match resp.data.and_then(|d| d.replies) {
            Some(r) => r,
            None => {
                // No more comment pages available.
                break;
            }
        };

        if replies.is_empty() {
            break;
        }

        let page_reply_count = replies.len();
        for item in replies {
            if existing_ids.contains(&item.rpid) {
                continue;
            }
            new_comments.push(Comment {
                comment_id: item.rpid,
                bvid: bvid.to_string(),
                aid,
                user_id: item.mid,
                user_name: item.member.uname,
                content: item.content.message,
                like_count: item.like,
                reply_count: item.rcount,
                comment_time: item.ctime,
            });
        }

        // Fewer results than the page size signals the last page.
        if page_reply_count < page_size {
            break;
        }
    }

    Ok((new_comments, success))
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that a minimal valid JSON comment response deserialises
    /// correctly into the expected response types.
    #[test]
    fn test_deserialize_comment_response() {
        let json = r#"{"code":0,"data":{"replies":[{"rpid":1,"mid":100,"like":5,"rcount":2,"ctime":1700000000,"member":{"uname":"user1"},"content":{"message":"hello"}}]}}"#;
        let resp: CommentResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.code, 0);

        let data = resp.data.unwrap();
        let replies = data.replies.unwrap();
        assert_eq!(replies.len(), 1);

        assert_eq!(replies[0].rpid, 1);
        assert_eq!(replies[0].mid, 100);
        assert_eq!(replies[0].like, 5);
        assert_eq!(replies[0].rcount, 2);
        assert_eq!(replies[0].ctime, 1700000000);
        assert_eq!(replies[0].member.uname, "user1");
        assert_eq!(replies[0].content.message, "hello");
    }

    /// Verify that error code 12061 (comments disabled) is handled
    /// gracefully by the response deserialiser.
    #[test]
    fn test_deserialize_comments_closed() {
        let json = r#"{"code":12061,"data":null}"#;
        let resp: CommentResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.code, 12061);
        assert!(resp.data.is_none());
    }
}
