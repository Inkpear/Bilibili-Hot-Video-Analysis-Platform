use axum::extract::{Query, State};
use axum::Json;
use sqlx::MySqlPool;

use crate::error::AppError;
use crate::models::*;

fn snap(date: Option<&str>) -> String {
    match date {
        Some(d) if !d.is_empty() => format!("'{}'", d),
        _ => "(SELECT MAX(snapshot_date) FROM snapshot_dates)".to_string(),
    }
}

pub async fn snapshot_dates(
    State(pool): State<MySqlPool>,
) -> Result<Json<ApiResponse<SnapshotDatesData>>, AppError> {
    let rows = sqlx::query_as!(
        SnapshotDateItem,
        r#"SELECT CAST(snapshot_date AS CHAR) AS `snapshot_date!: String`, CAST(ANY_VALUE(video_count) AS SIGNED) AS `video_count!: i64`
           FROM snapshot_dates GROUP BY snapshot_date ORDER BY snapshot_date DESC"#
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(ApiResponse::ok(SnapshotDatesData { dates: rows })))
}

pub async fn overview(
    State(pool): State<MySqlPool>,
    Query(query): Query<SnapshotQuery>,
) -> Result<Json<ApiResponse<OverviewData>>, AppError> {
    let sn = snap(query.snapshot_date.as_deref());
    let sql = format!(
        r#"SELECT
            CAST(COUNT(DISTINCT bvid) AS SIGNED) AS video_count,
            CAST(COUNT(DISTINCT COALESCE(NULLIF(category, ''), NULLIF(sub_category, ''))) AS SIGNED) AS category_count,
            CAST(COUNT(DISTINCT up_id) AS SIGNED) AS up_count,
            CAST(COALESCE(SUM(view_count), 0) AS SIGNED) AS total_views,
            CAST(COALESCE(SUM(like_count + coin_count + favorite_count + reply_count + share_count), 0) AS SIGNED) AS total_interactions,
            CAST(COALESCE(SUM(like_count), 0) AS SIGNED) AS total_likes,
            CAST(COALESCE(SUM(coin_count), 0) AS SIGNED) AS total_coins,
            CAST(COALESCE(SUM(favorite_count), 0) AS SIGNED) AS total_favorites,
            (SELECT MAX(snapshot_date) FROM snapshot_dates) AS latest_snapshot_date,
            COALESCE((SELECT DATE_FORMAT(FROM_UNIXTIME(MAX(collect_time)), '%Y-%m-%d %H:%i:%s') FROM hot_video WHERE collect_time IS NOT NULL), DATE_FORMAT(NOW(), '%Y-%m-%d %H:%i:%s')) AS update_time
        FROM hot_video
        WHERE snapshot_date = {sn}"#
    );
    let row = sqlx::query_as::<_, OverviewRow>(&sql)
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
    let sn = snap(query.snapshot_date.as_deref());

    if !(0..=500).contains(&limit) {
        return Err(AppError::bad_request("limit must be between 0 and 500"));
    }
    if offset < 0 {
        return Err(AppError::bad_request("offset must be >= 0"));
    }

    let total: (i64,) = if let Some(cat) = category {
        let sql = format!("SELECT CAST(COUNT(DISTINCT bvid) AS SIGNED) AS cnt FROM hot_video WHERE COALESCE(NULLIF(category, ''), NULLIF(sub_category, '')) = ? AND snapshot_date = {}", sn);
        sqlx::query_as(&sql)
            .bind(cat)
            .fetch_one(&pool)
            .await?
    } else {
        let sql = format!("SELECT CAST(COUNT(DISTINCT bvid) AS SIGNED) AS cnt FROM hot_video WHERE snapshot_date = {}", sn);
        sqlx::query_as(&sql)
            .fetch_one(&pool)
            .await?
    };

    if limit == 0 {
        return Ok(Json(ApiResponse::ok(HotVideosData {
            total: total.0,
            videos: vec![],
        })));
    }

    let where_clause = if category.is_some() {
        format!("WHERE COALESCE(NULLIF(h.category, ''), NULLIF(h.sub_category, '')) = ? AND h.snapshot_date = {}", sn)
    } else {
        format!("WHERE h.snapshot_date = {}", sn)
    };

    let heat_score_expr = r#"GREATEST(0.0, LN(GREATEST(h.view_count, 1))) * 3.0 +
        GREATEST(0.0, LN(GREATEST(h.like_count, 1))) * 3.5 +
        GREATEST(0.0, LN(GREATEST(h.coin_count, 1))) * 3.0 +
        GREATEST(0.0, LN(GREATEST(h.favorite_count, 1))) * 4.0 +
        GREATEST(0.0, LN(GREATEST(h.reply_count, 1))) * 1.0 +
        GREATEST(0.0, LN(GREATEST(h.share_count, 1))) * 2.0"#;

    let rows = match sort {
        "heat" => {
            let sql = format!(
                r#"SELECT
                    h.bvid, ANY_VALUE(h.title) AS title, ANY_VALUE(COALESCE(NULLIF(h.category, ''), NULLIF(h.sub_category, ''))) AS category, ANY_VALUE(h.up_name) AS up_name,
                    ANY_VALUE(h.view_count) AS view_count, ANY_VALUE(h.like_count) AS like_count, ANY_VALUE(h.coin_count) AS coin_count, ANY_VALUE(h.favorite_count) AS favorite_count,
                    ANY_VALUE(h.reply_count) AS reply_count, ANY_VALUE(h.share_count) AS share_count, ANY_VALUE(h.rank_position) AS rank_position,
                    MAX({heat}) AS heat_score
                FROM hot_video h
                {where_clause}
                GROUP BY h.bvid
                ORDER BY heat_score DESC
                LIMIT ? OFFSET ?"#,
                heat = heat_score_expr,
                where_clause = where_clause,
            );
            let mut q = sqlx::query_as::<_, HotVideoRow>(&sql);
            if let Some(cat) = category { q = q.bind(cat); }
            q = q.bind(limit).bind(offset);
            q.fetch_all(&pool).await?
        }
        "views" => {
            let sql = format!(
                r#"SELECT
                    h.bvid, ANY_VALUE(h.title) AS title, ANY_VALUE(COALESCE(NULLIF(h.category, ''), NULLIF(h.sub_category, ''))) AS category, ANY_VALUE(h.up_name) AS up_name,
                    ANY_VALUE(h.view_count) AS view_count, ANY_VALUE(h.like_count) AS like_count, ANY_VALUE(h.coin_count) AS coin_count, ANY_VALUE(h.favorite_count) AS favorite_count,
                    ANY_VALUE(h.reply_count) AS reply_count, ANY_VALUE(h.share_count) AS share_count, ANY_VALUE(h.rank_position) AS rank_position,
                    MAX({heat}) AS heat_score
                FROM hot_video h
                {where_clause}
                GROUP BY h.bvid
                ORDER BY view_count DESC
                LIMIT ? OFFSET ?"#,
                heat = heat_score_expr,
                where_clause = where_clause,
            );
            let mut q = sqlx::query_as::<_, HotVideoRow>(&sql);
            if let Some(cat) = category { q = q.bind(cat); }
            q = q.bind(limit).bind(offset);
            q.fetch_all(&pool).await?
        }
        "likes" => {
            let sql = format!(
                r#"SELECT
                    h.bvid, ANY_VALUE(h.title) AS title, ANY_VALUE(COALESCE(NULLIF(h.category, ''), NULLIF(h.sub_category, ''))) AS category, ANY_VALUE(h.up_name) AS up_name,
                    ANY_VALUE(h.view_count) AS view_count, ANY_VALUE(h.like_count) AS like_count, ANY_VALUE(h.coin_count) AS coin_count, ANY_VALUE(h.favorite_count) AS favorite_count,
                    ANY_VALUE(h.reply_count) AS reply_count, ANY_VALUE(h.share_count) AS share_count, ANY_VALUE(h.rank_position) AS rank_position,
                    MAX({heat}) AS heat_score
                FROM hot_video h
                {where_clause}
                GROUP BY h.bvid
                ORDER BY like_count DESC
                LIMIT ? OFFSET ?"#,
                heat = heat_score_expr,
                where_clause = where_clause,
            );
            let mut q = sqlx::query_as::<_, HotVideoRow>(&sql);
            if let Some(cat) = category { q = q.bind(cat); }
            q = q.bind(limit).bind(offset);
            q.fetch_all(&pool).await?
        }
        "coins" => {
            let sql = format!(
                r#"SELECT
                    h.bvid, ANY_VALUE(h.title) AS title, ANY_VALUE(COALESCE(NULLIF(h.category, ''), NULLIF(h.sub_category, ''))) AS category, ANY_VALUE(h.up_name) AS up_name,
                    ANY_VALUE(h.view_count) AS view_count, ANY_VALUE(h.like_count) AS like_count, ANY_VALUE(h.coin_count) AS coin_count, ANY_VALUE(h.favorite_count) AS favorite_count,
                    ANY_VALUE(h.reply_count) AS reply_count, ANY_VALUE(h.share_count) AS share_count, ANY_VALUE(h.rank_position) AS rank_position,
                    MAX({heat}) AS heat_score
                FROM hot_video h
                {where_clause}
                GROUP BY h.bvid
                ORDER BY coin_count DESC
                LIMIT ? OFFSET ?"#,
                heat = heat_score_expr,
                where_clause = where_clause,
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
    Query(query): Query<SnapshotQuery>,
) -> Result<Json<ApiResponse<PartitionCompareData>>, AppError> {
    let sn = snap(query.snapshot_date.as_deref());
    let sql = format!(
        r#"SELECT
            COALESCE(NULLIF(category, ''), NULLIF(sub_category, '')) AS category,
            CAST(COUNT(DISTINCT bvid) AS SIGNED) AS hot_video_count,
            CAST(COALESCE(AVG(view_count), 0) AS SIGNED) AS avg_views,
            CAST(COALESCE(AVG(like_count / NULLIF(view_count, 0)), 0) AS DOUBLE) AS avg_like_rate,
            CAST(COALESCE(AVG(coin_count / NULLIF(view_count, 0)), 0) AS DOUBLE) AS avg_coin_rate,
            CAST(COALESCE(AVG(favorite_count / NULLIF(view_count, 0)), 0) AS DOUBLE) AS avg_fav_rate,
            COALESCE(MAX(view_count), 0) AS top_view_count,
            CAST(COALESCE(SUM(view_count), 0) AS SIGNED) AS total_views,
            CAST(COUNT(DISTINCT up_id) AS SIGNED) AS up_count,
            CAST(COALESCE(SUM(like_count), 0) AS SIGNED) AS total_likes,
            CAST(COALESCE(SUM(coin_count), 0) AS SIGNED) AS total_coins,
            CAST(COALESCE(SUM(favorite_count), 0) AS SIGNED) AS total_favorites
        FROM hot_video
        WHERE snapshot_date = {}
        GROUP BY COALESCE(NULLIF(category, ''), NULLIF(sub_category, ''))
        ORDER BY total_views DESC"#,
        sn
    );
    let rows = sqlx::query_as::<_, PartitionCompareRow>(&sql)
        .fetch_all(&pool)
        .await?;

    let partitions = rows.into_iter().map(PartitionCompareItem::from).collect();

    Ok(Json(ApiResponse::ok(PartitionCompareData { partitions })))
}

async fn interpolate_lifecycle(pool: &MySqlPool, rows: &[LifecycleRow]) -> Vec<LifecyclePoint> {
    if rows.is_empty() { return vec![]; }

    let all_dates: Vec<String> = sqlx::query_as::<_, (String,)>(
        "SELECT snapshot_date FROM snapshot_dates GROUP BY snapshot_date ORDER BY snapshot_date ASC"
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|(d,)| d)
    .collect();

    let mut timeline = Vec::new();
    let min_date = rows.first().unwrap().snapshot_date.clone().unwrap_or_default();
    let max_date = rows.last().unwrap().snapshot_date.clone().unwrap_or_default();

    use std::collections::BTreeMap;
    let mut data_map: BTreeMap<&str, &LifecycleRow> = BTreeMap::new();
    for r in rows {
        if let Some(ref d) = r.snapshot_date { data_map.insert(d.as_str(), r); }
    }

    for date in &all_dates {
        if *date < min_date || *date > max_date { continue; }
        if let Some(row) = data_map.get(date.as_str()) {
            timeline.push(row.to_point());
        } else {
            let prev = rows.iter().filter(|r| r.snapshot_date.as_deref().unwrap_or("") < date.as_str()).last();
            let next = rows.iter().filter(|r| r.snapshot_date.as_deref().unwrap_or("") > date.as_str()).next();
            if let (Some(prev), Some(next)) = (prev, next) {
                let ratio = {
                    let p = prev.snapshot_date.as_deref().unwrap_or("").parse::<i64>().unwrap_or(0);
                    let n = next.snapshot_date.as_deref().unwrap_or("").parse::<i64>().unwrap_or(0);
                    let c = date.parse::<i64>().unwrap_or(0);
                    if n > p { (c - p) as f64 / (n - p) as f64 } else { 0.5 }
                };
                let interp = |a: i64, b: i64| (a as f64 + (b - a) as f64 * ratio) as i64;
                timeline.push(LifecyclePoint {
                    snapshot_date: date.clone(), snapshot_time: String::new(),
                    view_count: interp(prev.view_count.unwrap_or(0), next.view_count.unwrap_or(0)),
                    like_count: interp(prev.like_count.unwrap_or(0), next.like_count.unwrap_or(0)),
                    rank_position: prev.rank_position.unwrap_or(0),
                    rank_source: prev.rank_source.clone().unwrap_or_default(),
                    rank_name: prev.rank_name.clone().unwrap_or_default(),
                    view_growth: 0, rank_change: 0,
                });
            }
        }
    }

    for i in 1..timeline.len() {
        timeline[i].view_growth = timeline[i].view_count - timeline[i-1].view_count;
        timeline[i].rank_change = timeline[i-1].rank_position - timeline[i].rank_position;
    }
    timeline
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
            hh.bvid,
            hh.title,
            hh.category,
            hh.snapshot_date,
            COALESCE(DATE_FORMAT(FROM_UNIXTIME(hh.collect_time), '%H:%i'), '') AS snapshot_time,
            hh.view_count,
            hh.like_count,
            hh.rank_position,
            hh.rank_source,
            hh.rank_name,
            COALESCE(hh.view_count - LAG(hh.view_count) OVER (ORDER BY hh.snapshot_date), 0) AS view_growth,
            COALESCE(LAG(hh.rank_position) OVER (ORDER BY hh.snapshot_date) - hh.rank_position, 0) AS rank_change
        FROM (
            SELECT
                h.bvid, ANY_VALUE(h.title) AS title,
                ANY_VALUE(COALESCE(NULLIF(h.category, ''), NULLIF(h.sub_category, ''))) AS category,
                h.snapshot_date,
                ANY_VALUE(h.collect_time) AS collect_time,
                ANY_VALUE(h.view_count) AS view_count, ANY_VALUE(h.like_count) AS like_count,
                ANY_VALUE(h.rank_position) AS rank_position, ANY_VALUE(h.rank_source) AS rank_source,
                ANY_VALUE(h.rank_name) AS rank_name
            FROM hot_video h
            WHERE h.bvid = ?
            GROUP BY h.bvid, h.snapshot_date
        ) hh
        ORDER BY hh.snapshot_date ASC"#,
        &bvid
    )
    .fetch_all(&pool)
    .await?;

    if rows.is_empty() {
        return Ok(Json(ApiResponse::ok(None)));
    }

    let first = &rows[0];
    let mut timeline = interpolate_lifecycle(&pool, &rows).await;
    if let Some(ref snap) = query.snapshot_date.as_ref().filter(|s| !s.is_empty()) {
        timeline.retain(|p| p.snapshot_date.as_str() <= snap.as_str());
    }

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

    let heat_expr = r#"GREATEST(0.0, LN(GREATEST(h.view_count, 1))) * 3.0 +
        GREATEST(0.0, LN(GREATEST(h.like_count, 1))) * 3.5 +
        GREATEST(0.0, LN(GREATEST(h.coin_count, 1))) * 3.0 +
        GREATEST(0.0, LN(GREATEST(h.favorite_count, 1))) * 4.0 +
        GREATEST(0.0, LN(GREATEST(h.reply_count, 1))) * 1.0 +
        GREATEST(0.0, LN(GREATEST(h.share_count, 1))) * 2.0"#;

    let order_col = match sort {
        "views" => "MAX(h.view_count)".to_string(),
        "likes" => "MAX(h.like_count)".to_string(),
        "coins" => "MAX(h.coin_count)".to_string(),
        _ => format!("MAX({})", heat_expr),
    };

    let top_bvids: Vec<(String,)> = if let Some(cat) = category {
        let sql = format!(
            r#"SELECT h.bvid
               FROM hot_video h
               WHERE COALESCE(NULLIF(h.category, ''), NULLIF(h.sub_category, '')) = ? AND h.snapshot_date = {} AND EXISTS (
                 SELECT 1 FROM hot_video l WHERE l.bvid = h.bvid
               )
               GROUP BY h.bvid
               ORDER BY ({}) DESC
               LIMIT ?"#,
            snap(query.snapshot_date.as_deref()),
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
               FROM hot_video h
               WHERE h.snapshot_date = {} AND EXISTS (
                 SELECT 1 FROM hot_video l WHERE l.bvid = h.bvid
               )
               GROUP BY h.bvid
               ORDER BY ({}) DESC
               LIMIT ?"#,
            snap(query.snapshot_date.as_deref()),
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
                hh.bvid,
                hh.title,
                hh.category,
                hh.snapshot_date,
                COALESCE(DATE_FORMAT(FROM_UNIXTIME(hh.collect_time), '%H:%i'), '') AS snapshot_time,
                hh.view_count,
                hh.like_count,
                hh.rank_position,
                hh.rank_source,
                hh.rank_name,
                COALESCE(hh.view_count - LAG(hh.view_count) OVER (ORDER BY hh.snapshot_date), 0) AS view_growth,
                COALESCE(LAG(hh.rank_position) OVER (ORDER BY hh.snapshot_date) - hh.rank_position, 0) AS rank_change
            FROM (
                SELECT
                    h.bvid, ANY_VALUE(h.title) AS title,
                    ANY_VALUE(COALESCE(NULLIF(h.category, ''), NULLIF(h.sub_category, ''))) AS category,
                    h.snapshot_date,
                    ANY_VALUE(h.collect_time) AS collect_time,
                    ANY_VALUE(h.view_count) AS view_count, ANY_VALUE(h.like_count) AS like_count,
                    ANY_VALUE(h.rank_position) AS rank_position, ANY_VALUE(h.rank_source) AS rank_source,
                    ANY_VALUE(h.rank_name) AS rank_name
                FROM hot_video h
                WHERE h.bvid = ?
                GROUP BY h.bvid, h.snapshot_date
            ) hh
            ORDER BY hh.snapshot_date ASC"#,
            bvid
        )
        .fetch_all(&pool)
        .await?;

        if let Some(first) = rows.first() {
            let mut timeline = interpolate_lifecycle(&pool, &rows).await;
            if let Some(ref snap) = query.snapshot_date.as_ref().filter(|s| !s.is_empty()) {
                timeline.retain(|p| p.snapshot_date.as_str() <= snap.as_str());
            }

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

    let sn = snap(query.snapshot_date.as_deref());

    let overview_sql = format!(
        r#"SELECT
            CAST(COUNT(DISTINCT bvid) AS SIGNED) AS hot_video_count,
            CAST(COALESCE(AVG(view_count), 0) AS SIGNED) AS avg_views
           FROM hot_video
           WHERE COALESCE(NULLIF(category, ''), NULLIF(sub_category, '')) = ? AND snapshot_date = {}
           LIMIT 1"#,
        sn
    );
    let overview = sqlx::query_as::<_, PartitionOverviewRow>(&overview_sql)
        .bind(&category)
        .fetch_optional(&pool)
        .await?;

    let (hot_video_count, avg_views) = match overview {
        Some(o) => (o.hot_video_count.unwrap_or(0) as i32, o.avg_views.unwrap_or(0)),
        None => (0, 0i64),
    };

    let video_sql = format!(
        r#"SELECT
            h.bvid, ANY_VALUE(h.title) AS title, ANY_VALUE(h.view_count) AS view_count,
            CAST(MAX((GREATEST(0.0, LN(GREATEST(h.view_count, 1))) * 3.0 +
             GREATEST(0.0, LN(GREATEST(h.like_count, 1))) * 3.5 +
             GREATEST(0.0, LN(GREATEST(h.coin_count, 1))) * 3.0 +
             GREATEST(0.0, LN(GREATEST(h.favorite_count, 1))) * 4.0 +
             GREATEST(0.0, LN(GREATEST(h.reply_count, 1))) * 1.0 +
              GREATEST(0.0, LN(GREATEST(h.share_count, 1))) * 2.0)) AS DOUBLE) AS heat_score,
            ANY_VALUE(h.rank_position) AS rank_position
        FROM hot_video h
        WHERE COALESCE(NULLIF(h.category, ''), NULLIF(h.sub_category, '')) = ? AND h.snapshot_date = {}
        GROUP BY h.bvid
        ORDER BY view_count DESC
        LIMIT 10"#,
        sn
    );
    let video_rows = sqlx::query_as::<_, PartitionDetailVideoRow>(&video_sql)
        .bind(&category)
        .fetch_all(&pool)
        .await?;

    let videos = video_rows
        .into_iter()
        .map(PartitionDetailVideo::from)
        .collect();

    let up_sql = format!(
        r#"SELECT DISTINCT
            h.up_id,
            COALESCE(u.up_name, h.up_name) AS up_name,
            COALESCE(u.follower_count, 0) AS follower_count,
            CAST(COUNT(DISTINCT h.bvid) AS SIGNED) AS hot_video_count,
            CAST(SUM(h.view_count) AS SIGNED) AS total_views
        FROM hot_video h
        LEFT JOIN (
            SELECT ui.up_id, ANY_VALUE(ui.up_name) AS up_name,
                   ANY_VALUE(ui.follower_count) AS follower_count, ui.snapshot_date
            FROM up_info ui
            GROUP BY ui.up_id, ui.snapshot_date
        ) u ON h.up_id = u.up_id AND h.snapshot_date = u.snapshot_date
        WHERE COALESCE(NULLIF(h.category, ''), NULLIF(h.sub_category, '')) = ? AND h.snapshot_date = {}
        GROUP BY h.up_id, u.up_name, u.follower_count, h.up_name
        ORDER BY total_views DESC
        LIMIT 10"#,
        sn
    );
    let up_rows = sqlx::query_as::<_, PartitionDetailUpRow>(&up_sql)
        .bind(&category)
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
    let bvid = query.bvid.unwrap_or_default();

    let table_name = match source.as_str() {
        "title" => "word_cloud_title",
        "comment" => "word_cloud_comment",
        "danmaku" => "word_cloud_danmaku",
        _ => "word_cloud_title",
    };

    let rows = if bvid.is_empty() {
        if category_input == "全站" {
            let sql = format!(
                r#"SELECT
                    word AS name,
                    CAST(SUM(word_count) AS SIGNED) AS value
                FROM {}
                WHERE snapshot_date = {}
                GROUP BY word
                ORDER BY SUM(word_count) DESC
                LIMIT 100"#,
                table_name,
                snap(query.snapshot_date.as_deref())
            );
            sqlx::query_as::<_, WordCloudRow>(&sql)
                .fetch_all(&pool)
                .await?
        } else {
            let sql = format!(
                r#"SELECT
                    word AS name,
                    CAST(SUM(word_count) AS SIGNED) AS value
                FROM {} wc
                JOIN hot_video hv ON wc.bvid = hv.bvid AND wc.snapshot_date = hv.snapshot_date
                WHERE hv.category = ? AND wc.snapshot_date = {}
                GROUP BY word
                ORDER BY SUM(word_count) DESC
                LIMIT 100"#,
                table_name,
                snap(query.snapshot_date.as_deref())
            );
            sqlx::query_as::<_, WordCloudRow>(&sql)
                .bind(&category_input)
                .fetch_all(&pool)
                .await?
        }
    } else {
        let sql = format!(
            r#"SELECT
                word AS name,
                CAST(SUM(word_count) AS SIGNED) AS value
            FROM {}
            WHERE bvid = ? AND snapshot_date = {}
            GROUP BY word
            ORDER BY SUM(word_count) DESC
            LIMIT 100"#,
            table_name,
            snap(query.snapshot_date.as_deref())
        );
        sqlx::query_as::<_, WordCloudRow>(&sql)
            .bind(&bvid)
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

    let sort_col = match query.sort.as_deref() {
        Some("replies") => "reply_count",
        _ => "like_count",
    };

    let sn = snap(query.snapshot_date.as_deref());

    let rows = match (&query.bvid, &query.category) {
        (Some(bvid), _) => {
            let sql = format!(
                r#"SELECT
                    c.bvid, h.title, h.category, h.up_name, c.user_name, c.content,
                    CAST(c.like_count AS SIGNED) AS like_count,
                    CAST(c.reply_count AS SIGNED) AS reply_count,
                    DATE_FORMAT(FROM_UNIXTIME(c.comment_time), '%Y-%m-%d %H:%i:%s') AS comment_time
                FROM (
                    SELECT comment_id, bvid, user_name, content, like_count, reply_count, comment_time, snapshot_date
                    FROM comment_info
                    WHERE bvid = ? AND snapshot_date = {}
                    ORDER BY {} DESC
                    LIMIT ?
                ) c
                JOIN (
                    SELECT hh.bvid, ANY_VALUE(hh.title) AS title,
                        ANY_VALUE(COALESCE(NULLIF(hh.category, ''), NULLIF(hh.sub_category, ''))) AS category,
                        ANY_VALUE(hh.up_name) AS up_name,
                        hh.snapshot_date
                    FROM hot_video hh
                    WHERE hh.snapshot_date = {}
                    GROUP BY hh.bvid, hh.snapshot_date
                ) h ON c.bvid = h.bvid AND c.snapshot_date = h.snapshot_date"#,
                sn, sort_col, sn
            );
            sqlx::query_as::<_, HotCommentRow>(&sql)
                .bind(bvid)
                .bind(limit)
                .fetch_all(&pool)
                .await?
        }
        (_, Some(category)) => {
            let sql = format!(
                r#"SELECT
                    c.bvid, h.title, h.category, h.up_name, c.user_name, c.content,
                    CAST(c.like_count AS SIGNED) AS like_count,
                    CAST(c.reply_count AS SIGNED) AS reply_count,
                    DATE_FORMAT(FROM_UNIXTIME(c.comment_time), '%Y-%m-%d %H:%i:%s') AS comment_time
                FROM comment_info c
                JOIN (
                    SELECT hh.bvid, ANY_VALUE(hh.title) AS title,
                        ANY_VALUE(COALESCE(NULLIF(hh.category, ''), NULLIF(hh.sub_category, ''))) AS category,
                        ANY_VALUE(hh.up_name) AS up_name,
                        hh.snapshot_date
                    FROM hot_video hh
                    WHERE hh.snapshot_date = {}
                    GROUP BY hh.bvid, hh.snapshot_date
                ) h ON c.bvid = h.bvid AND c.snapshot_date = h.snapshot_date
                WHERE h.category = ? AND c.snapshot_date = {}
                ORDER BY {} DESC
                LIMIT ?"#,
                sn, sn, sort_col
            );
            sqlx::query_as::<_, HotCommentRow>(&sql)
                .bind(category)
                .bind(limit)
                .fetch_all(&pool)
                .await?
        }
        _ => {
            let sql = format!(
                r#"SELECT
                    c.bvid, h.title, h.category, h.up_name, c.user_name, c.content,
                    CAST(c.like_count AS SIGNED) AS like_count,
                    CAST(c.reply_count AS SIGNED) AS reply_count,
                    DATE_FORMAT(FROM_UNIXTIME(c.comment_time), '%Y-%m-%d %H:%i:%s') AS comment_time
                FROM comment_info c
                JOIN (
                    SELECT hh.bvid, ANY_VALUE(hh.title) AS title,
                        ANY_VALUE(COALESCE(NULLIF(hh.category, ''), NULLIF(hh.sub_category, ''))) AS category,
                        ANY_VALUE(hh.up_name) AS up_name,
                        hh.snapshot_date
                    FROM hot_video hh
                    WHERE hh.snapshot_date = {}
                    GROUP BY hh.bvid, hh.snapshot_date
                ) h ON c.bvid = h.bvid AND c.snapshot_date = h.snapshot_date
                WHERE c.snapshot_date = {}
                ORDER BY {} DESC
                LIMIT ?"#,
                sn, sn, sort_col
            );
            sqlx::query_as::<_, HotCommentRow>(&sql)
                .bind(limit)
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

    let sn = snap(query.snapshot_date.as_deref());
    let sql = format!(
        r#"SELECT
            bvid,
            view_count,
            like_count,
            coin_count,
            favorite_count,
            reply_count,
            share_count
        FROM hot_video
        WHERE bvid = ? AND snapshot_date = {}
        LIMIT 1"#,
        sn
    );
    let row = sqlx::query_as::<_, InteractionRow>(&sql)
        .bind(bvid)
        .fetch_optional(&pool)
        .await?;

    match row {
        Some(r) => Ok(Json(ApiResponse::ok(Some(InteractionData::from(r))))),
        None => Ok(Json(ApiResponse::ok(None))),
    }
}

pub async fn interaction_max(
    State(pool): State<MySqlPool>,
    Query(query): Query<SnapshotQuery>,
) -> Result<Json<ApiResponse<InteractionMaxData>>, AppError> {
    let sn = snap(query.snapshot_date.as_deref());
    let sql = format!(
        r#"SELECT
            CAST(MAX(CASE WHEN view_count > 0 THEN like_count / view_count ELSE 0 END) AS DOUBLE) AS max_like_rate,
            CAST(MAX(CASE WHEN view_count > 0 THEN coin_count / view_count ELSE 0 END) AS DOUBLE) AS max_coin_rate,
            CAST(MAX(CASE WHEN view_count > 0 THEN favorite_count / view_count ELSE 0 END) AS DOUBLE) AS max_fav_rate,
            CAST(MAX(CASE WHEN view_count > 0 THEN reply_count / view_count ELSE 0 END) AS DOUBLE) AS max_reply_rate,
            CAST(MAX(CASE WHEN view_count > 0 THEN share_count / view_count ELSE 0 END) AS DOUBLE) AS max_share_rate
        FROM hot_video
        WHERE snapshot_date = {}"#,
        sn
    );
    let row = sqlx::query_as::<_, InteractionMaxRow>(&sql)
        .fetch_one(&pool)
        .await?;

    Ok(Json(ApiResponse::ok(InteractionMaxData::from(row))))
}

pub async fn word_trend(
    State(pool): State<MySqlPool>,
    Query(query): Query<WordTrendQuery>,
) -> Result<Json<ApiResponse<WordTrendData>>, AppError> {
    let category = query.category.filter(|c| !c.is_empty());
    let source_type = query.source_type.unwrap_or_else(|| "title".to_string());
    let bvid = query.bvid.filter(|b| !b.is_empty());

    // 视频颗粒度: 查词云表
    if let Some(bvid) = bvid {
        let table = match source_type.as_str() {
            "comment" => "word_cloud_comment",
            "danmaku" => "word_cloud_danmaku",
            _ => "word_cloud_title",
        };
        let sql = format!(
            r#"SELECT
                word, snapshot_date,
                CAST(SUM(word_count) AS SIGNED) AS total_count,
                '' AS category, '{}' AS source_type
            FROM {}
            WHERE bvid = ?
            GROUP BY word, snapshot_date
            ORDER BY SUM(word_count) DESC
            LIMIT 20"#,
            source_type, table
        );
        let rows = sqlx::query_as::<_, WordTrendRow>(&sql)
            .bind(&bvid)
            .fetch_all(&pool)
            .await?;
        let mut items: Vec<WordTrendItem> = rows.into_iter().map(WordTrendItem::from).collect();
        if let Some(ref snap) = query.snapshot_date.filter(|s| !s.is_empty()) {
            items.retain(|i| i.snapshot_date.as_str() <= snap.as_str());
        }
        return Ok(Json(ApiResponse::ok(WordTrendData { items })));
    }

    // 分区颗粒度: JOIN hot_video 用 category 过滤
    if let Some(cat) = &category {
        let table = match source_type.as_str() {
            "comment" => "word_cloud_comment",
            "danmaku" => "word_cloud_danmaku",
            _ => "word_cloud_title",
        };
        let sql = format!(
            r#"SELECT
                wc.word, wc.snapshot_date,
                CAST(SUM(wc.word_count) AS SIGNED) AS total_count,
                '' AS category, '{}' AS source_type
            FROM {} wc
            JOIN hot_video hv ON wc.bvid = hv.bvid AND wc.snapshot_date = hv.snapshot_date
            WHERE hv.category = ?
            GROUP BY wc.word, wc.snapshot_date
            ORDER BY SUM(wc.word_count) DESC
            LIMIT 20"#,
            source_type, table
        );
        let rows = sqlx::query_as::<_, WordTrendRow>(&sql)
            .bind(cat)
            .fetch_all(&pool)
            .await?;
        let mut items: Vec<WordTrendItem> = rows.into_iter().map(WordTrendItem::from).collect();
        if let Some(ref snap) = query.snapshot_date.filter(|s| !s.is_empty()) {
            items.retain(|i| i.snapshot_date.as_str() <= snap.as_str());
        }
        return Ok(Json(ApiResponse::ok(WordTrendData { items })));
    }

    // 全站颗粒度: 查 word_trend 表

    let rows = if let Some(word) = query.word.filter(|w| !w.is_empty()) {
        if let Some(cat) = &category {
            let sql = r#"SELECT
                word, snapshot_date,
                CAST(total_count AS SIGNED) AS total_count,
                category, source_type
                FROM word_trend
                WHERE word = ? AND category = ? AND source_type = ?
            ORDER BY snapshot_date ASC"#;
            sqlx::query_as::<_, WordTrendRow>(sql)
                .bind(&word)
                .bind(cat)
                .bind(&source_type)
                .fetch_all(&pool)
                .await?
        } else {
            let sql = r#"SELECT
                word, snapshot_date,
                CAST(total_count AS SIGNED) AS total_count,
                category, source_type
                FROM word_trend
                WHERE word = ? AND source_type = ?
            ORDER BY snapshot_date ASC"#;
            sqlx::query_as::<_, WordTrendRow>(sql)
                .bind(&word)
                .bind(&source_type)
                .fetch_all(&pool)
                .await?
        }
    } else {
        if let Some(cat) = &category {
            let sql = r#"SELECT
                wt.word, wt.snapshot_date,
                CAST(wt.total_count AS SIGNED) AS total_count,
                wt.category, wt.source_type
            FROM word_trend wt
            INNER JOIN (
                SELECT word FROM word_trend
                WHERE category = ? AND source_type = ?
                GROUP BY word ORDER BY SUM(total_count) DESC LIMIT 10
            ) top_words ON wt.word = top_words.word
            WHERE wt.category = ? AND wt.source_type = ?
            ORDER BY wt.word, wt.snapshot_date ASC"#;
            sqlx::query_as::<_, WordTrendRow>(sql)
                .bind(cat)
                .bind(&source_type)
                .bind(cat)
                .bind(&source_type)
                .fetch_all(&pool)
                .await?
        } else {
            let sql = r#"SELECT
                wt.word, wt.snapshot_date,
                CAST(wt.total_count AS SIGNED) AS total_count,
                wt.category, wt.source_type
            FROM word_trend wt
            INNER JOIN (
                SELECT word FROM word_trend
                WHERE source_type = ?
                GROUP BY word ORDER BY SUM(total_count) DESC LIMIT 10
            ) top_words ON wt.word = top_words.word
            WHERE wt.source_type = ?
            ORDER BY wt.word, wt.snapshot_date ASC"#;
            sqlx::query_as::<_, WordTrendRow>(sql)
                .bind(&source_type)
                .bind(&source_type)
                .fetch_all(&pool)
                .await?
        }
    };

    let mut items: Vec<WordTrendItem> = rows.into_iter().map(WordTrendItem::from).collect();

    // Interpolation: fill missing dates for each word
    if !items.is_empty() {
        let all_dates: Vec<String> = sqlx::query_as::<_, (String,)>(
            "SELECT snapshot_date FROM snapshot_dates GROUP BY snapshot_date ORDER BY snapshot_date ASC"
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|(d,)| d)
        .collect();

        if all_dates.len() > 1 {
            use std::collections::BTreeMap;
            // Group by word
            let mut by_word: BTreeMap<String, Vec<WordTrendItem>> = BTreeMap::new();
            for item in items {
                by_word.entry(item.word.clone()).or_default().push(item);
            }

            let mut interpolated = Vec::new();
            for (_, mut points) in by_word {
                points.sort_by(|a, b| a.snapshot_date.cmp(&b.snapshot_date));
                let min_d = points.first().unwrap().snapshot_date.clone();
                let max_d = points.last().unwrap().snapshot_date.clone();
                // Build date→value map
                let date_val: BTreeMap<&str, i64> = points.iter()
                    .map(|p| (p.snapshot_date.as_str(), p.total_count))
                    .collect();

                // Get all existing values in order for outlier detection
                let ordered_vals: Vec<(String, i64)> = points.iter()
                    .map(|p| (p.snapshot_date.clone(), p.total_count))
                    .collect();

                for date in &all_dates {
                    if *date < min_d || *date > max_d { continue; }
                    if let Some(&val) = date_val.get(date.as_str()) {
                        // Outlier detection: if value drops >60% vs neighbors, interpolate
                        let mut use_val = val;
                        let idx = ordered_vals.iter().position(|(d, _)| d == date);
                        if let Some(i) = idx {
                            if i > 0 && i < ordered_vals.len() - 1 {
                                let prev_v = ordered_vals[i-1].1 as f64;
                                let next_v = ordered_vals[i+1].1 as f64;
                                let avg = (prev_v + next_v) / 2.0;
                                if avg > 0.0 && (val as f64) < avg * 0.4 {
                                    // Outlier: interpolate
                                    let ratio = {
                                        let p = ordered_vals[i-1].0.parse::<i64>().unwrap_or(0);
                                        let n = ordered_vals[i+1].0.parse::<i64>().unwrap_or(0);
                                        let c = date.parse::<i64>().unwrap_or(0);
                                        if n > p { (c - p) as f64 / (n - p) as f64 } else { 0.5 }
                                    };
                                    use_val = (prev_v + (next_v - prev_v) * ratio) as i64;
                                }
                            }
                        }
                        interpolated.push(WordTrendItem {
                            word: points[0].word.clone(),
                            snapshot_date: date.clone(),
                            total_count: use_val.max(0),
                            category: points[0].category.clone(),
                            source_type: points[0].source_type.clone(),
                        });
                    } else {
                        let prev = points.iter().filter(|p| p.snapshot_date < *date).last();
                        let next = points.iter().filter(|p| p.snapshot_date > *date).next();
                        if let (Some(prev), Some(next)) = (prev, next) {
                            let ratio = {
                                let p = prev.snapshot_date.parse::<i64>().unwrap_or(0);
                                let n = next.snapshot_date.parse::<i64>().unwrap_or(0);
                                let c = date.parse::<i64>().unwrap_or(0);
                                if n > p { (c - p) as f64 / (n - p) as f64 } else { 0.5 }
                            };
                            let val = (prev.total_count as f64 + (next.total_count - prev.total_count) as f64 * ratio) as i64;
                            interpolated.push(WordTrendItem {
                                word: points[0].word.clone(),
                                snapshot_date: date.clone(),
                                total_count: val.max(0),
                                category: points[0].category.clone(),
                                source_type: points[0].source_type.clone(),
                            });
                        }
                    }
                }
            }
            items = interpolated;
        }
    }

    if let Some(ref snap) = query.snapshot_date.filter(|s| !s.is_empty()) {
        items.retain(|i| i.snapshot_date <= *snap);
    }

    Ok(Json(ApiResponse::ok(WordTrendData { items })))
}
