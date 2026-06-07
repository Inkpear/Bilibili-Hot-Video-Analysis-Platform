use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoDetail {
    pub bvid: String,
    pub aid: i64,
    pub cid: i64,
    pub title: String,
    pub description: String,
    pub tags: String,
    pub sub_category: String,
    pub duration: i32,
    pub publish_time: i64,
    pub up_id: String,
    pub up_name: String,
    pub up_avatar: String,
    pub view_count: i64,
    pub like_count: i64,
    pub coin_count: i64,
    pub favorite_count: i64,
    pub share_count: i64,
    pub reply_count: i64,
    pub danmaku_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpInfo {
    pub up_id: String,
    pub up_name: String,
    pub up_avatar: String,
    pub follower_count: i64,
    pub video_count: i64,
    pub sign: String,
    pub official_verify: String,
}
