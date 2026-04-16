"""
DAVA Phone Bot - SMTP/Verizon SMS Bid Command Interface
Sends SMS replies via SMTP to the Verizon email-to-SMS gateway.
Receives commands either via IMAP polling or the HTTP JSON API.

How it works:
  - You text your bot email address from your phone
  - Bot reads the email via IMAP (shared with imap_ai_bot.py)
  - Bot replies via SMTP -> Verizon gateway -> SMS to your phone

Verizon SMS gateways:
  SMS:  <number>@vtext.com
  MMS:  <number>@vzwpix.com

Same commands as the IMAP bot:
  BID <item_id> <amount>     - Place a bid
  BID STATUS <item_id>       - Check bids on an item
  BID LIST                   - List active bids
  BID CANCEL <item_id>       - Cancel your bid
  SET TASK <kernel_id> <cmd> - Forward to mesh kernel
  BROADCAST <cmd>            - Broadcast to all kernels
  STATUS                     - System status
  HELP                       - Command list

Environment variables (set in .env or shell):
  SMTP_HOST        - SMTP server  (default: smtp.hoagandfamily.com)
  SMTP_PORT        - SMTP port    (default: 587)
  SMTP_USER        - Your email   (default: Collinhoag@hoagandfamily.com)
  SMTP_PASS        - Email password
  SMTP_USE_TLS     - true/false   (default: true)
  MY_PHONE         - Your Verizon number digits only (default: 4582393215)
  IMAP_HOST        - IMAP server  (default: imap.gmail.com)
  IMAP_PORT        - IMAP port    (default: 993)
  IMAP_USER        - IMAP login   (same as SMTP_USER if not set)
  IMAP_PASS        - IMAP password (same as SMTP_PASS if not set)
  IMAP_FOLDER      - Mailbox      (default: INBOX)
  IMAP_POLL_SEC    - Poll seconds (default: 30)
  BOT_HTTP_PORT    - HTTP API port (default: 8765)
  ONEDRIVE         - OneDrive root path
"""

import email as email_lib
import imaplib
import http.server
import json
import logging
import os
import smtplib
import ssl
import threading
import time
from datetime import datetime
from email.header import decode_header
from email.mime.text import MIMEText
from pathlib import Path
from typing import List

from imap_ai_bot import BidEngine, CommandProcessor, BID_DATA_PATH

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s - %(message)s",
)
logger = logging.getLogger("DAVA.PhoneBot")

# ---------------------------------------------------------------------------
# Config
# ---------------------------------------------------------------------------

SMTP_HOST    = os.environ.get("SMTP_HOST", "smtp.hoagandfamily.com")
SMTP_PORT    = int(os.environ.get("SMTP_PORT", 587))
SMTP_USER    = os.environ.get("SMTP_USER", "Collinhoag@hoagandfamily.com")
SMTP_PASS    = os.environ.get("SMTP_PASS", "")
SMTP_USE_TLS = os.environ.get("SMTP_USE_TLS", "true").lower() != "false"

MY_PHONE     = os.environ.get("MY_PHONE", "4582393215")
SMS_GATEWAY  = f"{MY_PHONE}@vtext.com"   # Verizon email-to-SMS

IMAP_HOST    = os.environ.get("IMAP_HOST", "imap.gmail.com")
IMAP_PORT    = int(os.environ.get("IMAP_PORT", 993))
IMAP_USER    = os.environ.get("IMAP_USER", SMTP_USER)
IMAP_PASS    = os.environ.get("IMAP_PASS", SMTP_PASS)
IMAP_FOLDER  = os.environ.get("IMAP_FOLDER", "INBOX")
IMAP_POLL_SEC = int(os.environ.get("IMAP_POLL_SEC", 30))

BOT_HTTP_PORT = int(os.environ.get("BOT_HTTP_PORT", 8765))

# ---------------------------------------------------------------------------
# SMTP SMS sender
# ---------------------------------------------------------------------------

def send_sms(body: str, to: str = SMS_GATEWAY) -> bool:
    """
    Send an SMS by emailing the Verizon email-to-SMS gateway via SMTP.
    Verizon converts the email body to a text message.
    Keep body under 160 chars for a single SMS segment.
    """
    if not SMTP_PASS:
        logger.warning("SMTP_PASS not set - SMS reply skipped")
        return False

    msg = MIMEText(body[:160])
    msg["From"] = SMTP_USER
    msg["To"] = to
    msg["Subject"] = ""  # Verizon ignores subject; body is the SMS

    try:
        if SMTP_USE_TLS:
            ctx = ssl.create_default_context()
            with smtplib.SMTP(SMTP_HOST, SMTP_PORT) as server:
                server.ehlo()
                server.starttls(context=ctx)
                server.login(SMTP_USER, SMTP_PASS)
                server.sendmail(SMTP_USER, to, msg.as_string())
        else:
            with smtplib.SMTP_SSL(SMTP_HOST, SMTP_PORT) as server:
                server.login(SMTP_USER, SMTP_PASS)
                server.sendmail(SMTP_USER, to, msg.as_string())

        logger.info(f"SMS sent via SMTP -> {to}")
        return True
    except Exception as e:
        logger.error(f"SMTP send failed: {e}")
        return False


def send_sms_async(body: str, to: str = SMS_GATEWAY):
    threading.Thread(target=send_sms, args=(body, to), daemon=True).start()


# ---------------------------------------------------------------------------
# Command processor (shared bid engine)
# ---------------------------------------------------------------------------

class PhoneCommandProcessor:
    def __init__(self):
        self.bid_engine = BidEngine()
        self.processor = CommandProcessor(self.bid_engine)
        self.log_path = BID_DATA_PATH.parent / "phone_bot_log.jsonl"
        self.log_path.parent.mkdir(parents=True, exist_ok=True)

    def handle(self, command: str, sender: str = SMTP_USER) -> str:
        result = self.processor.process(command.strip(), sender)
        self._log(sender, command, result.ok, result.message)
        return result.message

    def _log(self, sender: str, command: str, ok: bool, response: str):
        with open(self.log_path, "a") as f:
            f.write(json.dumps({
                "ts": datetime.utcnow().isoformat(),
                "sender": sender,
                "command": command,
                "ok": ok,
                "response": response,
            }) + "\n")


# ---------------------------------------------------------------------------
# IMAP poller - reads command emails, replies via SMS
# ---------------------------------------------------------------------------

def _decode_str(raw) -> str:
    parts = decode_header(raw or "")
    out = []
    for part, enc in parts:
        if isinstance(part, bytes):
            out.append(part.decode(enc or "utf-8", errors="replace"))
        else:
            out.append(str(part))
    return " ".join(out)


class IMAPCommandPoller:
    """
    Polls IMAP for unseen emails, treats the subject as a command,
    and sends the reply back via SMS to MY_PHONE.
    """

    def __init__(self, processor: PhoneCommandProcessor):
        self.processor = processor

    def _connect(self) -> imaplib.IMAP4_SSL:
        conn = imaplib.IMAP4_SSL(IMAP_HOST, IMAP_PORT)
        conn.login(IMAP_USER, IMAP_PASS)
        conn.select(IMAP_FOLDER)
        return conn

    def poll_once(self):
        try:
            conn = self._connect()
        except Exception as e:
            logger.error(f"IMAP connect failed: {e}")
            return

        try:
            _, data = conn.search(None, "UNSEEN")
            ids = data[0].split()
            if ids:
                logger.info(f"Found {len(ids)} unseen message(s)")

            for uid in ids:
                _, msg_data = conn.fetch(uid, "(RFC822)")
                raw = msg_data[0][1]
                msg = email_lib.message_from_bytes(raw)
                conn.store(uid, "+FLAGS", "\\Seen")

                subject = _decode_str(msg.get("Subject", "")).strip()
                sender  = _decode_str(msg.get("From", "")).lower()

                if not subject:
                    continue

                logger.info(f"Command email from {sender}: {subject!r}")
                reply = self.processor.handle(subject, sender)

                # Reply via SMS to Verizon gateway
                send_sms_async(reply[:160])
                logger.info(f"SMS reply queued: {reply[:80]}")
        finally:
            try:
                conn.logout()
            except Exception:
                pass

    def run(self):
        logger.info(f"IMAPCommandPoller running | {IMAP_HOST} every {IMAP_POLL_SEC}s")
        while True:
            self.poll_once()
            time.sleep(IMAP_POLL_SEC)


# ---------------------------------------------------------------------------
# HTTP JSON API - POST /command for direct use from phone browser/shortcut
# ---------------------------------------------------------------------------

class BotHTTPHandler(http.server.BaseHTTPRequestHandler):
    """
    POST /command   {"command": "BID ITEM1 50", "sender": "optional"}
    GET  /health
    """

    processor: PhoneCommandProcessor = None

    def log_message(self, fmt, *args):
        logger.debug(fmt % args)

    def do_GET(self):
        if self.path == "/health":
            body = json.dumps({
                "status": "ok",
                "sms_gateway": SMS_GATEWAY,
                "ts": datetime.utcnow().isoformat(),
            }).encode()
            self._respond(200, "application/json", body)
        else:
            self.send_response(404)
            self.end_headers()

    def do_POST(self):
        if self.path != "/command":
            self.send_response(404)
            self.end_headers()
            return

        length = int(self.headers.get("Content-Length", 0))
        raw = self.rfile.read(length)
        try:
            data = json.loads(raw)
            command = data.get("command", "").strip()
            sender  = data.get("sender", SMTP_USER)
            if not command:
                raise ValueError("'command' field is required")
            reply = self.processor.handle(command, sender)
            # Also send SMS so result shows up on your phone
            send_sms_async(reply[:160])
            result = {"ok": True, "response": reply}
        except Exception as e:
            result = {"ok": False, "response": str(e)}

        body = json.dumps(result).encode()
        self._respond(200, "application/json", body)

    def _respond(self, code: int, content_type: str, body: bytes):
        self.send_response(code)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


def make_http_server(processor: PhoneCommandProcessor) -> http.server.HTTPServer:
    BotHTTPHandler.processor = processor
    return http.server.HTTPServer(("0.0.0.0", BOT_HTTP_PORT), BotHTTPHandler)


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------

if __name__ == "__main__":
    logger.info("=" * 55)
    logger.info("DAVA Phone Bot (SMTP/Verizon)")
    logger.info(f"  SMTP: {SMTP_USER} -> {SMTP_HOST}:{SMTP_PORT}")
    logger.info(f"  SMS gateway: {SMS_GATEWAY}")
    logger.info(f"  IMAP: {IMAP_USER}@{IMAP_HOST} (poll every {IMAP_POLL_SEC}s)")
    logger.info(f"  HTTP API: 0.0.0.0:{BOT_HTTP_PORT}")
    logger.info("=" * 55)

    if not SMTP_PASS:
        logger.warning("SMTP_PASS not set - SMS replies will be skipped")

    proc = PhoneCommandProcessor()

    # IMAP poller thread
    poller = IMAPCommandPoller(proc)
    threading.Thread(target=poller.run, daemon=True).start()

    # HTTP API server thread
    http_server = make_http_server(proc)
    threading.Thread(target=http_server.serve_forever, daemon=True).start()
    logger.info("Phone bot running. Ctrl-C to stop.")

    try:
        while True:
            time.sleep(60)
    except KeyboardInterrupt:
        logger.info("Phone bot stopped.")
        http_server.shutdown()
