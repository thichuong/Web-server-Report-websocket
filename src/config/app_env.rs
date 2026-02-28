use std::env;

pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub redis_url: String,
    pub fetch_interval: u64,
    pub taapi_secret: String,
    pub cmc_api_key: Option<String>,
    pub finnhub_api_key: Option<String>,
}

impl AppConfig {
    #[must_use]
    pub fn load() -> Self {
        dotenvy::dotenv().ok();
        Self {
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8081".to_string())
                .parse()
                .unwrap_or(8081),
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string()),
            fetch_interval: env::var("FETCH_INTERVAL_SECONDS")
                .unwrap_or_else(|_| "5".to_string())
                .parse()
                .unwrap_or(5),
            taapi_secret: env::var("TAAPI_SECRET").unwrap_or_else(|_| "default_secret".to_string()),
            cmc_api_key: env::var("CMC_API_KEY").ok(),
            finnhub_api_key: env::var("FINNHUB_API_KEY").ok(),
        }
    }
}
