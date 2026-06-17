use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub comment_id: i64,
    pub bvid: String,
    pub aid: i64,
    pub user_id: i64,
    pub user_name: String,
    pub content: String,
    pub like_count: i32,
    pub reply_count: i32,
    pub comment_time: i64,
}
