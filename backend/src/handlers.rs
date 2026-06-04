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
    let category = query.category.as_deref();

    if !(0..=500).contains(&limit) {
        return Err(AppError::bad_request("limit must be between 0 and 500"));
    }
    if offset < 0 {
        return Err(AppError::bad_request("offset must be >= 0"));
    }

    let total: (i64,) = if let Some(cat) = category {
        sqlx::query_as("SELECT COUNT(*) AS cnt FROM ads_hot_video_rank WHERE category = ?")
            .bind(cat)
            .fetch_one(&pool)
            .await?
    } else {
        sqlx::query_as("SELECT COUNT(*) AS cnt FROM ads_hot_video_rank")
            .fetch_one(&pool)
            .await?
    };

    if limit == 0 {
        return Ok(Json(ApiResponse::ok(HotVideosData {
            total: total.0,
            videos: vec![],
        })));
    }

    let where_clause = if category.is_some() { "WHERE category = ?" } else { "" };

    let rows = match sort {
        "heat" => {
            let sql = format!(
                r#"SELECT
                    bvid, title, category, up_name,
                    view_count, like_count, coin_count, favorite_count,
                    reply_count, share_count, rank_position, heat_score
                FROM ads_hot_video_rank
                {}
                ORDER BY heat_score DESC
                LIMIT ? OFFSET ?"#,
                where_clause
            );
            let mut q = sqlx::query_as::<_, HotVideoRow>(&sql);
            if let Some(cat) = category { q = q.bind(cat); }
            q = q.bind(limit).bind(offset);
            q.fetch_all(&pool).await?
        }
        "views" => {
            let sql = format!(
                r#"SELECT
                    bvid, title, category, up_name,
                    view_count, like_count, coin_count, favorite_count,
                    reply_count, share_count, rank_position, heat_score
                FROM ads_hot_video_rank
                {}
                ORDER BY view_count DESC
                LIMIT ? OFFSET ?"#,
                where_clause
            );
            let mut q = sqlx::query_as::<_, HotVideoRow>(&sql);
            if let Some(cat) = category { q = q.bind(cat); }
            q = q.bind(limit).bind(offset);
            q.fetch_all(&pool).await?
        }
        "likes" => {
            let sql = format!(
                r#"SELECT
                    bvid, title, category, up_name,
                    view_count, like_count, coin_count, favorite_count,
                    reply_count, share_count, rank_position, heat_score
                FROM ads_hot_video_rank
                {}
                ORDER BY like_count DESC
                LIMIT ? OFFSET ?"#,
                where_clause
            );
            let mut q = sqlx::query_as::<_, HotVideoRow>(&sql);
            if let Some(cat) = category { q = q.bind(cat); }
            q = q.bind(limit).bind(offset);
            q.fetch_all(&pool).await?
        }
        "coins" => {
            let sql = format!(
                r#"SELECT
                    bvid, title, category, up_name,
                    view_count, like_count, coin_count, favorite_count,
                    reply_count, share_count, rank_position, heat_score
                FROM ads_hot_video_rank
                {}
                ORDER BY coin_count DESC
                LIMIT ? OFFSET ?"#,
                where_clause
            );
            let mut q = sqlx::query_as::<_, HotVideoRow>(&sql);
            if let Some(cat) = category { q = q.bind(cat); }
            q = q.bind(limit).bind(offset);
            q.fetch_all(&pool).await?
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
    let sort = query.sort.as_deref().unwrap_or("heat");
    let category = query.category.as_deref();

    if !(0..=500).contains(&limit) {
        return Err(AppError::bad_request("limit must be between 0 and 500"));
    }

    if limit == 0 {
        return Ok(Json(ApiResponse::ok(LifecycleTopData { videos: vec![] })));
    }

    let order_col = match sort {
        "views" => "h.view_count",
        "likes" => "h.like_count",
        "coins" => "h.coin_count",
        _ => "h.heat_score",
    };

    let top_bvids: Vec<(String,)> = if let Some(cat) = category {
        let sql = format!(
            r#"SELECT h.bvid
               FROM ads_hot_video_rank h
               WHERE h.category = ? AND EXISTS (
                 SELECT 1 FROM ads_video_lifecycle l WHERE l.bvid = h.bvid
               )
               ORDER BY {} DESC
               LIMIT ?"#,
            order_col
        );
        sqlx::query_as::<_, (String,)>(&sql)
            .bind(cat)
            .bind(limit)
            .fetch_all(&pool)
            .await?
    } else {
        let sql = format!(
            r#"SELECT h.bvid
               FROM ads_hot_video_rank h
               WHERE EXISTS (
                 SELECT 1 FROM ads_video_lifecycle l WHERE l.bvid = h.bvid
               )
               ORDER BY {} DESC
               LIMIT ?"#,
            order_col
        );
        sqlx::query_as::<_, (String,)>(&sql)
            .bind(limit)
            .fetch_all(&pool)
            .await?
    };

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

pub async fn word_cloud(
    State(pool): State<MySqlPool>,
    Query(query): Query<WordCloudQuery>,
) -> Result<Json<ApiResponse<WordCloudData>>, AppError> {
    let source = match query.source.as_deref() {
        Some("title") | Some("comment") | Some("danmaku") => query.source.unwrap(),
        Some(other) => {
            return Err(AppError::bad_request(format!(
                "invalid source '{}', allowed: title, comment, danmaku",
                other
            )));
        }
        None => "title".to_string(),
    };

    let category_input = query.category.unwrap_or_else(|| "全站".to_string());
    let db_category = if category_input == "全站" { "__ALL__" } else { &category_input };
    let bvid = query.bvid.unwrap_or_default();

    let rows = if bvid.is_empty() {
        sqlx::query_as!(
            WordCloudRow,
            r#"SELECT
                word AS `name: String`,
                SUM(word_count) AS `value: i32`
            FROM ads_word_cloud
            WHERE source_type = ? AND category = ?
            GROUP BY word
            ORDER BY SUM(word_count) DESC
            LIMIT 100"#,
            source,
            db_category
        )
        .fetch_all(&pool)
        .await?
    } else {
        sqlx::query_as!(
            WordCloudRow,
            r#"SELECT
                word AS `name: String`,
                SUM(word_count) AS `value: i32`
            FROM ads_word_cloud
            WHERE source_type = ? AND bvid = ?
            GROUP BY word
            ORDER BY SUM(word_count) DESC
            LIMIT 100"#,
            source,
            bvid
        )
        .fetch_all(&pool)
        .await?
    };

    let words = rows
        .into_iter()
        .map(|r| WordCloudItem {
            name: r.name.unwrap_or_default(),
            value: r.value.unwrap_or(0),
        })
        .collect();

    Ok(Json(ApiResponse::ok(WordCloudData {
        source,
        category: category_input,
        words,
    })))
}

pub async fn hot_comments(
    State(pool): State<MySqlPool>,
    Query(query): Query<HotCommentsQuery>,
) -> Result<Json<ApiResponse<HotCommentsData>>, AppError> {
    let limit = query.limit.unwrap_or(20);
    if !(0..=500).contains(&limit) {
        return Err(AppError::bad_request("limit must be between 0 and 500"));
    }

    if limit == 0 {
        return Ok(Json(ApiResponse::ok(HotCommentsData { comments: vec![] })));
    }

    let rows = match (&query.bvid, &query.category) {
        (Some(bvid), _) => {
            sqlx::query_as!(
                HotCommentRow,
                r#"SELECT
                    bvid, title, category, up_name, user_name, content,
                    like_count, reply_count,
                    DATE_FORMAT(comment_time, '%Y-%m-%d %H:%i:%s') AS `comment_time: String`
                FROM ads_hot_comments
                WHERE bvid = ?
                ORDER BY like_count DESC
                LIMIT ?"#,
                bvid,
                limit
            )
            .fetch_all(&pool)
            .await?
        }
        (_, Some(category)) => {
            sqlx::query_as!(
                HotCommentRow,
                r#"SELECT
                    bvid, title, category, up_name, user_name, content,
                    like_count, reply_count,
                    DATE_FORMAT(comment_time, '%Y-%m-%d %H:%i:%s') AS `comment_time: String`
                FROM ads_hot_comments
                WHERE category = ?
                ORDER BY like_count DESC
                LIMIT ?"#,
                category,
                limit
            )
            .fetch_all(&pool)
            .await?
        }
        _ => {
            sqlx::query_as!(
                HotCommentRow,
                r#"SELECT
                    bvid, title, category, up_name, user_name, content,
                    like_count, reply_count,
                    DATE_FORMAT(comment_time, '%Y-%m-%d %H:%i:%s') AS `comment_time: String`
                FROM ads_hot_comments
                ORDER BY like_count DESC
                LIMIT ?"#,
                limit
            )
            .fetch_all(&pool)
            .await?
        }
    };

    let comments = rows.into_iter().map(HotCommentItem::from).collect();

    Ok(Json(ApiResponse::ok(HotCommentsData { comments })))
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

pub async fn interaction_max(
    State(pool): State<MySqlPool>,
) -> Result<Json<ApiResponse<InteractionMaxData>>, AppError> {
    let row = sqlx::query_as!(
        InteractionMaxRow,
        r#"SELECT
            CAST(MAX(CASE WHEN view_count > 0 THEN like_count / view_count ELSE 0 END) AS DOUBLE) AS `max_like_rate: f64`,
            CAST(MAX(CASE WHEN view_count > 0 THEN coin_count / view_count ELSE 0 END) AS DOUBLE) AS `max_coin_rate: f64`,
            CAST(MAX(CASE WHEN view_count > 0 THEN favorite_count / view_count ELSE 0 END) AS DOUBLE) AS `max_fav_rate: f64`,
            CAST(MAX(CASE WHEN view_count > 0 THEN reply_count / view_count ELSE 0 END) AS DOUBLE) AS `max_reply_rate: f64`,
            CAST(MAX(CASE WHEN view_count > 0 THEN share_count / view_count ELSE 0 END) AS DOUBLE) AS `max_share_rate: f64`
        FROM ads_hot_video_rank"#
    )
    .fetch_one(&pool)
    .await?;

    Ok(Json(ApiResponse::ok(InteractionMaxData::from(row))))
}
