/// Task types that a Worker can pick up from the queue.
#[derive(Debug, Clone)]
pub enum Task {
    /// Fetch video detail from /x/web-interface/view.
    VideoDetail {
        bvid: String,
        ranked: Box<crate::api::ranking::RankedVideo>,
    },

    /// Fetch UP info.
    UpInfo { up_id: String, bvid: String },

    /// Fetch comments for a video.
    Comments { bvid: String, aid: i64 },

    /// Fetch danmaku for a video.
    Danmaku {
        bvid: String,
        cid: i64,
        danmaku_count: i64,
    },
}

impl Task {
    pub fn bvid(&self) -> &str {
        match self {
            Task::VideoDetail { bvid, .. } => bvid,
            Task::UpInfo { bvid, .. } => bvid,
            Task::Comments { bvid, .. } => bvid,
            Task::Danmaku { bvid, .. } => bvid,
        }
    }

    pub fn task_type_name(&self) -> &'static str {
        match self {
            Task::VideoDetail { .. } => "video_detail",
            Task::UpInfo { .. } => "up_info",
            Task::Comments { .. } => "comments",
            Task::Danmaku { .. } => "danmaku",
        }
    }
}
