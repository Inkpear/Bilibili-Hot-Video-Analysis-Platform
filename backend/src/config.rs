#[derive(Debug, Clone)]
pub struct AppConfig {
    pub api_token: Option<String>,
    pub bind: String,
}

impl AppConfig {
    pub fn load() -> Self {
        Self {
            api_token: std::env::var("API_TOKEN").ok().filter(|t| !t.is_empty()),
            bind: std::env::var("BIND").unwrap_or_else(|_| "0.0.0.0:9090".to_string()),
        }
    }

    pub fn auth_enabled(&self) -> bool {
        self.api_token
            .as_ref()
            .map(|t| !t.is_empty())
            .unwrap_or(false)
    }
}
