use crate::AppState;

pub async fn send_message(state: &AppState, to: &str, text: &str) -> Result<(), String> {
    let payload = serde_json::json!({
        "to": to,
        "text": text
    });

    let resp = state
        .http
        .post(&state.config.whatsapp_bridge_url)
        .header("X-Bridge-Auth", &state.config.bridge_secret)
        .json(&payload)
        .send()
        .await
        .map_err(|error| format!("bridge request failed: {error}"))?;

    if !resp.status().is_success() {
        return Err(format!("bridge returned {}", resp.status()));
    }

    Ok(())
}
