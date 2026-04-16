"""
DAVA IMAP AI Bot + Bid Engine
Monitors an email inbox via IMAP, parses BID commands and other AI bot
commands, executes them through the DAVA SET TASK mesh, and syncs results
to OneDrive.

Supported commands (sent as email subject lines):
  BID <item_id> <amount>          - Place a bid
  BID STATUS <item_id>            - Check bid status for an item
  BID LIST                        - List all active bids
  BID CANCEL <item_id>            - Cancel a pending bid
  SET TASK <kernel_id> <cmd>      - Forward to mesh command router
  BROADCAST <cmd>                 - Broadcast to all mesh kernels
  STATUS                          - DAVA system status report
  HELP                            - List all commands

Configuration via environment variables:
  IMAP_HOST      - IMAP server hostname  (default: imap.gmail.com)
  IMAP_PORT      - IMAP port             (default: 993)
  IMAP_USER      - Email address
  IMAP_PASS      - Email password / app password
  IMAP_FOLDER    - Mailbox folder        (default: INBOX)
  IMAP_POLL_SEC  - Poll interval seconds (default: 30)
  ONEDRIVE       - Path to OneDrive root (default: C:/Users/colli/OneDrive)
  BOT_ALLOWED    - Comma-separated list of allowed sender emails (optional)
"""

import imaplib
import email
import json
import logging
import os
import time
from dataclasses import dataclass, asdict
from datetime import datetime
from email.header import decode_header
from enum import Enum
from pathlib import Path
from typing import Dict, List, Optional

# ---------------------------------------------------------------------------
# Config
# ---------------------------------------------------------------------------

IMAP_HOST = os.environ.get("IMAP_HOST", "imap.gmail.com")
IMAP_PORT = int(os.environ.get("IMAP_PORT", 993))
IMAP_USER = os.environ.get("IMAP_USER", "")
IMAP_PASS = os.environ.get("IMAP_PASS", "")
IMAP_FOLDER = os.environ.get("IMAP_FOLDER", "INBOX")
IMAP_POLL_SEC = int(os.environ.get("IMAP_POLL_SEC", 30))

ONEDRIVE = os.environ.get("ONEDRIVE", "C:/Users/colli/OneDrive")
BID_DATA_PATH = Path(ONEDRIVE) / "HoagsOS" / "DAVA" / "hot" / "bids"

ALLOWED_SENDERS: List[str] = [
    s.strip().lower()
    for s in os.environ.get("BOT_ALLOWED", "").split(",")
    if s.strip()
]

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s - %(message)s",
)
logger = logging.getLogger("DAVA.IMAPBot")


# ---------------------------------------------------------------------------
# Bid Engine
# ---------------------------------------------------------------------------

class BidStatus(Enum):
    PENDING = "pending"
    ACTIVE = "active"
    WON = "won"
    LOST = "lost"
    CANCELLED = "cancelled"


@dataclass
class Bid:
    bid_id: str
    item_id: str
    amount: float
    bidder: str
    status: BidStatus
    created_at: str
    updated_at: str
    notes: str = ""

    def to_dict(self) -> dict:
        d = asdict(self)
        d["status"] = self.status.value
        return d

    @classmethod
    def from_dict(cls, d: dict) -> "Bid":
        d["status"] = BidStatus(d["status"])
        return cls(**d)


class BidEngine:
    """Manages bids - persists to OneDrive."""

    def __init__(self, data_path: Path = BID_DATA_PATH):
        self.data_path = data_path
        self.data_path.mkdir(parents=True, exist_ok=True)
        self.bids: Dict[str, Bid] = {}
        self._load()

    # -- persistence --------------------------------------------------------

    def _bid_file(self, bid_id: str) -> Path:
        return self.data_path / f"{bid_id}.json"

    def _load(self):
        for f in self.data_path.glob("*.json"):
            try:
                with open(f) as fp:
                    self.bids[f.stem] = Bid.from_dict(json.load(fp))
            except Exception as e:
                logger.warning(f"Could not load bid {f}: {e}")
        logger.info(f"BidEngine loaded {len(self.bids)} bids from {self.data_path}")

    def _save(self, bid: Bid):
        with open(self._bid_file(bid.bid_id), "w") as fp:
            json.dump(bid.to_dict(), fp, indent=2)

    # -- operations ---------------------------------------------------------

    def place_bid(self, item_id: str, amount: float, bidder: str) -> Bid:
        now = datetime.utcnow().isoformat()
        bid_id = f"bid_{item_id}_{int(time.time())}"
        bid = Bid(
            bid_id=bid_id,
            item_id=item_id,
            amount=amount,
            bidder=bidder,
            status=BidStatus.ACTIVE,
            created_at=now,
            updated_at=now,
        )
        self.bids[bid_id] = bid
        self._save(bid)
        logger.info(f"Bid placed: {bid_id} | {item_id} @ {amount} by {bidder}")
        return bid

    def get_status(self, item_id: str) -> List[Bid]:
        return [b for b in self.bids.values() if b.item_id == item_id]

    def list_active(self) -> List[Bid]:
        return [b for b in self.bids.values() if b.status == BidStatus.ACTIVE]

    def cancel_bid(self, item_id: str, bidder: str) -> Optional[Bid]:
        for bid in self.bids.values():
            if bid.item_id == item_id and bid.bidder == bidder and bid.status == BidStatus.ACTIVE:
                bid.status = BidStatus.CANCELLED
                bid.updated_at = datetime.utcnow().isoformat()
                self._save(bid)
                logger.info(f"Bid cancelled: {bid.bid_id}")
                return bid
        return None

    def summary(self) -> str:
        total = len(self.bids)
        active = len(self.list_active())
        return f"Bids total={total} active={active} path={self.data_path}"


# ---------------------------------------------------------------------------
# Command Parser
# ---------------------------------------------------------------------------

class CommandResult:
    def __init__(self, ok: bool, message: str):
        self.ok = ok
        self.message = message

    def __str__(self):
        tag = "OK" if self.ok else "ERR"
        return f"[{tag}] {self.message}"


class CommandProcessor:
    """Parses and dispatches commands from email subjects."""

    def __init__(self, bid_engine: BidEngine):
        self.bid_engine = bid_engine

    def process(self, subject: str, sender: str) -> CommandResult:
        subject = subject.strip()
        upper = subject.upper()

        # BID commands
        if upper.startswith("BID"):
            return self._handle_bid(subject[3:].strip(), sender)

        # SET TASK forwarding
        if upper.startswith("SET TASK"):
            return self._handle_set_task(subject[8:].strip())

        # BROADCAST
        if upper.startswith("BROADCAST"):
            return self._handle_broadcast(subject[9:].strip())

        # STATUS
        if upper == "STATUS":
            return CommandResult(True, self.bid_engine.summary())

        # HELP
        if upper == "HELP":
            return CommandResult(True, _HELP_TEXT)

        return CommandResult(False, f"Unknown command: {subject!r}. Send HELP for usage.")

    # -- bid sub-commands ---------------------------------------------------

    def _handle_bid(self, args: str, sender: str) -> CommandResult:
        upper = args.upper()

        if upper.startswith("STATUS"):
            item_id = args[6:].strip()
            if not item_id:
                return CommandResult(False, "Usage: BID STATUS <item_id>")
            bids = self.bid_engine.get_status(item_id)
            if not bids:
                return CommandResult(True, f"No bids found for item {item_id!r}")
            lines = [f"  {b.bid_id}: {b.amount} [{b.status.value}]" for b in bids]
            return CommandResult(True, f"Bids for {item_id}:\n" + "\n".join(lines))

        if upper == "LIST":
            active = self.bid_engine.list_active()
            if not active:
                return CommandResult(True, "No active bids.")
            lines = [f"  {b.bid_id}: {b.item_id} @ {b.amount}" for b in active]
            return CommandResult(True, "Active bids:\n" + "\n".join(lines))

        if upper.startswith("CANCEL"):
            item_id = args[6:].strip()
            if not item_id:
                return CommandResult(False, "Usage: BID CANCEL <item_id>")
            bid = self.bid_engine.cancel_bid(item_id, sender)
            if bid:
                return CommandResult(True, f"Cancelled bid {bid.bid_id} for item {item_id}")
            return CommandResult(False, f"No active bid found for item {item_id!r} by {sender}")

        # BID <item_id> <amount>
        parts = args.split()
        if len(parts) < 2:
            return CommandResult(False, "Usage: BID <item_id> <amount>")
        item_id = parts[0]
        try:
            amount = float(parts[1])
        except ValueError:
            return CommandResult(False, f"Invalid amount: {parts[1]!r}")
        bid = self.bid_engine.place_bid(item_id, amount, sender)
        return CommandResult(True, f"Bid placed! ID={bid.bid_id} | {item_id} @ {amount}")

    # -- mesh forwarding ----------------------------------------------------

    def _handle_set_task(self, args: str) -> CommandResult:
        # Lazy import to avoid circular deps
        try:
            from set_task_system import DistributedCommandServer
            parts = args.split(None, 1)
            if len(parts) < 2:
                return CommandResult(False, "Usage: SET TASK <kernel_id> <command>")
            kernel_id = int(parts[0])
            command = parts[1]
            server = DistributedCommandServer()
            task_id = server.set_task(kernel_id, command)
            return CommandResult(True, f"Task dispatched: {task_id}")
        except Exception as e:
            return CommandResult(False, f"SET TASK error: {e}")

    def _handle_broadcast(self, command: str) -> CommandResult:
        try:
            from set_task_system import MeshCommandRouter
            router = MeshCommandRouter()
            results = router.route_command(command)
            return CommandResult(True, f"Broadcast sent to {len(results)} nodes")
        except Exception as e:
            return CommandResult(False, f"BROADCAST error: {e}")


_HELP_TEXT = """DAVA IMAP AI Bot commands (send as email subject):
  BID <item_id> <amount>     - Place a bid
  BID STATUS <item_id>       - Check bids on an item
  BID LIST                   - List all active bids
  BID CANCEL <item_id>       - Cancel your active bid on an item
  SET TASK <id> <cmd>        - Send command to mesh kernel
  BROADCAST <cmd>            - Broadcast to all mesh kernels
  STATUS                     - System status summary
  HELP                       - Show this help"""


# ---------------------------------------------------------------------------
# IMAP Client
# ---------------------------------------------------------------------------

def _decode_header_str(raw) -> str:
    """Safely decode an email header value."""
    parts = decode_header(raw or "")
    decoded = []
    for part, enc in parts:
        if isinstance(part, bytes):
            decoded.append(part.decode(enc or "utf-8", errors="replace"))
        else:
            decoded.append(str(part))
    return " ".join(decoded)


class IMAPBot:
    """
    Polls an IMAP mailbox, parses command emails, dispatches via
    CommandProcessor, and logs replies to OneDrive.
    """

    def __init__(self):
        self.bid_engine = BidEngine()
        self.processor = CommandProcessor(self.bid_engine)
        self.log_path = BID_DATA_PATH.parent / "bot_log.jsonl"
        self.log_path.parent.mkdir(parents=True, exist_ok=True)

    # -- IMAP connection ----------------------------------------------------

    def _connect(self) -> imaplib.IMAP4_SSL:
        conn = imaplib.IMAP4_SSL(IMAP_HOST, IMAP_PORT)
        conn.login(IMAP_USER, IMAP_PASS)
        conn.select(IMAP_FOLDER)
        return conn

    # -- email fetching -----------------------------------------------------

    def _fetch_unseen(self, conn: imaplib.IMAP4_SSL) -> List[email.message.Message]:
        _, data = conn.search(None, "UNSEEN")
        ids = data[0].split()
        messages = []
        for uid in ids:
            _, msg_data = conn.fetch(uid, "(RFC822)")
            raw = msg_data[0][1]
            msg = email.message_from_bytes(raw)
            messages.append(msg)
            # Mark as seen
            conn.store(uid, "+FLAGS", "\\Seen")
        return messages

    def _get_subject(self, msg: email.message.Message) -> str:
        return _decode_header_str(msg.get("Subject", ""))

    def _get_sender(self, msg: email.message.Message) -> str:
        return _decode_header_str(msg.get("From", "")).lower()

    # -- authorization ------------------------------------------------------

    def _is_allowed(self, sender: str) -> bool:
        if not ALLOWED_SENDERS:
            return True  # open if no allowlist configured
        return any(allowed in sender for allowed in ALLOWED_SENDERS)

    # -- logging ------------------------------------------------------------

    def _log(self, entry: dict):
        with open(self.log_path, "a") as f:
            f.write(json.dumps(entry) + "\n")

    # -- main loop ----------------------------------------------------------

    def poll_once(self):
        try:
            conn = self._connect()
        except Exception as e:
            logger.error(f"IMAP connect failed: {e}")
            return

        try:
            messages = self._fetch_unseen(conn)
            logger.info(f"Fetched {len(messages)} unseen message(s)")

            for msg in messages:
                subject = self._get_subject(msg)
                sender = self._get_sender(msg)

                if not self._is_allowed(sender):
                    logger.warning(f"Ignored message from unauthorized sender: {sender}")
                    continue

                logger.info(f"Processing: [{sender}] {subject!r}")
                result = self.processor.process(subject, sender)
                logger.info(str(result))

                self._log({
                    "ts": datetime.utcnow().isoformat(),
                    "sender": sender,
                    "subject": subject,
                    "ok": result.ok,
                    "response": result.message,
                })
        finally:
            try:
                conn.logout()
            except Exception:
                pass

    def run(self):
        logger.info(f"DAVA IMAP AI Bot starting | host={IMAP_HOST} user={IMAP_USER}")
        logger.info(f"Polling every {IMAP_POLL_SEC}s | bids -> {BID_DATA_PATH}")
        if not IMAP_USER or not IMAP_PASS:
            logger.error("IMAP_USER and IMAP_PASS must be set. Exiting.")
            return

        while True:
            self.poll_once()
            time.sleep(IMAP_POLL_SEC)


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------

if __name__ == "__main__":
    bot = IMAPBot()
    bot.run()
