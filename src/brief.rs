use chrono::Utc;

use crate::{
    ai, collectors,
    models::{Briefing, IntelItem},
    AppState,
};

pub async fn build_daily_briefing(state: &AppState) -> Briefing {
    let items = collectors::collect_intel(state, None).await;
    build_from_items(state, &state.config.briefing_name, items).await
}

pub async fn build_topic_briefing(state: &AppState, topic: &str) -> Briefing {
    let items = collectors::collect_intel(state, Some(topic)).await;
    let title = format!("Briefing: {topic}");
    build_from_items(state, &title, items).await
}

async fn build_from_items(state: &AppState, title: &str, items: Vec<IntelItem>) -> Briefing {
    let text = ai::write_briefing(state, title, &items).await;
    Briefing {
        title: title.to_string(),
        generated_at: Utc::now(),
        text,
        items,
    }
}
