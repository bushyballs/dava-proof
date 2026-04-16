"""
DAVA Phone Bot - SMS / Mobile Bid Command Interface
Lets you send BID commands and IMAP bot commands from your phone via:

  1. SMS (Twilio webhook) - text commands to your Twilio number
  2. HTTP endpoint       - POST from any mobile app or shortcut

Same command set as the IMAP bot:
  BID <item_id> <amount>
  BID STATUS <item_id>
  BID LIST
  BID CANCEL <item_id>
  STATUS
  HELP

Environment variables:
  TWILIO_ACCOUNT_SID  - Twilio account SID
  TWILIO_AUTH_TOKEN   - Twilio auth token
  TWILIO_FROM_NUMBER  - Your Twilio phone number  (+1xxxxxxxxxx)
  PHONE_ALLOWED       - Comma-separated allowed phone numbers (+1xxxxxxxxxx,...)
  BOT_HTTP_PORT       - HTTP server port (default: 8765)
  ONEDRIVE            - Path to OneDrive root
"""

import hashlib
import hmac
import http.server
import json
import logging
import os
import threading
import time
import urllib.parse
from datetime import datetime
from pathlib import Path
from typing import Optional

# Reuse the core bid engine and command processor from the IMAP bot
from imap_ai_bot import BidEngine, CommandProcessor, BID_DATA_PATH

logger = logging.getLogger("DAVA.PhoneBot")
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s - %(message)s",
)

# ---------------------------------------------------------------------------
# Config
# ---------------------------------------------------------------------------

TWILIO_ACCOUNT_SID = os.environ.get("TWILIO_ACCOUNT_SID", "")
TWILIO_AUTH_TOKEN = os.environ.get("TWILIO_AUTH_TOKEN", "")
TWILIO_FROM_NUMBER = os.environ.get("TWILIO_FROM_NUMBER", "")
BOT_HTTP_PORT = int(os.environ.get("BOT_HTTP_PORT", 8765))

ALLOWED_PHONES = [
    p.strip()
    for p in os.environ.get("PHONE_ALLOWED", "").split(",")
    if p.strip()
]

# ---------------------------------------------------------------------------
# SMS sender (Twilio REST API - no SDK needed)
# ---------------------------------------------------------------------------

def send_sms(to: str, body: str) -> bool:
    """Send an SMS reply via Twilio REST API."""
    if not TWILIO_ACCOUNT_SID or not TWILIO_AUTH_TOKEN or not TWILIO_FROM_NUMBER:
        logger.warning("Twilio not configured - SMS reply skipped")
        return False

    import urllib.request
    import base64

    url = f"https://api.twilio.com/2010-04-01/Accounts/{TWILIO_ACCOUNT_SID}/Messages.json"
    data = urllib.parse.urlencode({
        "From": TWILIO_FROM_NUMBER,
        "To": to,
        "Body": body[:1600],  # Twilio SMS limit
    }).encode()

    creds = base64.b64encode(f"{TWILIO_ACCOUNT_SID}:{TWILIO_AUTH_TOKEN}".encode()).decode()
    req = urllib.request.Request(url, data=data, headers={
        "Authorization": f"Basic {creds}",
        "Content-Type": "application/x-www-form-urlencoded",
    })
    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            logger.info(f"SMS sent to {to} | HTTP {resp.status}")
            return True
    except Exception as e:
        logger.error(f"SMS send failed: {e}")
        return False


def validate_twilio_signature(signature: str, url: str, params: dict) -> bool:
    """Validate that an incoming request is genuinely from Twilio."""
    if not TWILIO_AUTH_TOKEN:
        return True  # skip validation if not configured
    s = url + "".join(f"{k}{v}" for k, v in sorted(params.items()))
    expected = hmac.new(TWILIO_AUTH_TOKEN.encode(), s.encode(), hashlib.sha1).digest()
    import base64
    expected_b64 = base64.b64encode(expected).decode()
    return hmac.compare_digest(signature, expected_b64)


# ---------------------------------------------------------------------------
# Phone Command Processor
# ---------------------------------------------------------------------------

class PhoneCommandProcessor:
    def __init__(self):
        self.bid_engine = BidEngine()
        self.processor = CommandProcessor(self.bid_engine)
        self.log_path = BID_DATA_PATH.parent / "phone_bot_log.jsonl"
        self.log_path.parent.mkdir(parents=True, exist_ok=True)

    def is_allowed(self, phone: str) -> bool:
        if not ALLOWED_PHONES:
            return True
        return phone in ALLOWED_PHONES

    def handle(self, command: str, sender: str) -> str:
        command = command.strip()
        result = self.processor.process(command, sender)
        self._log(sender, command, result.ok, result.message)
        return result.message

    def _log(self, sender: str, command: str, ok: bool, response: str):
        entry = {
            "ts": datetime.utcnow().isoformat(),
            "sender": sender,
            "command": command,
            "ok": ok,
            "response": response,
        }
        with open(self.log_path, "a") as f:
            f.write(json.dumps(entry) + "\n")


# ---------------------------------------------------------------------------
# HTTP / Webhook Server
# ---------------------------------------------------------------------------

class BotHTTPHandler(http.server.BaseHTTPRequestHandler):
    """
    Handles two routes:
      POST /sms      - Twilio webhook (TwiML response)
      POST /command  - Direct JSON API for mobile apps / iOS Shortcuts

    Direct API usage:
      POST /command
      Content-Type: application/json
      {"command": "BID ITEM123 50.00", "sender": "+15551234567"}

      Response:
      {"ok": true, "response": "Bid placed! ..."}
    """

    processor: PhoneCommandProcessor = None  # set by server factory

    def log_message(self, fmt, *args):
        logger.info(fmt % args)

    def do_POST(self):
        if self.path == "/sms":
            self._handle_sms()
        elif self.path == "/command":
            self._handle_command()
        else:
            self.send_response(404)
            self.end_headers()

    # -- Twilio SMS webhook ------------------------------------------------

    def _handle_sms(self):
        length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(length).decode()
        params = dict(urllib.parse.parse_qsl(body))

        from_number = params.get("From", "unknown")
        text = params.get("Body", "").strip()

        if not self.processor.is_allowed(from_number):
            twiml = self._twiml("Not authorized.")
        else:
            response = self.processor.handle(text, from_number)
            # Also fire off an SMS reply asynchronously
            threading.Thread(
                target=send_sms, args=(from_number, response), daemon=True
            ).start()
            twiml = self._twiml(response)

        self.send_response(200)
        self.send_header("Content-Type", "text/xml")
        self.end_headers()
        self.wfile.write(twiml.encode())

    def _twiml(self, message: str) -> str:
        safe = message.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")
        return f'<?xml version="1.0" encoding="UTF-8"?><Response><Message>{safe}</Message></Response>'

    # -- Direct JSON API ---------------------------------------------------

    def _handle_command(self):
        length = int(self.headers.get("Content-Length", 0))
        raw = self.rfile.read(length)
        try:
            data = json.loads(raw)
            command = data.get("command", "").strip()
            sender = data.get("sender", "http_client")

            if not command:
                raise ValueError("'command' field required")

            if not self.processor.is_allowed(sender):
                result = {"ok": False, "response": "Not authorized."}
            else:
                reply = self.processor.handle(command, sender)
                result = {"ok": True, "response": reply}
        except Exception as e:
            result = {"ok": False, "response": str(e)}

        payload = json.dumps(result).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    # -- GET /health -------------------------------------------------------

    def do_GET(self):
        if self.path == "/health":
            payload = json.dumps({"status": "ok", "ts": datetime.utcnow().isoformat()}).encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(payload)
        else:
            self.send_response(404)
            self.end_headers()


def make_server(processor: PhoneCommandProcessor) -> http.server.HTTPServer:
    # Inject processor as class attribute so handlers share it
    BotHTTPHandler.processor = processor
    server = http.server.HTTPServer(("0.0.0.0", BOT_HTTP_PORT), BotHTTPHandler)
    return server


# ---------------------------------------------------------------------------
# SMS Poller (alternative: poll-based instead of webhook)
# Poll a Twilio inbox for unread SMS messages
# ---------------------------------------------------------------------------

class TwilioSMSPoller:
    """
    Polls Twilio for new inbound SMS messages and processes them.
    Use this when you can't expose a public webhook (e.g. home network).
    """

    def __init__(self, processor: PhoneCommandProcessor, poll_sec: int = 30):
        self.processor = processor
        self.poll_sec = poll_sec
        self.seen_sids: set = set()

    def _fetch_messages(self) -> list:
        import urllib.request, base64
        url = (
            f"https://api.twilio.com/2010-04-01/Accounts/"
            f"{TWILIO_ACCOUNT_SID}/Messages.json?PageSize=20&Direction=inbound"
        )
        creds = base64.b64encode(
            f"{TWILIO_ACCOUNT_SID}:{TWILIO_AUTH_TOKEN}".encode()
        ).decode()
        req = urllib.request.Request(url, headers={"Authorization": f"Basic {creds}"})
        try:
            with urllib.request.urlopen(req, timeout=10) as resp:
                data = json.loads(resp.read())
                return data.get("messages", [])
        except Exception as e:
            logger.error(f"Twilio fetch failed: {e}")
            return []

    def poll_once(self):
        messages = self._fetch_messages()
        for msg in messages:
            sid = msg.get("sid")
            if sid in self.seen_sids:
                continue
            self.seen_sids.add(sid)

            from_number = msg.get("from", "unknown")
            body = msg.get("body", "").strip()
            if not body:
                continue

            if not self.processor.is_allowed(from_number):
                logger.warning(f"Ignored SMS from unauthorized number: {from_number}")
                continue

            logger.info(f"SMS command from {from_number}: {body!r}")
            reply = self.processor.handle(body, from_number)
            send_sms(from_number, reply)

    def run(self):
        logger.info(f"TwilioSMSPoller running | poll every {self.poll_sec}s")
        while True:
            self.poll_once()
            time.sleep(self.poll_sec)


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------

if __name__ == "__main__":
    proc = PhoneCommandProcessor()

    # Start HTTP server for Twilio webhook or direct API calls
    server = make_server(proc)
    srv_thread = threading.Thread(target=server.serve_forever, daemon=True)
    srv_thread.start()
    logger.info(f"Phone bot HTTP server listening on port {BOT_HTTP_PORT}")
    logger.info("Routes: POST /sms (Twilio webhook)  POST /command (JSON API)  GET /health")

    # If no public URL for webhooks, fall back to SMS polling
    if TWILIO_ACCOUNT_SID and TWILIO_AUTH_TOKEN:
        poller = TwilioSMSPoller(proc)
        poll_thread = threading.Thread(target=poller.run, daemon=True)
        poll_thread.start()
        logger.info("Twilio SMS poller started")

    logger.info("Phone bot running. Ctrl-C to stop.")
    try:
        while True:
            time.sleep(60)
    except KeyboardInterrupt:
        logger.info("Phone bot stopped.")
        server.shutdown()
