use reqwest::Client;
use serde::Deserialize;
use anyhow::Result;
use crate::models::Comment;

#[derive(Debug, Deserialize)]
struct CommentResponse {
    code: i32,
    data: CommentData,
}

#[derive(Debug, Deserialize)]
struct CommentData {
    replies: Vec<Reply>,
}

#[derive(Debug, Deserialize)]
struct Reply {
    rpid: i64,
    mid: i64,
    member: Member,
    content: Content,
    like: i32,
    rcount: i32,
    ctime: i64,
}

#[derive(Debug, Deserialize)]
struct Member {
    mid: String,
    uname: String,
}

#[derive(Debug, Deserialize)]
struct Content {
    message: String,
}

pub async fn fetch_comments(
    client: &Client, 
    base_url: &str, 
    bvid: &str, 
    aid: i64,
    limit: usize
) -> Result<Vec<Comment>> {
    let url = format!(
        "{}/x/v2/reply?type=1&oid={}&pn=1&ps={}", 
        base_url, aid, limit
    );
    
    let resp: CommentResponse = client.get(&url).send().await?.json().await?;
    
    if resp.code != 0 {
        anyhow::bail!("API error: code={}", resp.code);
    }
    
    let comments = resp.data.replies.iter().map(|r| Comment {
        comment_id: r.rpid,
        bvid: bvid.to_string(),
        aid,
        user_id: r.mid,
        user_name: r.member.uname.clone(),
        content: r.content.message.clone(),
        like_count: r.like,
        reply_count: r.rcount,
        comment_time: r.ctime,
    }).collect();
    
    Ok(comments)
}
