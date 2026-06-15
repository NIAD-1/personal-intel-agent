use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceConfig {
    pub name: &'static str,
    pub category: &'static str,
    pub url: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelItem {
    pub title: String,
    pub url: String,
    pub source: String,
    pub category: String,
    pub summary: String,
    pub published_at: Option<DateTime<Utc>>,
    pub score: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Briefing {
    pub title: String,
    pub generated_at: DateTime<Utc>,
    pub text: String,
    pub items: Vec<IntelItem>,
}

#[derive(Debug, Deserialize)]
pub struct WhatsAppBridgeMessage {
    #[serde(alias = "from")]
    pub sender: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub push_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WhatsAppBridgeResponse {
    pub reply: Option<String>,
}
