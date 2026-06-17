use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Danmaku {
    pub danmaku_id: i64,
    pub bvid: String,
    pub cid: i64,
    pub content: String,
    pub video_time: f64,
    pub mode: i32,
    pub fontsize: i32,
    pub color: i64,
    pub mid_hash: String,
    pub send_time: i64,
}
