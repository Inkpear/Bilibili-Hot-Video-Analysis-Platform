use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub api_token: Option<String>,
    pub bind: String,
}

impl AppConfig {
    pub fn load() -> Self {
        let cfg = config::Config::builder()
            .add_source(config::Environment::default().separator("__"))
            .add_source(config::File::with_name("backend/config").required(false))
            .set_default(
                "database_url",
                "mysql://hive:Hive@123456@127.0.0.1:3306/bilibili",
            )
            .unwrap()
            .set_default("api_token", "")
            .unwrap()
            .set_default("bind", "0.0.0.0:3000")
            .unwrap()
            .build()
            .expect("failed to build config");

        cfg.try_deserialize().expect("failed to deserialize config")
    }

    pub fn auth_enabled(&self) -> bool {
        self.api_token
            .as_ref()
            .map(|t| !t.is_empty())
            .unwrap_or(false)
    }
}
