import makeWASocket, {
  useMultiFileAuthState,
  DisconnectReason,
  fetchLatestBaileysVersion,
  makeCacheableSignalKeyStore,
} from "@whiskeysockets/baileys";
import { Boom } from "@hapi/boom";
import pino from "pino";
import QRCode from "qrcode";
import qrcode from "qrcode-terminal";
import fs from "fs";
import http from "http";

const BACKEND_URL = process.env.BACKEND_URL || "http://localhost:3010";
const BRIDGE_PORT = parseInt(process.env.PORT || process.env.BRIDGE_PORT || "8002", 10);
const BRIDGE_SECRET = process.env.BRIDGE_SECRET || "local_dev_secret_123";
const AUTH_FOLDER = process.env.AUTH_FOLDER || "./auth_state";

const logger = pino({ level: "warn" });
const jidMap = new Map();
const processedMessages = new Set();

let sock = null;
let latestQR = null;

process.on("unhandledRejection", (err) => {
  console.error("Unhandled rejection:", err?.message || err);
});

async function startBot() {
  const { state, saveCreds } = await useMultiFileAuthState(AUTH_FOLDER);
  const { version } = await fetchLatestBaileysVersion();

  sock = makeWASocket({
    version,
    auth: {
      creds: state.creds,
      keys: makeCacheableSignalKeyStore(state.keys, logger),
    },
    logger,
    printQRInTerminal: true,
  });

  sock.ev.on("creds.update", saveCreds);

  sock.ev.on("connection.update", (update) => {
    const { connection, lastDisconnect, qr } = update;

    if (qr) {
      latestQR = qr;
      qrcode.generate(qr, { small: true });
      console.log(`QR ready. Open http://localhost:${BRIDGE_PORT}/qr if you prefer a browser view.`);
    }

    if (connection === "open") {
      latestQR = null;
      console.log("WhatsApp connected.");
    }

    if (connection === "close") {
      const reason = new Boom(lastDisconnect?.error)?.output?.statusCode;
      if (reason === DisconnectReason.loggedOut) {
        try {
          fs.rmSync(AUTH_FOLDER, { recursive: true, force: true });
        } catch (_) {}
      }
      console.log(`WhatsApp connection closed (${reason}). Reconnecting...`);
      startBot();
    }
  });

  sock.ev.on("messages.upsert", async ({ messages, type }) => {
    if (type !== "notify") return;

    for (const msg of messages) {
      if (msg.key.fromMe || !msg.message) continue;
      if (processedMessages.has(msg.key.id)) continue;
      processedMessages.add(msg.key.id);
      if (processedMessages.size > 1000) {
        for (const id of Array.from(processedMessages).slice(0, 500)) {
          processedMessages.delete(id);
        }
      }

      const chatJid = msg.key.remoteJid;
      if (!chatJid || chatJid.endsWith("@broadcast") || chatJid.endsWith("@newsletter")) continue;

      const isGroup = chatJid.endsWith("@g.us");
      const participantJid = msg.key.participant || chatJid;
      const phone = participantJid.replace("@s.whatsapp.net", "").replace("@lid", "");
      const pushName = msg.pushName || "";

      jidMap.set(phone, chatJid);
      jidMap.set(chatJid, chatJid);

      let body = "";
      if (msg.message.conversation) {
        body = msg.message.conversation;
      } else if (msg.message.extendedTextMessage?.text) {
        body = msg.message.extendedTextMessage.text;
      } else {
        continue;
      }

      if (isGroup && !shouldHandleGroupText(body)) {
        continue;
      }

      try {
        await sock.readMessages([msg.key]);
        await sock.sendPresenceUpdate("composing", chatJid);
      } catch (_) {}

      console.log(`Incoming ${isGroup ? "group" : "dm"} message from ${pushName || phone}: ${body}`);

      try {
        const resp = await fetch(`${BACKEND_URL}/bridge/incoming`, {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            "X-Bridge-Auth": BRIDGE_SECRET,
          },
          body: JSON.stringify({
            from: chatJid,
            body,
            push_name: pushName,
            message_id: msg.key.id,
          }),
        });

        if (!resp.ok) {
          console.error(`Backend returned ${resp.status}: ${await resp.text()}`);
          continue;
        }

        const data = await resp.json();
        if (data.reply) {
          await sock.sendPresenceUpdate("paused", chatJid);
          await sock.sendMessage(chatJid, { text: data.reply });
        }
      } catch (error) {
        console.error("Backend request failed:", error.message);
      }
    }
  });
}

function shouldHandleGroupText(body) {
  const lower = body.toLowerCase().trim();
  return (
    lower.includes("brief") ||
    lower.includes("deep dive") ||
    lower === "today" ||
    lower.startsWith("today ") ||
    lower.includes("@intel") ||
    lower.includes("news agent")
  );
}

const server = http.createServer(async (req, res) => {
  if (req.method === "POST" && req.url === "/send") {
    if (req.headers["x-bridge-auth"] !== BRIDGE_SECRET) {
      res.writeHead(401, { "Content-Type": "application/json" });
      res.end(JSON.stringify({ error: "unauthorized" }));
      return;
    }

    let body = "";
    req.on("data", (chunk) => (body += chunk));
    req.on("end", async () => {
      try {
        const payload = JSON.parse(body);
        let jid = payload.to;
        if (jidMap.has(jid)) {
          jid = jidMap.get(jid);
        } else if (!jid.includes("@")) {
          jid = `${jid}@s.whatsapp.net`;
        }

        await sock.sendMessage(jid, { text: payload.text || "" });
        res.writeHead(200, { "Content-Type": "application/json" });
        res.end(JSON.stringify({ status: "sent", to: jid }));
      } catch (error) {
        console.error("Send failed:", error);
        res.writeHead(500, { "Content-Type": "application/json" });
        res.end(JSON.stringify({ error: error.message }));
      }
    });
    return;
  }

  if (req.method === "GET" && req.url === "/health") {
    res.writeHead(200, { "Content-Type": "application/json" });
    res.end(JSON.stringify({ status: "ok", connected: !!sock }));
    return;
  }

  if (req.method === "GET" && req.url === "/qr") {
    res.writeHead(200, { "Content-Type": "text/html" });
    if (!latestQR) {
      res.end("<h1>WhatsApp connected</h1>");
      return;
    }
    try {
      const dataUrl = await QRCode.toDataURL(latestQR, { margin: 2, width: 320 });
      res.end(`<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Personal Intel Agent QR</title>
    <style>
      body { margin: 0; min-height: 100vh; display: grid; place-items: center; font-family: system-ui, sans-serif; background: #101214; color: #f5f7fa; }
      main { text-align: center; max-width: 440px; padding: 28px; }
      img { background: white; padding: 12px; border-radius: 8px; }
      p { color: #b8c0cc; line-height: 1.5; }
    </style>
  </head>
  <body>
    <main>
      <h1>Scan WhatsApp QR</h1>
      <img src="${dataUrl}" alt="WhatsApp QR code" />
      <p>Open WhatsApp, go to linked devices, and scan this code. Refresh if it expires.</p>
    </main>
  </body>
</html>`);
    } catch (error) {
      res.end(`<pre>${latestQR}</pre><p>QR render failed: ${error.message}</p>`);
    }
    return;
  }

  res.writeHead(404);
  res.end("not found");
});

server.listen(BRIDGE_PORT, () => {
  console.log(`Bridge listening on http://localhost:${BRIDGE_PORT}`);
  console.log(`Forwarding incoming messages to ${BACKEND_URL}`);
});

startBot();
