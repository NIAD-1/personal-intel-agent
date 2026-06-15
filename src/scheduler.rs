use chrono::{Timelike, Utc};

use crate::{brief, whatsapp, AppState};

pub async fn run_daily_scheduler(state: AppState) {
    tracing::info!("Daily briefing scheduler started");
    let mut last_sent_date = None;

    loop {
        if let Some(interval) = state.config.briefing_interval_seconds {
            tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
            send_scheduled_briefing(&state).await;
            continue;
        }

        let now = Utc::now();
        let today = now.date_naive();
        if now.hour() == state.config.briefing_hour_utc && last_sent_date != Some(today) {
            send_scheduled_briefing(&state).await;
            last_sent_date = Some(today);
        }

        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}

async fn send_scheduled_briefing(state: &AppState) {
    let Some(recipient) = state.config.briefing_recipient.as_deref() else {
        tracing::warn!("BRIEFING_RECIPIENT is not set; skipping scheduled briefing");
        return;
    };

    let briefing = brief::build_daily_briefing(state).await;
    match whatsapp::send_message(state, recipient, &briefing.text).await {
        Ok(()) => tracing::info!("Scheduled briefing sent to {recipient}"),
        Err(error) => tracing::error!("Failed to send scheduled briefing: {error}"),
    }
}
