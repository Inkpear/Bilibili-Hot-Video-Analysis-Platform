use axum::extract::{Query, State};
use axum::Json;
use sqlx::MySqlPool;

use crate::error::AppError;
use crate::models::*;

pub async fn overview(
    State(pool): State<MySqlPool>,
) -> Result<Json<ApiResponse<OverviewData>>, AppError> {
    let row = sqlx::query_as!(
        OverviewRow,
        r#"SELECT
            video_count,
            category_count,
            up_count,
            total_views,
            total_interactions,
            latest_snapshot_date,
            DATE_FORMAT(update_time, '%Y-%m-%d %H:%i:%s') AS `update_time: String`
        FROM ads_screen_overview
        ORDER BY update_time DESC
        LIMIT 1"#
    )
    .fetch_optional(&pool)
    .await?;

    let data = match row {
        Some(r) => OverviewData::from(r),
        None => OverviewData::default(),
    };

    Ok(Json(ApiResponse::ok(data)))
}

pub async fn hot_videos(
    State(pool): State<MySqlPool>,
    Query(query): Query<HotVideosQuery>,
) -> Result<Json<ApiResponse<HotVideosData>>, AppError> {
    let sort = query.sort.as_deref().unwrap_or("heat");
    let limit = query.limit.unwrap_or(20);
    let offset = query.offset.unwrap_or(0);

    if limit < 0 || limit > 100 {
        return Err(AppError::bad_request("limit must be between 0 and 100"));
    }
    if offset < 0 {
        return Err(AppError::bad_request("offset must be >= 0"));
    }

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) AS cnt FROM ads_hot_video_rank")
        .fetch_one(&pool)
        .await?;

    if limit == 0 {
        return Ok(Json(ApiResponse::ok(HotVideosData {
            total: total.0,
            videos: vec![],
        })));
    }

    let rows = match sort {
        "heat" => {
            sqlx::query_as!(
                HotVideoRow,
                r#"SELECT
                    bvid, title, category, up_name,
                    view_count, like_count, coin_count, favorite_count,
                    reply_count, share_count, rank_position, heat_score
                FROM ads_hot_video_rank
                ORDER BY heat_score DESC
                LIMIT ? OFFSET ?"#,
                limit,
                offset
            )
            .fetch_all(&pool)
            .await?
        }
        "views" => {
            sqlx::query_as!(
                HotVideoRow,
                r#"SELECT
                    bvid, title, category, up_name,
                    view_count, like_count, coin_count, favorite_count,
                    reply_count, share_count, rank_position, heat_score
                FROM ads_hot_video_rank
                ORDER BY view_count DESC
                LIMIT ? OFFSET ?"#,
                limit,
                offset
            )
            .fetch_all(&pool)
            .await?
        }
        "likes" => {
            sqlx::query_as!(
                HotVideoRow,
                r#"SELECT
                    bvid, title, category, up_name,
                    view_count, like_count, coin_count, favorite_count,
                    reply_count, share_count, rank_position, heat_score
                FROM ads_hot_video_rank
                ORDER BY like_count DESC
                LIMIT ? OFFSET ?"#,
                limit,
                offset
            )
            .fetch_all(&pool)
            .await?
        }
        "coins" => {
            sqlx::query_as!(
                HotVideoRow,
                r#"SELECT
                    bvid, title, category, up_name,
                    view_count, like_count, coin_count, favorite_count,
                    reply_count, share_count, rank_position, heat_score
                FROM ads_hot_video_rank
                ORDER BY coin_count DESC
                LIMIT ? OFFSET ?"#,
                limit,
                offset
            )
            .fetch_all(&pool)
            .await?
        }
        _ => {
            return Err(AppError::bad_request(
                "invalid sort, allowed values: heat, views, likes, coins",
            ));
        }
    };

    let videos = rows.into_iter().map(HotVideoItem::from).collect();

    Ok(Json(ApiResponse::ok(HotVideosData {
        total: total.0,
        videos,
    })))
}

pub async fn partition_compare(
    State(pool): State<MySqlPool>,
) -> Result<Json<ApiResponse<PartitionCompareData>>, AppError> {
    let rows = sqlx::query_as!(
        PartitionCompareRow,
        r#"SELECT
            category,
            hot_video_count,
            avg_views,
            avg_like_rate,
            avg_coin_rate,
            avg_fav_rate,
            top_view_count,
            total_views,
            up_count
        FROM ads_partition_compare
        ORDER BY total_views DESC"#
    )
    .fetch_all(&pool)
    .await?;

    let partitions = rows.into_iter().map(PartitionCompareItem::from).collect();

    Ok(Json(ApiResponse::ok(PartitionCompareData { partitions })))
}

pub async fn lifecycle(
    State(pool): State<MySqlPool>,
    Query(query): Query<LifecycleQuery>,
) -> Result<Json<ApiResponse<Option<LifecycleData>>>, AppError> {
    let bvid = match query.bvid {
        Some(b) if !b.is_empty() => b,
        _ => return Err(AppError::bad_request("bvid is required")),
    };

    let rows = sqlx::query_as!(
        LifecycleRow,
        r#"SELECT
            bvid,
            title,
            category,
            snapshot_date,
            snapshot_time,
            view_count,
            like_count,
            rank_position,
            rank_source,
            rank_name,
            view_growth,
            rank_change
        FROM ads_video_lifecycle
        WHERE bvid = ?
        ORDER BY snapshot_id ASC"#,
        bvid
    )
    .fetch_all(&pool)
    .await?;

    if rows.is_empty() {
        return Ok(Json(ApiResponse::ok(None)));
    }

    let first = &rows[0];
    let timeline = rows.iter().map(|r| r.to_point()).collect();

    Ok(Json(ApiResponse::ok(Some(LifecycleData {
        bvid: first.bvid.clone().unwrap_or_default(),
        title: first.title.clone().unwrap_or_default(),
        category: first.category.clone().unwrap_or_default(),
        timeline,
    }))))
}

pub async fn lifecycle_top(
    State(pool): State<MySqlPool>,
    Query(query): Query<LifecycleTopQuery>,
) -> Result<Json<ApiResponse<LifecycleTopData>>, AppError> {
    let limit = query.limit.unwrap_or(5);
    if limit < 0 || limit > 100 {
        return Err(AppError::bad_request("limit must be between 0 and 100"));
    }

    if limit == 0 {
        return Ok(Json(ApiResponse::ok(LifecycleTopData { videos: vec![] })));
    }

    let top_bvids: Vec<(String,)> = sqlx::query_as(
        r#"SELECT h.bvid
           FROM ads_hot_video_rank h
           WHERE EXISTS (
             SELECT 1 FROM ads_video_lifecycle l WHERE l.bvid = h.bvid
           )
           ORDER BY h.heat_score DESC
           LIMIT ?"#,
    )
    .bind(limit)
    .fetch_all(&pool)
    .await?;

    let mut videos = Vec::new();
    for (bvid,) in top_bvids {
        let rows = sqlx::query_as!(
            LifecycleRow,
            r#"SELECT
                bvid,
                title,
                category,
                snapshot_date,
                snapshot_time,
                view_count,
                like_count,
                rank_position,
                rank_source,
                rank_name,
                view_growth,
                rank_change
            FROM ads_video_lifecycle
            WHERE bvid = ?
            ORDER BY snapshot_id ASC"#,
            bvid
        )
        .fetch_all(&pool)
        .await?;

        if let Some(first) = rows.first() {
            let timeline = rows.iter().map(|r| r.to_point()).collect();

            videos.push(LifecycleData {
                bvid: first.bvid.clone().unwrap_or_default(),
                title: first.title.clone().unwrap_or_default(),
                category: first.category.clone().unwrap_or_default(),
                timeline,
            });
        }
    }

    Ok(Json(ApiResponse::ok(LifecycleTopData { videos })))
}

pub async fn partition_detail(
    State(pool): State<MySqlPool>,
    Query(query): Query<PartitionDetailQuery>,
) -> Result<Json<ApiResponse<PartitionDetailData>>, AppError> {
    let category = match query.category {
        Some(c) if !c.is_empty() => c,
        _ => return Err(AppError::bad_request("category is required")),
    };

    let overview = sqlx::query_as!(
        PartitionOverviewRow,
        r#"SELECT hot_video_count, avg_views
           FROM ads_partition_compare
           WHERE category = ?
           LIMIT 1"#,
        category
    )
    .fetch_optional(&pool)
    .await?;

    let (hot_video_count, avg_views) = match overview {
        Some(o) => (o.hot_video_count.unwrap_or(0), o.avg_views.unwrap_or(0)),
        None => (0, 0i64),
    };

    let video_rows = sqlx::query_as!(
        PartitionDetailVideoRow,
        r#"SELECT
            bvid, title, view_count, heat_score, rank_position
        FROM ads_hot_video_rank
        WHERE category = ?
        ORDER BY heat_score DESC
        LIMIT 10"#,
        category
    )
    .fetch_all(&pool)
    .await?;

    let videos = video_rows
        .into_iter()
        .map(PartitionDetailVideo::from)
        .collect();

    let up_rows = sqlx::query_as!(
        PartitionDetailUpRow,
        r#"SELECT DISTINCT
            u.up_id,
            u.up_name,
            u.follower_count,
            u.hot_video_count,
            u.total_views
        FROM ads_up_summary u
        JOIN ads_hot_video_rank h ON h.up_name = u.up_name
        WHERE h.category = ?
        ORDER BY u.total_views DESC
        LIMIT 10"#,
        category
    )
    .fetch_all(&pool)
    .await?;

    let top_ups = up_rows.into_iter().map(PartitionDetailUp::from).collect();

    Ok(Json(ApiResponse::ok(PartitionDetailData {
        category,
        hot_video_count,
        avg_views,
        videos,
        top_ups,
    })))
}

pub async fn tag_cloud(
    State(pool): State<MySqlPool>,
    Query(query): Query<TagCloudQuery>,
) -> Result<Json<ApiResponse<TagCloudData>>, AppError> {
    let category = match query.category {
        Some(c) if !c.is_empty() => c,
        _ => "全站".to_string(),
    };

    let rows = sqlx::query_as!(
        TagCloudRow,
        r#"SELECT
            tag AS `name: String`,
            tag_count AS `value: i32`
        FROM ads_tag_cloud
        WHERE category = ?
        ORDER BY tag_count DESC
        LIMIT 100"#,
        category
    )
    .fetch_all(&pool)
    .await?;

    let tags = rows
        .into_iter()
        .map(|r| TagCloudItem {
            name: r.name.unwrap_or_default(),
            value: r.value.unwrap_or(0),
        })
        .collect();

    Ok(Json(ApiResponse::ok(TagCloudData { tags })))
}

pub async fn interaction(
    State(pool): State<MySqlPool>,
    Query(query): Query<InteractionQuery>,
) -> Result<Json<ApiResponse<Option<InteractionData>>>, AppError> {
    let bvid = match query.bvid {
        Some(b) if !b.is_empty() => b,
        _ => return Err(AppError::bad_request("bvid is required")),
    };

    let row = sqlx::query_as!(
        InteractionRow,
        r#"SELECT
            bvid,
            view_count,
            like_count,
            coin_count,
            favorite_count,
            reply_count,
            share_count
        FROM ads_hot_video_rank
        WHERE bvid = ?
        LIMIT 1"#,
        bvid
    )
    .fetch_optional(&pool)
    .await?;

    match row {
        Some(r) => Ok(Json(ApiResponse::ok(Some(InteractionData::from(r))))),
        None => Ok(Json(ApiResponse::ok(None))),
    }
}
