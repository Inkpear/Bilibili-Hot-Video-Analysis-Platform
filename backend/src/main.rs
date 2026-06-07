// backend/src/main.rs
use axum::{Router, routing::get};
use tower_http::cors::CorsLayer;
use sqlx::MySqlPool;
use std::sync::Arc;

mod handlers;
mod models;

pub struct AppState {
    db: MySqlPool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::init();
    
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "mysql://root:password@localhost/bilibili".to_string());
    
    let pool = MySqlPool::connect(&database_url).await?;
    
    let state = Arc::new(AppState { db: pool });
    
    let app = Router::new()
        .route("/api/overview", get(handlers::overview))
        .route("/api/partitions", get(handlers::partitions))
        .route("/api/partition-heatmap", get(handlers::partition_heatmap))
        .route("/api/trend", get(handlers::trend))
        .route("/api/word-cloud", get(handlers::word_cloud))
        .route("/api/videos", get(handlers::videos))
        .route("/api/video/{bvid}", get(handlers::video_detail))
        .route("/api/word-trend", get(handlers::word_trend))
        .route("/api/snapshot-dates", get(handlers::snapshot_dates))
        .layer(CorsLayer::permissive())
        .with_state(state);
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:9090").await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}
