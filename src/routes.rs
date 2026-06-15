use axum::{
    extract::{Json, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};

use crate::{
    brief,
    models::{WhatsAppBridgeMessage, WhatsAppBridgeResponse},
    whatsapp, AppState,
};

pub async fn health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "personal-intel-agent",
        "version": "0.1.0"
    }))
}

pub async fn run_briefing_now(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !authorized(&state, &headers) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "unauthorized" })),
        )
            .into_response();
    }

    let Some(recipient) = state.config.briefing_recipient.as_deref() else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "BRIEFING_RECIPIENT is not set" })),
        )
            .into_response();
    };

    let briefing = brief::build_daily_briefing(&state).await;
    match whatsapp::send_message(&state, recipient, &briefing.text).await {
        Ok(()) => Json(serde_json::json!({
            "status": "sent",
            "items": briefing.items.len()
        }))
        .into_response(),
        Err(error) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "error": error })),
        )
            .into_response(),
    }
}

pub async fn whatsapp_incoming(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<WhatsAppBridgeMessage>,
) -> impl IntoResponse {
    if headers.get("X-Bridge-Auth").and_then(|v| v.to_str().ok())
        != Some(state.config.bridge_secret.as_str())
    {
        return (
            StatusCode::UNAUTHORIZED,
            Json(WhatsAppBridgeResponse {
                reply: Some("Unauthorized.".to_string()),
            }),
        )
            .into_response();
    }

    let text = payload.body.trim();
    if text.is_empty() || is_help(text) {
        return Json(WhatsAppBridgeResponse {
            reply: Some(help_text(payload.push_name.as_deref())),
        })
        .into_response();
    }

    let lower = text.to_ascii_lowercase();
    let topic = lower
        .strip_prefix("deep dive ")
        .or_else(|| lower.strip_prefix("today "))
        .or_else(|| lower.strip_prefix("brief "))
        .map(str::trim);

    let state_clone = state.clone();
    let recipient = payload.sender.clone();
    let topic_owned = topic.map(ToString::to_string);

    tokio::spawn(async move {
        let briefing = match topic_owned.as_deref() {
            Some(topic) if !topic.is_empty() => {
                brief::build_topic_briefing(&state_clone, topic).await
            }
            _ => brief::build_daily_briefing(&state_clone).await,
        };

        if let Err(error) = whatsapp::send_message(&state_clone, &recipient, &briefing.text).await {
            tracing::error!("Failed to send interactive briefing: {error}");
        }
    });

    Json(WhatsAppBridgeResponse {
        reply: Some("On it. I’m gathering the brief and will send it here shortly.".to_string()),
    })
    .into_response()
}

fn authorized(state: &AppState, headers: &HeaderMap) -> bool {
    headers.get("X-Admin-Key").and_then(|v| v.to_str().ok())
        == Some(state.config.admin_api_key.as_str())
}

fn is_help(text: &str) -> bool {
    matches!(
        text.to_ascii_lowercase().as_str(),
        "hi" | "hello" | "hey" | "help" | "menu" | "start" | "/start"
    )
}

fn help_text(name: Option<&str>) -> String {
    let who = name.filter(|v| !v.trim().is_empty()).unwrap_or("there");
    format!(
        "Hi {who}. I’m your personal intelligence agent.\n\n\
         Send:\n\
         - `today` for the full daily brief\n\
         - `today tech` for a category brief\n\
         - `deep dive OpenAI` for a focused brief\n\n\
         Scheduled morning delivery is controlled by BRIEFING_RECIPIENT and ENABLE_DAILY_BRIEFING."
    )
}
