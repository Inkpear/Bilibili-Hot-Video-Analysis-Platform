use sqlx::mysql::MySqlPoolOptions;
use sqlx::MySqlPool;

const DEFAULT_DATABASE_URL: &str = "mysql://hive:Hive@123456@127.0.0.1:3306/bilibili";

pub async fn create_pool() -> Result<MySqlPool, sqlx::Error> {
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string());

    tracing::info!("connecting to database: {}", mask_password(&database_url));

    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .min_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(&database_url)
        .await?;

    tracing::info!("database pool created");
    Ok(pool)
}

fn mask_password(url: &str) -> String {
    if let Some(at) = url.find('@') {
        if let Some(colon) = url[..at].rfind(':') {
            let mut masked = url[..colon + 1].to_string();
            masked.push_str("***@");
            masked.push_str(&url[at + 1..]);
            return masked;
        }
    }
    url.to_string()
}
