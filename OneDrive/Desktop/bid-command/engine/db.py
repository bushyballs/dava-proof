"""engine.db — SQLite bid database for tracking bid pipeline status."""

import sqlite3
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from uuid import uuid4


# ---------------------------------------------------------------------------
# Dataclass
# ---------------------------------------------------------------------------

@dataclass
class BidRecord:
    id: str
    sol_number: str
    title: str
    agency: str
    due_date: str
    status: str  # draft, ready, sent, pending, won, lost
    base_price: float = 0.0
    grand_total: float = 0.0
    co_name: str = ""
    co_email: str = ""
    naics: str = ""
    state: str = ""
    created_at: str = ""
    updated_at: str = ""


# ---------------------------------------------------------------------------
# Database class
# ---------------------------------------------------------------------------

_CREATE_TABLE = """
CREATE TABLE IF NOT EXISTS bids (
    id          TEXT PRIMARY KEY,
    sol_number  TEXT NOT NULL,
    title       TEXT NOT NULL,
    agency      TEXT NOT NULL,
    due_date    TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'draft',
    base_price  REAL NOT NULL DEFAULT 0.0,
    grand_total REAL NOT NULL DEFAULT 0.0,
    co_name     TEXT NOT NULL DEFAULT '',
    co_email    TEXT NOT NULL DEFAULT '',
    naics       TEXT NOT NULL DEFAULT '',
    state       TEXT NOT NULL DEFAULT '',
    created_at  TEXT NOT NULL DEFAULT '',
    updated_at  TEXT NOT NULL DEFAULT ''
)
"""


class BidDB:
    """SQLite-backed bid database."""

    def __init__(self, db_path: Path) -> None:
        self._path = Path(db_path)
        self._init_db()

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------

    def _connect(self) -> sqlite3.Connection:
        conn = sqlite3.connect(self._path)
        conn.execute("PRAGMA journal_mode=WAL")
        conn.row_factory = sqlite3.Row
        return conn

    def _init_db(self) -> None:
        with self._connect() as conn:
            conn.execute(_CREATE_TABLE)
            conn.commit()

    @staticmethod
    def _row_to_record(row: sqlite3.Row) -> BidRecord:
        return BidRecord(
            id=row["id"],
            sol_number=row["sol_number"],
            title=row["title"],
            agency=row["agency"],
            due_date=row["due_date"],
            status=row["status"],
            base_price=row["base_price"],
            grand_total=row["grand_total"],
            co_name=row["co_name"],
            co_email=row["co_email"],
            naics=row["naics"],
            state=row["state"],
            created_at=row["created_at"],
            updated_at=row["updated_at"],
        )

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    def create_bid(
        self,
        sol_number: str,
        title: str,
        agency: str,
        due_date: str,
        **kwargs,
    ) -> BidRecord:
        """Insert a new bid and return the resulting BidRecord."""
        bid_id = uuid4().hex[:8]
        now = datetime.now(timezone.utc).isoformat()

        record = BidRecord(
            id=bid_id,
            sol_number=sol_number,
            title=title,
            agency=agency,
            due_date=due_date,
            status=kwargs.get("status", "draft"),
            base_price=kwargs.get("base_price", 0.0),
            grand_total=kwargs.get("grand_total", 0.0),
            co_name=kwargs.get("co_name", ""),
            co_email=kwargs.get("co_email", ""),
            naics=kwargs.get("naics", ""),
            state=kwargs.get("state", ""),
            created_at=now,
            updated_at=now,
        )

        with self._connect() as conn:
            conn.execute(
                """
                INSERT INTO bids
                    (id, sol_number, title, agency, due_date, status,
                     base_price, grand_total, co_name, co_email,
                     naics, state, created_at, updated_at)
                VALUES
                    (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                """,
                (
                    record.id, record.sol_number, record.title,
                    record.agency, record.due_date, record.status,
                    record.base_price, record.grand_total,
                    record.co_name, record.co_email,
                    record.naics, record.state,
                    record.created_at, record.updated_at,
                ),
            )
            conn.commit()

        return record

    def get_bid(self, bid_id: str) -> BidRecord | None:
        """Fetch a single bid by its id, or None if not found."""
        with self._connect() as conn:
            row = conn.execute(
                "SELECT * FROM bids WHERE id = ?", (bid_id,)
            ).fetchone()
        return self._row_to_record(row) if row else None

    def update_status(self, bid_id: str, status: str) -> None:
        """Update a bid's status and set updated_at to now."""
        now = datetime.now(timezone.utc).isoformat()
        with self._connect() as conn:
            conn.execute(
                "UPDATE bids SET status = ?, updated_at = ? WHERE id = ?",
                (status, now, bid_id),
            )
            conn.commit()

    def list_bids(self, status: str | None = None) -> list[BidRecord]:
        """Return all bids ordered by due_date; optionally filter by status."""
        with self._connect() as conn:
            if status is not None:
                rows = conn.execute(
                    "SELECT * FROM bids WHERE status = ? ORDER BY due_date",
                    (status,),
                ).fetchall()
            else:
                rows = conn.execute(
                    "SELECT * FROM bids ORDER BY due_date"
                ).fetchall()
        return [self._row_to_record(r) for r in rows]
