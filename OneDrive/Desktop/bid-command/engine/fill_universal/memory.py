"""DAVA's field memory — SQLite-backed learning store."""

from __future__ import annotations

import json
import sqlite3
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


class FieldMemory:
    """Persistent memory for field patterns DAVA has learned."""

    def __init__(self, db_path: Path) -> None:
        self._db_path = db_path
        self._conn = sqlite3.connect(str(db_path))
        self._conn.row_factory = sqlite3.Row
        self._init_tables()

    def _init_tables(self) -> None:
        cur = self._conn.cursor()
        cur.executescript("""
            CREATE TABLE IF NOT EXISTS field_memory (
                id INTEGER PRIMARY KEY,
                label_normalized TEXT UNIQUE,
                classification TEXT,
                typical_value TEXT,
                context_key TEXT,
                times_seen INTEGER DEFAULT 0,
                times_approved INTEGER DEFAULT 0,
                last_seen TEXT,
                source_pdfs TEXT DEFAULT '[]'
            );
            CREATE TABLE IF NOT EXISTS template_memory (
                id INTEGER PRIMARY KEY,
                pdf_hash TEXT UNIQUE,
                form_name TEXT,
                field_count INTEGER DEFAULT 0,
                fields_json TEXT DEFAULT '[]',
                times_seen INTEGER DEFAULT 1,
                last_seen TEXT
            );
        """)
        self._conn.commit()

    def store(self, label: str, classification: str, value: str, context_key: str, source_pdf: str, approved: bool) -> None:
        normalized = label.strip().lower()
        now = datetime.now(timezone.utc).isoformat()
        cur = self._conn.cursor()
        existing = cur.execute(
            "SELECT id, times_seen, times_approved, source_pdfs FROM field_memory WHERE label_normalized = ?",
            (normalized,),
        ).fetchone()
        if existing:
            pdfs = json.loads(existing["source_pdfs"])
            if source_pdf not in pdfs:
                pdfs.append(source_pdf)
            cur.execute(
                """UPDATE field_memory SET classification = ?, typical_value = ?, context_key = ?,
                   times_seen = times_seen + 1, times_approved = times_approved + ?,
                   last_seen = ?, source_pdfs = ? WHERE id = ?""",
                (classification, value, context_key, 1 if approved else 0, now, json.dumps(pdfs), existing["id"]),
            )
        else:
            cur.execute(
                """INSERT INTO field_memory (label_normalized, classification, typical_value, context_key,
                   times_seen, times_approved, last_seen, source_pdfs) VALUES (?, ?, ?, ?, 1, ?, ?, ?)""",
                (normalized, classification, value, context_key, 1 if approved else 0, now, json.dumps([source_pdf])),
            )
        self._conn.commit()

    def recall(self, label: str) -> dict[str, Any] | None:
        normalized = label.strip().lower()
        cur = self._conn.cursor()
        row = cur.execute(
            "SELECT * FROM field_memory WHERE label_normalized = ? AND times_approved > 0",
            (normalized,),
        ).fetchone()
        if not row:
            return None
        approvals = row["times_approved"]
        confidence = min(0.95, 0.7 + (approvals / (approvals + 10)) * 0.25)
        return {
            "value": row["typical_value"], "classification": row["classification"],
            "context_key": row["context_key"], "confidence": round(confidence, 3),
            "times_seen": row["times_seen"], "times_approved": row["times_approved"],
        }

    def store_template(self, pdf_hash: str, form_name: str, fields_json: str) -> None:
        now = datetime.now(timezone.utc).isoformat()
        fields = json.loads(fields_json)
        cur = self._conn.cursor()
        existing = cur.execute("SELECT id FROM template_memory WHERE pdf_hash = ?", (pdf_hash,)).fetchone()
        if existing:
            cur.execute(
                """UPDATE template_memory SET form_name = ?, field_count = ?, fields_json = ?,
                   times_seen = times_seen + 1, last_seen = ? WHERE id = ?""",
                (form_name, len(fields), fields_json, now, existing["id"]),
            )
        else:
            cur.execute(
                """INSERT INTO template_memory (pdf_hash, form_name, field_count, fields_json, times_seen, last_seen)
                   VALUES (?, ?, ?, ?, 1, ?)""",
                (pdf_hash, form_name, len(fields), fields_json, now),
            )
        self._conn.commit()

    def recall_template(self, pdf_hash: str) -> dict[str, Any] | None:
        cur = self._conn.cursor()
        row = cur.execute("SELECT * FROM template_memory WHERE pdf_hash = ?", (pdf_hash,)).fetchone()
        if not row:
            return None
        return {"form_name": row["form_name"], "field_count": row["field_count"],
                "fields_json": row["fields_json"], "times_seen": row["times_seen"]}

    def stats(self) -> dict[str, int]:
        cur = self._conn.cursor()
        fields = cur.execute("SELECT COUNT(*) as c FROM field_memory").fetchone()["c"]
        templates = cur.execute("SELECT COUNT(*) as c FROM template_memory").fetchone()["c"]
        return {"total_fields": fields, "total_templates": templates}
