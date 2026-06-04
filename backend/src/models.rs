use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub code: i32,
    pub message: String,
    pub data: T,
}

impl<T> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            code: 0,
            message: "ok".to_string(),
            data,
        }
    }
}

// ---- /api/overview ----

#[derive(Debug, Serialize)]
pub struct OverviewData {
    pub video_count: i32,
    pub category_count: i32,
    pub up_count: i32,
    pub total_views: i64,
    pub total_interactions: i64,
    pub latest_snapshot_date: Option<String>,
    pub update_time: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct OverviewRow {
    pub video_count: Option<i32>,
    pub category_count: Option<i32>,
    pub up_count: Option<i32>,
    pub total_views: Option<i64>,
    pub total_interactions: Option<i64>,
    pub latest_snapshot_date: Option<String>,
    pub update_time: Option<String>,
}

impl From<OverviewRow> for OverviewData {
    fn from(row: OverviewRow) -> Self {
        Self {
            video_count: row.video_count.unwrap_or(0),
            category_count: row.category_count.unwrap_or(0),
            up_count: row.up_count.unwrap_or(0),
            total_views: row.total_views.unwrap_or(0),
            total_interactions: row.total_interactions.unwrap_or(0),
            latest_snapshot_date: row.latest_snapshot_date,
            update_time: row.update_time.unwrap_or_default(),
        }
    }
}

impl Default for OverviewData {
    fn default() -> Self {
        Self {
            video_count: 0,
            category_count: 0,
            up_count: 0,
            total_views: 0,
            total_interactions: 0,
            latest_snapshot_date: None,
            update_time: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }
}

// ---- /api/hot-videos ----

#[derive(Debug, Serialize)]
pub struct HotVideoItem {
    pub bvid: String,
    pub title: String,
    pub category: String,
    pub up_name: String,
    pub view_count: i64,
    pub like_count: i64,
    pub coin_count: i64,
    pub favorite_count: i64,
    pub reply_count: i64,
    pub share_count: i64,
    pub rank_position: i32,
    pub heat_score: f64,
}

#[derive(Debug, sqlx::FromRow)]
pub struct HotVideoRow {
    pub bvid: Option<String>,
    pub title: Option<String>,
    pub category: Option<String>,
    pub up_name: Option<String>,
    pub view_count: Option<i64>,
    pub like_count: Option<i64>,
    pub coin_count: Option<i64>,
    pub favorite_count: Option<i64>,
    pub reply_count: Option<i64>,
    pub share_count: Option<i64>,
    pub rank_position: Option<i32>,
    pub heat_score: Option<f64>,
}

impl From<HotVideoRow> for HotVideoItem {
    fn from(row: HotVideoRow) -> Self {
        Self {
            bvid: row.bvid.unwrap_or_default(),
            title: row.title.unwrap_or_default(),
            category: row.category.unwrap_or_default(),
            up_name: row.up_name.unwrap_or_default(),
            view_count: row.view_count.unwrap_or(0),
            like_count: row.like_count.unwrap_or(0),
            coin_count: row.coin_count.unwrap_or(0),
            favorite_count: row.favorite_count.unwrap_or(0),
            reply_count: row.reply_count.unwrap_or(0),
            share_count: row.share_count.unwrap_or(0),
            rank_position: row.rank_position.unwrap_or(0),
            heat_score: row.heat_score.unwrap_or(0.0),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct HotVideosData {
    pub total: i64,
    pub videos: Vec<HotVideoItem>,
}

// ---- /api/partition-compare ----

#[derive(Debug, Serialize)]
pub struct PartitionCompareItem {
    pub category: String,
    pub hot_video_count: i32,
    pub avg_views: i64,
    pub avg_like_rate: f64,
    pub avg_coin_rate: f64,
    pub avg_fav_rate: f64,
    pub top_view_count: i64,
    pub total_views: i64,
    pub up_count: i32,
}

#[derive(Debug, sqlx::FromRow)]
pub struct PartitionCompareRow {
    pub category: Option<String>,
    pub hot_video_count: Option<i32>,
    pub avg_views: Option<i64>,
    pub avg_like_rate: Option<f64>,
    pub avg_coin_rate: Option<f64>,
    pub avg_fav_rate: Option<f64>,
    pub top_view_count: Option<i64>,
    pub total_views: Option<i64>,
    pub up_count: Option<i32>,
}

impl From<PartitionCompareRow> for PartitionCompareItem {
    fn from(row: PartitionCompareRow) -> Self {
        Self {
            category: row.category.unwrap_or_default(),
            hot_video_count: row.hot_video_count.unwrap_or(0),
            avg_views: row.avg_views.unwrap_or(0),
            avg_like_rate: row.avg_like_rate.unwrap_or(0.0),
            avg_coin_rate: row.avg_coin_rate.unwrap_or(0.0),
            avg_fav_rate: row.avg_fav_rate.unwrap_or(0.0),
            top_view_count: row.top_view_count.unwrap_or(0),
            total_views: row.total_views.unwrap_or(0),
            up_count: row.up_count.unwrap_or(0),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PartitionCompareData {
    pub partitions: Vec<PartitionCompareItem>,
}

// ---- /api/lifecycle ----

#[derive(Debug, Serialize)]
pub struct LifecyclePoint {
    pub snapshot_date: String,
    pub snapshot_time: String,
    pub view_count: i64,
    pub like_count: i64,
    pub rank_position: i32,
    pub rank_source: String,
    pub rank_name: String,
    pub view_growth: i64,
    pub rank_change: i32,
}

#[derive(Debug, sqlx::FromRow)]
pub struct LifecycleRow {
    pub bvid: Option<String>,
    pub title: Option<String>,
    pub category: Option<String>,
    pub snapshot_date: Option<String>,
    pub snapshot_time: Option<String>,
    pub view_count: Option<i64>,
    pub like_count: Option<i64>,
    pub rank_position: Option<i32>,
    pub rank_source: Option<String>,
    pub rank_name: Option<String>,
    pub view_growth: Option<i64>,
    pub rank_change: Option<i32>,
}

impl LifecycleRow {
    pub fn to_point(&self) -> LifecyclePoint {
        LifecyclePoint {
            snapshot_date: self.snapshot_date.clone().unwrap_or_default(),
            snapshot_time: self.snapshot_time.clone().unwrap_or_default(),
            view_count: self.view_count.unwrap_or(0),
            like_count: self.like_count.unwrap_or(0),
            rank_position: self.rank_position.unwrap_or(0),
            rank_source: self.rank_source.clone().unwrap_or_default(),
            rank_name: self.rank_name.clone().unwrap_or_default(),
            view_growth: self.view_growth.unwrap_or(0),
            rank_change: self.rank_change.unwrap_or(0),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct LifecycleData {
    pub bvid: String,
    pub title: String,
    pub category: String,
    pub timeline: Vec<LifecyclePoint>,
}

#[derive(Debug, Serialize)]
pub struct LifecycleTopData {
    pub videos: Vec<LifecycleData>,
}

// ---- /api/partition-detail ----

#[derive(Debug, sqlx::FromRow)]
pub struct PartitionOverviewRow {
    pub hot_video_count: Option<i32>,
    pub avg_views: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct PartitionDetailVideo {
    pub bvid: String,
    pub title: String,
    pub view_count: i64,
    pub heat_score: f64,
    pub rank_position: i32,
}

#[derive(Debug, sqlx::FromRow)]
pub struct PartitionDetailVideoRow {
    pub bvid: Option<String>,
    pub title: Option<String>,
    pub view_count: Option<i64>,
    pub heat_score: Option<f64>,
    pub rank_position: Option<i32>,
}

impl From<PartitionDetailVideoRow> for PartitionDetailVideo {
    fn from(row: PartitionDetailVideoRow) -> Self {
        Self {
            bvid: row.bvid.unwrap_or_default(),
            title: row.title.unwrap_or_default(),
            view_count: row.view_count.unwrap_or(0),
            heat_score: row.heat_score.unwrap_or(0.0),
            rank_position: row.rank_position.unwrap_or(0),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PartitionDetailUp {
    pub up_id: String,
    pub up_name: String,
    pub follower_count: i64,
    pub hot_video_count: i64,
    pub total_views: i64,
}

#[derive(Debug, sqlx::FromRow)]
pub struct PartitionDetailUpRow {
    pub up_id: Option<String>,
    pub up_name: Option<String>,
    pub follower_count: Option<i64>,
    pub hot_video_count: Option<i64>,
    pub total_views: Option<i64>,
}

impl From<PartitionDetailUpRow> for PartitionDetailUp {
    fn from(row: PartitionDetailUpRow) -> Self {
        Self {
            up_id: row.up_id.unwrap_or_default(),
            up_name: row.up_name.unwrap_or_default(),
            follower_count: row.follower_count.unwrap_or(0),
            hot_video_count: row.hot_video_count.unwrap_or(0),
            total_views: row.total_views.unwrap_or(0),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PartitionDetailData {
    pub category: String,
    pub hot_video_count: i32,
    pub avg_views: i64,
    pub videos: Vec<PartitionDetailVideo>,
    pub top_ups: Vec<PartitionDetailUp>,
}

// ---- /api/interaction ----

#[derive(Debug, Serialize)]
pub struct InteractionData {
    pub bvid: String,
    pub view_count: i64,
    pub like_count: i64,
    pub coin_count: i64,
    pub favorite_count: i64,
    pub reply_count: i64,
    pub share_count: i64,
    pub like_rate: f64,
    pub coin_rate: f64,
    pub fav_rate: f64,
    pub reply_rate: f64,
    pub share_rate: f64,
}

#[derive(Debug, sqlx::FromRow)]
pub struct InteractionRow {
    pub bvid: Option<String>,
    pub view_count: Option<i64>,
    pub like_count: Option<i64>,
    pub coin_count: Option<i64>,
    pub favorite_count: Option<i64>,
    pub reply_count: Option<i64>,
    pub share_count: Option<i64>,
}

impl From<InteractionRow> for InteractionData {
    fn from(row: InteractionRow) -> Self {
        let view_count = row.view_count.unwrap_or(0);
        let like_count = row.like_count.unwrap_or(0);
        let coin_count = row.coin_count.unwrap_or(0);
        let favorite_count = row.favorite_count.unwrap_or(0);
        let reply_count = row.reply_count.unwrap_or(0);
        let share_count = row.share_count.unwrap_or(0);

        let v = view_count as f64;
        let (like_rate, coin_rate, fav_rate, reply_rate, share_rate) = if v > 0.0 {
            (
                like_count as f64 / v,
                coin_count as f64 / v,
                favorite_count as f64 / v,
                reply_count as f64 / v,
                share_count as f64 / v,
            )
        } else {
            (0.0, 0.0, 0.0, 0.0, 0.0)
        };

        Self {
            bvid: row.bvid.unwrap_or_default(),
            view_count,
            like_count,
            coin_count,
            favorite_count,
            reply_count,
            share_count,
            like_rate,
            coin_rate,
            fav_rate,
            reply_rate,
            share_rate,
        }
    }
}

// ---- /api/interaction-max ----

#[derive(Debug, Serialize)]
pub struct InteractionMaxData {
    pub max_like_rate: f64,
    pub max_coin_rate: f64,
    pub max_fav_rate: f64,
    pub max_reply_rate: f64,
    pub max_share_rate: f64,
}

#[derive(Debug, sqlx::FromRow)]
pub struct InteractionMaxRow {
    pub max_like_rate: Option<f64>,
    pub max_coin_rate: Option<f64>,
    pub max_fav_rate: Option<f64>,
    pub max_reply_rate: Option<f64>,
    pub max_share_rate: Option<f64>,
}

impl From<InteractionMaxRow> for InteractionMaxData {
    fn from(row: InteractionMaxRow) -> Self {
        Self {
            max_like_rate: row.max_like_rate.unwrap_or(0.0),
            max_coin_rate: row.max_coin_rate.unwrap_or(0.0),
            max_fav_rate: row.max_fav_rate.unwrap_or(0.0),
            max_reply_rate: row.max_reply_rate.unwrap_or(0.0),
            max_share_rate: row.max_share_rate.unwrap_or(0.0),
        }
    }
}

// ---- /api/word-cloud ----

#[derive(Debug, Serialize)]
pub struct WordCloudItem {
    pub name: String,
    pub value: i32,
}

#[derive(Debug, sqlx::FromRow)]
pub struct WordCloudRow {
    pub name: Option<String>,
    pub value: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct WordCloudData {
    pub source: String,
    pub category: String,
    pub words: Vec<WordCloudItem>,
}

// ---- /api/hot-comments ----

#[derive(Debug, Serialize)]
pub struct HotCommentItem {
    pub bvid: String,
    pub title: String,
    pub category: String,
    pub up_name: String,
    pub user_name: String,
    pub content: String,
    pub like_count: i64,
    pub reply_count: i64,
    pub comment_time: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct HotCommentRow {
    pub bvid: Option<String>,
    pub title: Option<String>,
    pub category: Option<String>,
    pub up_name: Option<String>,
    pub user_name: Option<String>,
    pub content: Option<String>,
    pub like_count: Option<i64>,
    pub reply_count: Option<i64>,
    pub comment_time: Option<String>,
}

impl From<HotCommentRow> for HotCommentItem {
    fn from(row: HotCommentRow) -> Self {
        Self {
            bvid: row.bvid.unwrap_or_default(),
            title: row.title.unwrap_or_default(),
            category: row.category.unwrap_or_default(),
            up_name: row.up_name.unwrap_or_default(),
            user_name: row.user_name.unwrap_or_default(),
            content: row.content.unwrap_or_default(),
            like_count: row.like_count.unwrap_or(0),
            reply_count: row.reply_count.unwrap_or(0),
            comment_time: row.comment_time,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct HotCommentsData {
    pub comments: Vec<HotCommentItem>,
}

// ---- query parameters ----

#[derive(Debug, serde::Deserialize)]
pub struct HotVideosQuery {
    pub sort: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub category: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct LifecycleQuery {
    pub bvid: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct LifecycleTopQuery {
    pub limit: Option<i64>,
    pub sort: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct PartitionDetailQuery {
    pub category: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct InteractionQuery {
    pub bvid: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct WordCloudQuery {
    pub source: Option<String>,
    pub category: Option<String>,
    pub bvid: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct HotCommentsQuery {
    pub category: Option<String>,
    pub bvid: Option<String>,
    pub limit: Option<i64>,
}
