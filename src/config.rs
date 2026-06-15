use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub port: u16,
    pub admin_api_key: String,
    pub google_api_key: String,
    pub ai_model: String,
    pub tavily_api_key: String,
    pub bridge_secret: String,
    pub whatsapp_bridge_url: String,
    pub briefing_recipient: Option<String>,
    pub enable_daily_briefing: bool,
    pub briefing_hour_utc: u32,
    pub briefing_interval_seconds: Option<u64>,
    pub briefing_name: String,
    pub interests: Vec<String>,
    pub blocked_keywords: Vec<String>,
}

impl AppConfig {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        Self {
            port: env::var("PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(3010),
            admin_api_key: env::var("ADMIN_API_KEY").unwrap_or_else(|_| "change-me".to_string()),
            google_api_key: env::var("GOOGLE_API_KEY").unwrap_or_default(),
            ai_model: env::var("AI_MODEL").unwrap_or_else(|_| "gemini-1.5-flash".to_string()),
            tavily_api_key: env::var("TAVILY_API_KEY").unwrap_or_default(),
            bridge_secret: env::var("BRIDGE_SECRET").unwrap_or_else(|_| "local_dev_secret_123".to_string()),
            whatsapp_bridge_url: env::var("WHATSAPP_BRIDGE_URL")
                .unwrap_or_else(|_| "http://localhost:8002/send".to_string()),
            briefing_recipient: env::var("BRIEFING_RECIPIENT").ok().filter(|v| !v.trim().is_empty()),
            enable_daily_briefing: env::var("ENABLE_DAILY_BRIEFING")
                .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
                .unwrap_or(false),
            briefing_hour_utc: env::var("BRIEFING_HOUR_UTC").ok().and_then(|v| v.parse().ok()).unwrap_or(7),
            briefing_interval_seconds: env::var("BRIEFING_INTERVAL_SECONDS").ok().and_then(|v| v.parse().ok()),
            briefing_name: env::var("BRIEFING_NAME").unwrap_or_else(|_| "Morning Brief".to_string()),
            interests: split_csv(&env::var("INTERESTS").unwrap_or_else(|_| {
                "AI,Technology,Business Africa,Finance,Sports,Travel,Relocation,Nigeria,Startups".to_string()
            })),
            blocked_keywords: split_csv(&env::var("BLOCKED_KEYWORDS").unwrap_or_default()),
        }
    }
}

fn split_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToString::to_string)
        .collect()
}
