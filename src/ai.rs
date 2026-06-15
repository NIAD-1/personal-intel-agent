use serde_json::Value;

use crate::{models::IntelItem, AppState};

pub async fn write_briefing(state: &AppState, title: &str, items: &[IntelItem]) -> String {
    if state.config.google_api_key.trim().is_empty() {
        return fallback_briefing(title, items);
    }

    let prompt = format!(
        "Write a WhatsApp daily intelligence brief.\n\
         Audience: one busy professional arriving at the office.\n\
         Interests: {}.\n\
         Rules: concise, useful, source-grounded, include links, avoid hype, mark social chatter as signals.\n\
         Format: title, Top 7, then category sections, then Watchlist.\n\n\
         Items:\n{}",
        state.config.interests.join(", "),
        render_items_for_prompt(items)
    );

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        state.config.ai_model, state.config.google_api_key
    );

    let payload = serde_json::json!({
        "contents": [{
            "role": "user",
            "parts": [{ "text": prompt }]
        }],
        "generationConfig": {
            "temperature": 0.25,
            "maxOutputTokens": 1800
        }
    });

    match state.http.post(url).json(&payload).send().await {
        Ok(resp) => match resp.json::<Value>().await {
            Ok(json) => json["candidates"][0]["content"]["parts"][0]["text"]
                .as_str()
                .map(normalize)
                .unwrap_or_else(|| fallback_briefing(title, items)),
            Err(error) => {
                tracing::warn!("AI response parse failed: {error}");
                fallback_briefing(title, items)
            }
        },
        Err(error) => {
            tracing::warn!("AI briefing failed: {error}");
            fallback_briefing(title, items)
        }
    }
}

fn render_items_for_prompt(items: &[IntelItem]) -> String {
    items
        .iter()
        .take(35)
        .enumerate()
        .map(|(index, item)| {
            format!(
                "{}. [{}] {} - {}\nSource: {}\nSummary: {}\nURL: {}",
                index + 1,
                item.category,
                item.title,
                item.source,
                item.source,
                item.summary,
                item.url
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub fn fallback_briefing(title: &str, items: &[IntelItem]) -> String {
    let mut lines = vec![
        format!("*{title}*"),
        String::new(),
        "*Top stories*".to_string(),
    ];

    for (index, item) in items.iter().take(10).enumerate() {
        lines.push(format!(
            "{}. *{}* ({})\n{}\n{}",
            index + 1,
            item.title,
            item.source,
            item.summary,
            item.url
        ));
    }

    lines.push(String::new());
    lines.push("_Reply with `deep dive <topic>` or `today tech`._".to_string());
    lines.join("\n\n")
}

fn normalize(text: &str) -> String {
    text.trim().replace("\n\n\n", "\n\n")
}
