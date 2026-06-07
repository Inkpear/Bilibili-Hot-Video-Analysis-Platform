// backend/src/models.rs
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub code: i32,
    pub message: String,
    pub data: T,
}

#[derive(Debug, Serialize)]
pub struct OverviewData {
    pub total_views: i64,
    pub total_likes: i64,
    pub partition_count: i32,
    pub hot_title: String,
    pub new_video_count: i32,
}

#[derive(Debug, Serialize)]
pub struct PartitionHeatmapData {
    pub categories: Vec<String>,
    pub dates: Vec<String>,
    pub values: Vec<Vec<f64>>,
}

#[derive(Debug, Serialize)]
pub struct VideoListItem {
    pub bvid: String,
    pub title: String,
    pub up_name: String,
    pub category: String,
    pub view_count: i64,
    pub like_count: i64,
}

#[derive(Debug, Serialize)]
pub struct WordCloudItem {
    pub word: String,
    pub count: i32,
}
