# Personal Intelligence Agent

WhatsApp-first daily briefing agent for personal news, finance, sports, business, tech, AI, travel, relocation, and social signals.

It follows the same useful pattern as RAI:

- Rust Axum backend
- Baileys WhatsApp bridge
- scheduled push messages
- interactive WhatsApp commands
- source gathering, dedupe, ranking, and AI summarization

## Current MVP

- Daily brief from curated RSS/news sources
- Hacker News signal collection
- Reddit public search signal collection
- Tavily search support when `TAVILY_API_KEY` is set
- Gemini brief writing when `GOOGLE_API_KEY` is set
- Fallback brief formatting when AI keys are absent
- WhatsApp DM/group delivery through the bridge

X and Facebook should be added as credentialed collectors using official APIs or an approved third-party provider. The app is deliberately structured so those can plug into `src/collectors.rs` without changing the briefing pipeline.

## Run

```bash
cd /Users/work/Desktop/claw-code/personal-intel-agent
cp .env.example .env
cargo run
```

In another terminal:

```bash
cd /Users/work/Desktop/claw-code/personal-intel-agent/bridge
npm install
BACKEND_URL=http://localhost:3010 npm start
```

Scan the WhatsApp QR code, then message the bot:

- `today`
- `today tech`
- `deep dive OpenAI`
- `brief relocation to Canada`

## Scheduled delivery

Set these in `.env`:

```bash
ENABLE_DAILY_BRIEFING=true
BRIEFING_RECIPIENT=2348012345678
BRIEFING_HOUR_UTC=7
```

For a group, send the bot a message inside the group first so the bridge can learn the group JID, then use that JID as `BRIEFING_RECIPIENT`.

For local testing, use:

```bash
BRIEFING_INTERVAL_SECONDS=300
```

## Manual trigger

```bash
curl -X POST http://localhost:3010/api/briefing/run \
  -H "X-Admin-Key: change-me"
```

## Deploy

For Render, use two web services: one backend and one WhatsApp bridge. See [docs/render-setup.md](/Users/work/Desktop/claw-code/personal-intel-agent/docs/render-setup.md).
