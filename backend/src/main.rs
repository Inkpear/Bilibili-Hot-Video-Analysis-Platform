use axum::Router;
use tower_http::cors::CorsLayer;

mod db;
mod error;
mod handlers;
mod models;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "bilibili_hot_backend=info,tower_http=info".into()),
        )
        .init();

    let pool = db::create_pool()
        .await
        .expect("failed to create database pool");

    let app = Router::new()
        .route("/api/overview", axum::routing::get(handlers::overview))
        .route("/api/hot-videos", axum::routing::get(handlers::hot_videos))
        .route(
            "/api/partition-compare",
            axum::routing::get(handlers::partition_compare),
        )
        .route("/api/lifecycle", axum::routing::get(handlers::lifecycle))
        .route(
            "/api/lifecycle/top",
            axum::routing::get(handlers::lifecycle_top),
        )
        .route(
            "/api/partition-detail",
            axum::routing::get(handlers::partition_detail),
        )
        .route("/api/tag-cloud", axum::routing::get(handlers::tag_cloud))
        .route(
            "/api/interaction",
            axum::routing::get(handlers::interaction),
        )
        .layer(CorsLayer::permissive())
        .with_state(pool);

    let bind = "0.0.0.0:3000";
    tracing::info!("listening on {}", bind);

    let listener = tokio::net::TcpListener::bind(bind).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
