// backend/src/handlers.rs
use axum::extract::{State, Query};
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;
use crate::AppState;
use crate::models::*;

#[derive(Debug, Deserialize)]
pub struct SnapshotDateQuery {
    pub snapshot_date: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CategoryQuery {
    pub category: Option<String>,
    pub snapshot_date: Option<String>,
}

pub async fn overview(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SnapshotDateQuery>,
) -> Json<ApiResponse<OverviewData>> {
    // TODO: 查询数据库
    Json(ApiResponse {
        code: 0,
        message: "ok".to_string(),
        data: OverviewData {
            total_views: 0,
            total_likes: 0,
            partition_count: 0,
            hot_title: "".to_string(),
            new_video_count: 0,
        },
    })
}

pub async fn partition_heatmap(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SnapshotDateQuery>,
) -> Json<ApiResponse<PartitionHeatmapData>> {
    // TODO: 查询数据库
    Json(ApiResponse {
        code: 0,
        message: "ok".to_string(),
        data: PartitionHeatmapData {
            categories: vec![],
            dates: vec![],
            values: vec![],
        },
    })
}

pub async fn partitions(
    State(_state): State<Arc<AppState>>,
    Query(_query): Query<SnapshotDateQuery>,
) -> Json<ApiResponse<Vec<String>>> {
    Json(ApiResponse {
        code: 0,
        message: "ok".to_string(),
        data: vec![],
    })
}

pub async fn trend(
    State(_state): State<Arc<AppState>>,
    Query(_query): Query<CategoryQuery>,
) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse {
        code: 0,
        message: "ok".to_string(),
        data: serde_json::json!({}),
    })
}

pub async fn word_cloud(
    State(_state): State<Arc<AppState>>,
    Query(_query): Query<CategoryQuery>,
) -> Json<ApiResponse<Vec<WordCloudItem>>> {
    Json(ApiResponse {
        code: 0,
        message: "ok".to_string(),
        data: vec![],
    })
}

pub async fn videos(
    State(_state): State<Arc<AppState>>,
    Query(_query): Query<CategoryQuery>,
) -> Json<ApiResponse<Vec<VideoListItem>>> {
    Json(ApiResponse {
        code: 0,
        message: "ok".to_string(),
        data: vec![],
    })
}

pub async fn video_detail(
    State(_state): State<Arc<AppState>>,
    axum::extract::Path(bvid): axum::extract::Path<String>,
) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse {
        code: 0,
        message: "ok".to_string(),
        data: serde_json::json!({ "bvid": bvid }),
    })
}

pub async fn word_trend(
    State(_state): State<Arc<AppState>>,
    Query(_query): Query<CategoryQuery>,
) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse {
        code: 0,
        message: "ok".to_string(),
        data: serde_json::json!({ "trends": [] }),
    })
}

pub async fn snapshot_dates(
    State(_state): State<Arc<AppState>>,
) -> Json<ApiResponse<Vec<String>>> {
    Json(ApiResponse {
        code: 0,
        message: "ok".to_string(),
        data: vec![],
    })
}
