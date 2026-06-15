# Render + WhatsApp Setup

Use two Render web services:

1. `personal-intel-backend` for the Rust briefing API.
2. `personal-intel-bridge` for the WhatsApp Baileys bridge and QR page.

This split is cleaner on Render because each web service gets one public port.

## Backend service

Create a new Render **Web Service** from this repo/folder.

- Runtime: Docker
- Dockerfile path: `Dockerfile.backend`
- Health check path: `/health`

Environment variables:

```bash
ADMIN_API_KEY=make-a-long-random-secret
BRIDGE_SECRET=make-another-long-random-secret
GOOGLE_API_KEY=your-gemini-key
AI_MODEL=gemini-1.5-flash
TAVILY_API_KEY=your-tavily-key
WHATSAPP_BRIDGE_URL=https://YOUR-BRIDGE-SERVICE.onrender.com/send
ENABLE_DAILY_BRIEFING=true
BRIEFING_HOUR_UTC=7
BRIEFING_RECIPIENT=2348012345678
INTERESTS=AI,Technology,Business Africa,Finance,Sports,Travel,Relocation,Nigeria,Startups
```

Render sets `PORT` automatically.

## WhatsApp bridge service

Create a second Render **Web Service** from the `bridge` folder.

- Runtime: Docker
- Root directory: `bridge`
- Dockerfile path: `Dockerfile`
- Health check path: `/health`

Environment variables:

```bash
BACKEND_URL=https://YOUR-BACKEND-SERVICE.onrender.com
BRIDGE_SECRET=same-value-as-backend
AUTH_FOLDER=/var/data/auth_state
```

Add a persistent disk:

- Mount path: `/var/data`
- Size: 1 GB is enough

Without persistent storage, WhatsApp will disconnect after deploys/restarts and you will need to scan the QR code again.

## Connect WhatsApp

After both services deploy:

1. Open `https://YOUR-BRIDGE-SERVICE.onrender.com/qr`.
2. In WhatsApp, go to **Linked devices**.
3. Scan the QR code.
4. Send the bot `today` in DM.

For a group, add the linked WhatsApp account to the group and send `today` once in the group. Then scheduled messages can go to that group once you set `BRIEFING_RECIPIENT` to the group JID shown in the backend/bridge logs.

## UptimeRobot

Create two HTTPS monitors:

- `https://YOUR-BACKEND-SERVICE.onrender.com/health`
- `https://YOUR-BRIDGE-SERVICE.onrender.com/health`

Use a 5-minute interval. This keeps the services warm and also tells you when WhatsApp or the backend falls over.

## Important WhatsApp note

Baileys is not the official WhatsApp Business API. It is practical for a personal agent, but WhatsApp can force re-linking or rate-limit accounts. Keep message volume low, use your own number carefully, and avoid spammy broadcast behavior.

