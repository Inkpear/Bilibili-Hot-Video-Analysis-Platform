use serde::{Deserialize, Serialize};

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
