use axum::Router;
use tower_http::cors::CorsLayer;

mod auth;
mod config;
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

    let cfg = config::AppConfig::load();

    if cfg.auth_enabled() {
        tracing::info!("auth enabled, token required");
    } else {
        tracing::warn!("auth disabled, no API_TOKEN configured");
    }

    let pool = db::create_pool()
        .await
        .expect("failed to create database pool");

    let api_routes = Router::new()
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
        .route("/api/word-cloud", axum::routing::get(handlers::word_cloud))
        .route(
            "/api/hot-comments",
            axum::routing::get(handlers::hot_comments),
        )
        .route(
            "/api/interaction",
            axum::routing::get(handlers::interaction),
        );

    let app = Router::new()
        .merge(api_routes)
        .layer(axum::middleware::from_fn_with_state(
            cfg.clone(),
            auth::auth_middleware,
        ))
        .layer(CorsLayer::permissive())
        .with_state(pool);

    let bind = &cfg.bind;
    tracing::info!("listening on {}", bind);

    let listener = tokio::net::TcpListener::bind(bind).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
