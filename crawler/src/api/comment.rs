use std::collections::HashSet;

use serde::Deserialize;

use crate::error::{CrawlerError, Result};
use crate::models::Comment;

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

/// Fetch hot comments for a single video from `/x/v2/reply`.
///
/// Returns `(new_comments, all_pages_fetched)`.
/// Returns `Err(RateLimited)` on HTTP 429.
pub async fn collect_comments(
    client: &reqwest::Client,
    cookie: &str,
    base_url: &str,
    bvid: &str,
    aid: i64,
    max_comments: usize,
    existing_ids: &HashSet<i64>,
) -> Result<(Vec<Comment>, bool)> {
    let page_size: usize = 20;
    let max_pages = (max_comments / page_size).clamp(1, 10);

    let mut new_comments: Vec<Comment> = Vec::new();
    let mut success = true;

    for page in 1..=max_pages {
        let url = format!(
            "{}/x/v2/reply?type=1&oid={}&sort=1&ps={}&pn={}",
            base_url, aid, page_size, page,
        );

        let response = client
            .get(&url)
            .header("Cookie", cookie)
            .header("Referer", "https://www.bilibili.com")
            .send()
            .await;

        let resp = match response {
            Ok(r) => {
                if r.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    return Err(CrawlerError::RateLimited {
                        retry_after: Some(60),
                    });
                }
                match r.json::<CommentResponse>().await {
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
                }
            }
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

        // 12061 = comments closed — not a failure
        if resp.code == 12061 {
            return Ok((Vec::new(), true));
        }

        if resp.code != 0 {
            tracing::debug!(
                "Comment API returned code {} for bvid {} page {}",
                resp.code,
                bvid,
                page
            );
            success = false;
            break;
        }

        let replies = match resp.data.and_then(|d| d.replies) {
            Some(r) => r,
            None => break,
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

        if page_reply_count < page_size {
            break;
        }
    }

    Ok((new_comments, success))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_comment_response() {
        let json = r#"{"code":0,"data":{"replies":[{"rpid":1,"mid":100,"like":5,"rcount":2,"ctime":1700000000,"member":{"uname":"user1"},"content":{"message":"hello"}}]}}"#;
        let resp: CommentResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.code, 0);
        let replies = resp.data.unwrap().replies.unwrap();
        assert_eq!(replies.len(), 1);
    }

    #[test]
    fn test_deserialize_comments_closed() {
        let json = r#"{"code":12061,"data":null}"#;
        let resp: CommentResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.code, 12061);
    }
}
