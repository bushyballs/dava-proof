# pdffill — Universal PDF Filler Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a universal PDF field detection and filling engine that handles any PDF using a 3-tier detection system, DAVA + Claude intelligence stack, and learning memory.

**Architecture:** Layered Python module at `engine/fill_universal/`. Four stages (detect -> classify -> fill -> render) as separate modules behind a single `fill_pdf()` entry point. DAVA's memory in SQLite. Ollama for local inference, Anthropic SDK for Claude escalation. All PyMuPDF for PDF operations.

**Tech Stack:** Python 3.14, PyMuPDF (fitz), SQLite, Ollama API (HTTP), Anthropic SDK, pytest

**Spec:** `docs/superpowers/specs/2026-04-08-pdffill-universal-design.md`

---

## File Structure

```
engine/fill_universal/
    __init__.py          — Public API: fill_pdf(), detect_fields()
    models.py            — Dataclasses: DetectedField, ClassifiedField, FilledField, FillResult
    detect.py            — 3-tier field detection (AcroForm, structural, vision)
    classify.py          — Rule-based + DAVA memory field classification
    fill.py              — 4-level intelligence stack (context, memory, DAVA, Claude)
    render.py            — Text placement, font sizing, confidence overlay
    memory.py            — DAVA's SQLite memory (field_memory, template_memory)
    context.py           — Context file loader (JSON -> nested dict)
    cli.py               — CLI entry point: fill, detect, memory subcommands

tests/
    test_fill_universal/
        __init__.py
        test_models.py
        test_detect.py
        test_classify.py
        test_fill.py
        test_render.py
        test_memory.py
        test_context.py
        test_cli.py
        test_integration.py
        fixtures/
            sample_acroform.pdf    — PDF with native form fields
            sample_flat.pdf        — PDF with underscored blanks, no form fields
            sample_table.pdf       — PDF with empty table cells
            company.json           — Test context file
```

---

### Task 1: Models — Data Structures

**Files:**
- Create: `engine/fill_universal/__init__.py`
- Create: `engine/fill_universal/models.py`
- Test: `tests/test_fill_universal/__init__.py`
- Test: `tests/test_fill_universal/test_models.py`

- [ ] **Step 1: Write the failing test**

```python
# tests/test_fill_universal/test_models.py
"""Tests for fill_universal data models."""

from engine.fill_universal.models import (
    DetectedField,
    ClassifiedField,
    FilledField,
    FillResult,
)


def test_detected_field_defaults():
    f = DetectedField(page=0, bbox=(10, 20, 200, 35), label="Name")
    assert f.page == 0
    assert f.bbox == (10, 20, 200, 35)
    assert f.label == "Name"
    assert f.field_type == "text"
    assert f.source == "structural"
    assert f.widget_name == ""


def test_classified_field_from_detected():
    det = DetectedField(page=0, bbox=(10, 20, 200, 35), label="Name")
    clf = ClassifiedField.from_detected(det, classification="identity.name", confidence=0.95)
    assert clf.label == "Name"
    assert clf.classification == "identity.name"
    assert clf.confidence == 0.95
    assert clf.page == 0


def test_filled_field_from_classified():
    det = DetectedField(page=0, bbox=(10, 20, 200, 35), label="Name")
    clf = ClassifiedField.from_detected(det, classification="identity.name", confidence=0.95)
    filled = FilledField.from_classified(clf, value="Hoags Inc.", source_level="context", confidence=1.0)
    assert filled.value == "Hoags Inc."
    assert filled.source_level == "context"
    assert filled.confidence == 1.0


def test_fill_result_summary():
    det = DetectedField(page=0, bbox=(10, 20, 200, 35), label="Name")
    clf = ClassifiedField.from_detected(det, classification="identity.name", confidence=0.95)
    f1 = FilledField.from_classified(clf, value="Hoags Inc.", source_level="context", confidence=1.0)
    f2 = FilledField.from_classified(clf, value="Unknown", source_level="dava_reason", confidence=0.4)
    result = FillResult(fields=[f1, f2], filled_pdf_path="out/filled.pdf")
    assert result.total_fields == 2
    assert result.green_count == 1
    assert result.red_count == 1
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /c/Users/colli/OneDrive/Desktop/bid-command && python -m pytest tests/test_fill_universal/test_models.py -v`
Expected: FAIL — module not found

- [ ] **Step 3: Write minimal implementation**

```python
# engine/fill_universal/__init__.py
"""fill_universal — Universal PDF field detection and filling engine."""
```

```python
# engine/fill_universal/models.py
"""Data models for the fill_universal pipeline."""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path


@dataclass
class DetectedField:
    """A detected fillable region on a PDF page."""
    page: int
    bbox: tuple[float, float, float, float]  # x0, y0, x1, y1
    label: str
    field_type: str = "text"       # text, checkbox, signature, date, currency, essay
    source: str = "structural"     # acroform, structural, vision
    widget_name: str = ""


@dataclass
class ClassifiedField:
    """A detected field enriched with classification."""
    page: int
    bbox: tuple[float, float, float, float]
    label: str
    field_type: str
    source: str
    widget_name: str
    classification: str            # identity.name, temporal.date, currency, etc.
    confidence: float              # 0.0 - 1.0

    @classmethod
    def from_detected(cls, det: DetectedField, classification: str, confidence: float) -> ClassifiedField:
        return cls(
            page=det.page,
            bbox=det.bbox,
            label=det.label,
            field_type=det.field_type,
            source=det.source,
            widget_name=det.widget_name,
            classification=classification,
            confidence=confidence,
        )


@dataclass
class FilledField:
    """A classified field with a generated value."""
    page: int
    bbox: tuple[float, float, float, float]
    label: str
    field_type: str
    source: str
    widget_name: str
    classification: str
    value: str
    source_level: str              # context, dava_memory, dava_reason, claude
    confidence: float

    @classmethod
    def from_classified(cls, clf: ClassifiedField, value: str, source_level: str, confidence: float) -> FilledField:
        return cls(
            page=clf.page,
            bbox=clf.bbox,
            label=clf.label,
            field_type=clf.field_type,
            source=clf.source,
            widget_name=clf.widget_name,
            classification=clf.classification,
            value=value,
            source_level=source_level,
            confidence=confidence,
        )


@dataclass
class FillResult:
    """Result of filling a PDF."""
    fields: list[FilledField] = field(default_factory=list)
    filled_pdf_path: str = ""
    overlay_pdf_path: str = ""
    report_path: str = ""

    @property
    def total_fields(self) -> int:
        return len(self.fields)

    @property
    def green_count(self) -> int:
        return sum(1 for f in self.fields if f.confidence >= 0.85)

    @property
    def yellow_count(self) -> int:
        return sum(1 for f in self.fields if 0.5 <= f.confidence < 0.85)

    @property
    def red_count(self) -> int:
        return sum(1 for f in self.fields if f.confidence < 0.5)
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd /c/Users/colli/OneDrive/Desktop/bid-command && python -m pytest tests/test_fill_universal/test_models.py -v`
Expected: 4 passed

- [ ] **Step 5: Commit**

```bash
cd /c/Users/colli/OneDrive/Desktop/bid-command
git add engine/fill_universal/__init__.py engine/fill_universal/models.py tests/test_fill_universal/
git commit -m "feat(pdffill): add data models — DetectedField, ClassifiedField, FilledField, FillResult"
```

---

### Task 2: Context Loader — Read Context Files

**Files:**
- Create: `engine/fill_universal/context.py`
- Test: `tests/test_fill_universal/test_context.py`
- Create: `tests/test_fill_universal/fixtures/company.json`

- [ ] **Step 1: Create test fixture**

```json
// tests/test_fill_universal/fixtures/company.json
{
    "identity": {
        "name": "Hoags Inc.",
        "cage": "15XV5",
        "uei": "DUHWVUXFNPV5",
        "address": "4075 Aerial Way Apt 152, Eugene, OR 97402-8738",
        "phone": "(458) 239-3215",
        "email": "collinhoag@hoagsandfamily.com",
        "signer": "Collin Hoag",
        "title": "President"
    },
    "bid": {
        "date": "04/08/2026"
    }
}
```

- [ ] **Step 2: Write the failing test**

```python
# tests/test_fill_universal/test_context.py
"""Tests for context file loading."""

from pathlib import Path
from engine.fill_universal.context import load_context, resolve_key


FIXTURES = Path(__file__).parent / "fixtures"


def test_load_context_from_json():
    ctx = load_context(FIXTURES / "company.json")
    assert ctx["identity"]["name"] == "Hoags Inc."
    assert ctx["identity"]["cage"] == "15XV5"
    assert ctx["bid"]["date"] == "04/08/2026"


def test_load_context_from_dict():
    raw = {"identity": {"name": "Test Co."}}
    ctx = load_context(raw)
    assert ctx["identity"]["name"] == "Test Co."


def test_resolve_key_dotted():
    ctx = {"identity": {"name": "Hoags Inc.", "phone": "(458) 239-3215"}}
    assert resolve_key(ctx, "identity.name") == "Hoags Inc."
    assert resolve_key(ctx, "identity.phone") == "(458) 239-3215"


def test_resolve_key_missing_returns_none():
    ctx = {"identity": {"name": "Hoags Inc."}}
    assert resolve_key(ctx, "identity.fax") is None
    assert resolve_key(ctx, "nonexistent.key") is None


def test_resolve_key_top_level():
    ctx = {"date": "04/08/2026"}
    assert resolve_key(ctx, "date") == "04/08/2026"
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cd /c/Users/colli/OneDrive/Desktop/bid-command && python -m pytest tests/test_fill_universal/test_context.py -v`
Expected: FAIL — cannot import context

- [ ] **Step 4: Write minimal implementation**

```python
# engine/fill_universal/context.py
"""Context file loader — reads JSON context into a nested dict."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any


def load_context(source: Path | dict) -> dict:
    """Load context from a JSON file path or a raw dict.

    Args:
        source: Path to a JSON file, or an already-loaded dict.

    Returns:
        Nested context dict.
    """
    if isinstance(source, dict):
        return source

    path = Path(source)
    with open(path, "r") as f:
        return json.load(f)


def resolve_key(ctx: dict, dotted_key: str) -> Any | None:
    """Resolve a dotted key path against a nested context dict.

    Args:
        ctx: Nested dict (e.g. {"identity": {"name": "Hoags Inc."}}).
        dotted_key: Dot-separated path (e.g. "identity.name").

    Returns:
        The value at that path, or None if any segment is missing.
    """
    parts = dotted_key.split(".")
    current = ctx
    for part in parts:
        if not isinstance(current, dict) or part not in current:
            return None
        current = current[part]
    return current
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cd /c/Users/colli/OneDrive/Desktop/bid-command && python -m pytest tests/test_fill_universal/test_context.py -v`
Expected: 5 passed

- [ ] **Step 6: Commit**

```bash
cd /c/Users/colli/OneDrive/Desktop/bid-command
git add engine/fill_universal/context.py tests/test_fill_universal/test_context.py tests/test_fill_universal/fixtures/
git commit -m "feat(pdffill): add context loader — JSON file and dict input, dotted key resolution"
```

---

### Task 3: DAVA Memory — SQLite Learning Store

**Files:**
- Create: `engine/fill_universal/memory.py`
- Test: `tests/test_fill_universal/test_memory.py`

- [ ] **Step 1: Write the failing test**

```python
# tests/test_fill_universal/test_memory.py
"""Tests for DAVA's field memory."""

import tempfile
from pathlib import Path
from engine.fill_universal.memory import FieldMemory


def _tmp_memory() -> FieldMemory:
    """Create a FieldMemory backed by a temporary SQLite file."""
    tmp = tempfile.mktemp(suffix=".db")
    return FieldMemory(Path(tmp))


def test_empty_recall_returns_none():
    mem = _tmp_memory()
    assert mem.recall("offeror name") is None


def test_store_and_recall():
    mem = _tmp_memory()
    mem.store(
        label="offeror name",
        classification="identity.name",
        value="Hoags Inc.",
        context_key="identity.name",
        source_pdf="test.pdf",
        approved=True,
    )
    hit = mem.recall("offeror name")
    assert hit is not None
    assert hit["value"] == "Hoags Inc."
    assert hit["classification"] == "identity.name"
    assert hit["times_approved"] == 1


def test_store_increments_counts():
    mem = _tmp_memory()
    mem.store("offeror name", "identity.name", "Hoags Inc.", "identity.name", "a.pdf", True)
    mem.store("offeror name", "identity.name", "Hoags Inc.", "identity.name", "b.pdf", True)
    mem.store("offeror name", "identity.name", "Hoags Inc.", "identity.name", "c.pdf", False)
    hit = mem.recall("offeror name")
    assert hit["times_seen"] == 3
    assert hit["times_approved"] == 2


def test_recall_confidence_scales_with_approvals():
    mem = _tmp_memory()
    # 1 approval -> low confidence
    mem.store("field a", "identity.name", "val", "key", "a.pdf", True)
    hit1 = mem.recall("field a")
    # 10 approvals -> higher confidence
    for i in range(9):
        mem.store("field a", "identity.name", "val", "key", f"{i}.pdf", True)
    hit10 = mem.recall("field a")
    assert hit10["confidence"] > hit1["confidence"]


def test_unapproved_not_recalled():
    mem = _tmp_memory()
    mem.store("secret", "identity.name", "val", "key", "a.pdf", approved=False)
    assert mem.recall("secret") is None


def test_store_template():
    mem = _tmp_memory()
    mem.store_template(
        pdf_hash="abc123",
        form_name="SF-1449",
        fields_json='[{"label": "Name"}]',
    )
    hit = mem.recall_template("abc123")
    assert hit is not None
    assert hit["form_name"] == "SF-1449"


def test_recall_template_miss():
    mem = _tmp_memory()
    assert mem.recall_template("nonexistent") is None


def test_memory_stats():
    mem = _tmp_memory()
    mem.store("a", "identity.name", "v", "k", "x.pdf", True)
    mem.store("b", "currency", "v", "k", "x.pdf", True)
    stats = mem.stats()
    assert stats["total_fields"] == 2
    assert stats["total_templates"] == 0
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /c/Users/colli/OneDrive/Desktop/bid-command && python -m pytest tests/test_fill_universal/test_memory.py -v`
Expected: FAIL — cannot import memory

- [ ] **Step 3: Write minimal implementation**

```python
# engine/fill_universal/memory.py
"""DAVA's field memory — SQLite-backed learning store."""

from __future__ import annotations

import json
import sqlite3
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


class FieldMemory:
    """Persistent memory for field patterns DAVA has learned.

    Stores label -> (classification, value, context_key) mappings with
    usage counts and approval flags. Only recalls approved entries.
    """

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

    def store(
        self,
        label: str,
        classification: str,
        value: str,
        context_key: str,
        source_pdf: str,
        approved: bool,
    ) -> None:
        """Store or update a field pattern in memory."""
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
                """UPDATE field_memory
                   SET classification = ?, typical_value = ?, context_key = ?,
                       times_seen = times_seen + 1,
                       times_approved = times_approved + ?,
                       last_seen = ?, source_pdfs = ?
                   WHERE id = ?""",
                (classification, value, context_key, 1 if approved else 0, now, json.dumps(pdfs), existing["id"]),
            )
        else:
            cur.execute(
                """INSERT INTO field_memory
                   (label_normalized, classification, typical_value, context_key,
                    times_seen, times_approved, last_seen, source_pdfs)
                   VALUES (?, ?, ?, ?, 1, ?, ?, ?)""",
                (normalized, classification, value, context_key, 1 if approved else 0, now, json.dumps([source_pdf])),
            )
        self._conn.commit()

    def recall(self, label: str) -> dict[str, Any] | None:
        """Recall a field pattern by label. Only returns approved entries."""
        normalized = label.strip().lower()
        cur = self._conn.cursor()
        row = cur.execute(
            "SELECT * FROM field_memory WHERE label_normalized = ? AND times_approved > 0",
            (normalized,),
        ).fetchone()

        if not row:
            return None

        # Confidence scales with approvals: asymptotic toward 0.95
        approvals = row["times_approved"]
        confidence = min(0.95, 0.7 + (approvals / (approvals + 10)) * 0.25)

        return {
            "value": row["typical_value"],
            "classification": row["classification"],
            "context_key": row["context_key"],
            "confidence": round(confidence, 3),
            "times_seen": row["times_seen"],
            "times_approved": row["times_approved"],
        }

    def store_template(self, pdf_hash: str, form_name: str, fields_json: str) -> None:
        """Store or update a PDF template in memory."""
        now = datetime.now(timezone.utc).isoformat()
        fields = json.loads(fields_json)
        cur = self._conn.cursor()

        existing = cur.execute(
            "SELECT id FROM template_memory WHERE pdf_hash = ?", (pdf_hash,)
        ).fetchone()

        if existing:
            cur.execute(
                """UPDATE template_memory
                   SET form_name = ?, field_count = ?, fields_json = ?,
                       times_seen = times_seen + 1, last_seen = ?
                   WHERE id = ?""",
                (form_name, len(fields), fields_json, now, existing["id"]),
            )
        else:
            cur.execute(
                """INSERT INTO template_memory
                   (pdf_hash, form_name, field_count, fields_json, times_seen, last_seen)
                   VALUES (?, ?, ?, ?, 1, ?)""",
                (pdf_hash, form_name, len(fields), fields_json, now),
            )
        self._conn.commit()

    def recall_template(self, pdf_hash: str) -> dict[str, Any] | None:
        """Recall a template by PDF hash."""
        cur = self._conn.cursor()
        row = cur.execute(
            "SELECT * FROM template_memory WHERE pdf_hash = ?", (pdf_hash,)
        ).fetchone()

        if not row:
            return None

        return {
            "form_name": row["form_name"],
            "field_count": row["field_count"],
            "fields_json": row["fields_json"],
            "times_seen": row["times_seen"],
        }

    def stats(self) -> dict[str, int]:
        """Return summary stats about the memory."""
        cur = self._conn.cursor()
        fields = cur.execute("SELECT COUNT(*) as c FROM field_memory").fetchone()["c"]
        templates = cur.execute("SELECT COUNT(*) as c FROM template_memory").fetchone()["c"]
        return {"total_fields": fields, "total_templates": templates}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd /c/Users/colli/OneDrive/Desktop/bid-command && python -m pytest tests/test_fill_universal/test_memory.py -v`
Expected: 8 passed

- [ ] **Step 5: Commit**

```bash
cd /c/Users/colli/OneDrive/Desktop/bid-command
git add engine/fill_universal/memory.py tests/test_fill_universal/test_memory.py
git commit -m "feat(pdffill): add DAVA memory — SQLite field/template storage with confidence scaling"
```

---

### Task 4: DETECT — 3-Tier Field Detection

**Files:**
- Create: `engine/fill_universal/detect.py`
- Test: `tests/test_fill_universal/test_detect.py`

- [ ] **Step 1: Write the failing test**

```python
# tests/test_fill_universal/test_detect.py
"""Tests for 3-tier field detection."""

import fitz  # PyMuPDF
import tempfile
from pathlib import Path
from engine.fill_universal.detect import detect_fields_on_page, detect_all_fields
from engine.fill_universal.models import DetectedField


def _make_pdf_with_form_field() -> Path:
    """Create a tiny PDF with one AcroForm text widget."""
    doc = fitz.open()
    page = doc.new_page(width=612, height=792)
    widget = fitz.Widget()
    widget.field_type = fitz.PDF_WIDGET_TYPE_TEXT
    widget.field_name = "offeror_name"
    widget.rect = fitz.Rect(100, 200, 400, 220)
    page.add_widget(widget)
    tmp = Path(tempfile.mktemp(suffix=".pdf"))
    doc.save(str(tmp))
    doc.close()
    return tmp


def _make_pdf_with_underscores() -> Path:
    """Create a PDF with 'Name: ________' style blanks."""
    doc = fitz.open()
    page = doc.new_page(width=612, height=792)
    page.insert_text((50, 100), "Name:", fontsize=12)
    # Draw a horizontal line (underscore blank)
    page.draw_line((120, 102), (350, 102), width=0.5)
    page.insert_text((50, 140), "Date:", fontsize=12)
    page.draw_line((120, 142), (250, 142), width=0.5)
    tmp = Path(tempfile.mktemp(suffix=".pdf"))
    doc.save(str(tmp))
    doc.close()
    return tmp


def test_tier1_acroform_detection():
    pdf = _make_pdf_with_form_field()
    doc = fitz.open(str(pdf))
    fields = detect_fields_on_page(doc, 0)
    doc.close()
    assert len(fields) >= 1
    acro = [f for f in fields if f.source == "acroform"]
    assert len(acro) == 1
    assert acro[0].widget_name == "offeror_name"


def test_tier2_structural_underscores():
    pdf = _make_pdf_with_underscores()
    doc = fitz.open(str(pdf))
    fields = detect_fields_on_page(doc, 0)
    doc.close()
    structural = [f for f in fields if f.source == "structural"]
    assert len(structural) >= 2
    labels = {f.label.lower() for f in structural}
    assert "name" in labels or any("name" in l for l in labels)


def test_detect_all_fields_returns_all_pages():
    pdf = _make_pdf_with_form_field()
    fields = detect_all_fields(pdf)
    assert len(fields) >= 1
    assert all(isinstance(f, DetectedField) for f in fields)
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /c/Users/colli/OneDrive/Desktop/bid-command && python -m pytest tests/test_fill_universal/test_detect.py -v`
Expected: FAIL — cannot import detect

- [ ] **Step 3: Write minimal implementation**

```python
# engine/fill_universal/detect.py
"""3-tier field detection: AcroForm -> structural analysis -> vision escalation."""

from __future__ import annotations

import re
from pathlib import Path

import fitz  # PyMuPDF

from engine.fill_universal.models import DetectedField


# ---------------------------------------------------------------------------
# Tier 1: AcroForm field extraction
# ---------------------------------------------------------------------------

def _detect_acroform(page: fitz.Page, page_idx: int) -> list[DetectedField]:
    """Extract native form widgets from a PDF page."""
    fields: list[DetectedField] = []
    for widget in page.widgets():
        if widget.field_type in (fitz.PDF_WIDGET_TYPE_TEXT, fitz.PDF_WIDGET_TYPE_CHECKBOX):
            field_type = "checkbox" if widget.field_type == fitz.PDF_WIDGET_TYPE_CHECKBOX else "text"
            rect = widget.rect
            fields.append(DetectedField(
                page=page_idx,
                bbox=(rect.x0, rect.y0, rect.x1, rect.y1),
                label=widget.field_name or "",
                field_type=field_type,
                source="acroform",
                widget_name=widget.field_name or "",
            ))
    return fields


# ---------------------------------------------------------------------------
# Tier 2: Structural analysis
# ---------------------------------------------------------------------------

_LABEL_RE = re.compile(r"^(.+?)\s*:\s*$")


def _detect_structural(page: fitz.Page, page_idx: int) -> list[DetectedField]:
    """Detect fillable regions from page geometry: underscores, empty cells, checkboxes."""
    fields: list[DetectedField] = []
    text_dict = page.get_text("dict", flags=fitz.TEXT_PRESERVE_WHITESPACE)
    drawings = page.get_drawings()

    # Collect horizontal lines (potential underscore blanks)
    h_lines: list[tuple[float, float, float, float]] = []
    for drawing in drawings:
        for item in drawing.get("items", []):
            if len(item) >= 3 and item[0] == "l":
                p1, p2 = item[1], item[2]
                # Horizontal line: y coords close, x spans > 30
                if abs(p1.y - p2.y) < 3.0 and abs(p1.x - p2.x) > 30:
                    x0, x1 = min(p1.x, p2.x), max(p1.x, p2.x)
                    y = (p1.y + p2.y) / 2
                    h_lines.append((x0, y, x1, y))

    # Collect text blocks for label matching
    text_blocks: list[dict] = []
    for block in text_dict.get("blocks", []):
        if block.get("type") == 0:  # text block
            for line in block.get("lines", []):
                text = "".join(span["text"] for span in line.get("spans", []))
                if text.strip():
                    bbox = line["bbox"]
                    text_blocks.append({"text": text.strip(), "bbox": bbox})

    # Match labels to nearby horizontal lines
    for line in h_lines:
        lx0, ly, lx1, _ = line
        best_label = ""
        best_dist = 999.0

        for tb in text_blocks:
            tx0, ty0, tx1, ty1 = tb["bbox"]
            # Label should be to the left of the line, on the same vertical band
            if tx1 < lx0 + 10 and abs((ty0 + ty1) / 2 - ly) < 15:
                dist = lx0 - tx1
                if 0 < dist < best_dist:
                    best_dist = dist
                    label_text = tb["text"].rstrip(": ")
                    best_label = label_text

        if best_label:
            fields.append(DetectedField(
                page=page_idx,
                bbox=(lx0, ly - 12, lx1, ly + 2),
                label=best_label,
                field_type="text",
                source="structural",
            ))

    # Detect small rectangles as checkboxes (8-16pt squares)
    for drawing in drawings:
        for item in drawing.get("items", []):
            if len(item) >= 3 and item[0] == "re":
                rect = item[1]
                w = abs(rect.width)
                h = abs(rect.height)
                if 6 < w < 20 and 6 < h < 20 and abs(w - h) < 4:
                    fields.append(DetectedField(
                        page=page_idx,
                        bbox=(rect.x0, rect.y0, rect.x1, rect.y1),
                        label="",
                        field_type="checkbox",
                        source="structural",
                    ))

    # Detect "Label:" patterns in text with whitespace after (no line, but a fill zone)
    for tb in text_blocks:
        m = _LABEL_RE.match(tb["text"])
        if m:
            label = m.group(1).strip()
            tx0, ty0, tx1, ty1 = tb["bbox"]
            # The fill zone is to the right of the colon, extending to page margin
            if tx1 < 400:  # not already at the right edge
                fields.append(DetectedField(
                    page=page_idx,
                    bbox=(tx1 + 5, ty0, min(tx1 + 250, 560), ty1),
                    label=label,
                    field_type="text",
                    source="structural",
                ))

    return fields


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------

def detect_fields_on_page(doc: fitz.Document, page_idx: int) -> list[DetectedField]:
    """Detect all fillable fields on a single page using tiered approach.

    Tier 1: AcroForm widgets (instant).
    Tier 2: Structural analysis (fast).
    Tier 3: Vision escalation (not implemented yet — placeholder for LLaVA/Claude Vision).
    """
    page = doc[page_idx]

    # Tier 1
    acro_fields = _detect_acroform(page, page_idx)
    if acro_fields:
        return acro_fields

    # Tier 2
    structural_fields = _detect_structural(page, page_idx)

    # Tier 3 would trigger here if len(structural_fields) < 3 and page has text
    # TODO: Vision escalation via Ollama LLaVA or Claude Vision (Task 9)

    return structural_fields


def detect_all_fields(pdf_path: Path) -> list[DetectedField]:
    """Detect fillable fields across all pages of a PDF.

    Args:
        pdf_path: Path to the PDF file.

    Returns:
        List of DetectedField objects across all pages.
    """
    doc = fitz.open(str(pdf_path))
    all_fields: list[DetectedField] = []
    try:
        for page_idx in range(len(doc)):
            fields = detect_fields_on_page(doc, page_idx)
            all_fields.extend(fields)
    finally:
        doc.close()
    return all_fields
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd /c/Users/colli/OneDrive/Desktop/bid-command && python -m pytest tests/test_fill_universal/test_detect.py -v`
Expected: 3 passed

- [ ] **Step 5: Commit**

```bash
cd /c/Users/colli/OneDrive/Desktop/bid-command
git add engine/fill_universal/detect.py tests/test_fill_universal/test_detect.py
git commit -m "feat(pdffill): add 3-tier field detection — AcroForm + structural analysis"
```

---

### Task 5: CLASSIFY — Rule-Based + Memory Classification

**Files:**
- Create: `engine/fill_universal/classify.py`
- Test: `tests/test_fill_universal/test_classify.py`

- [ ] **Step 1: Write the failing test**

```python
# tests/test_fill_universal/test_classify.py
"""Tests for field classification."""

from engine.fill_universal.classify import classify_field, classify_fields
from engine.fill_universal.models import DetectedField, ClassifiedField


def _field(label: str, field_type: str = "text") -> DetectedField:
    return DetectedField(page=0, bbox=(0, 0, 100, 20), label=label, field_type=field_type)


def test_classify_name_field():
    result = classify_field(_field("Offeror Name"))
    assert result.classification == "identity.name"
    assert result.confidence >= 0.9


def test_classify_cage_field():
    result = classify_field(_field("CAGE Code"))
    assert result.classification == "identity.code"


def test_classify_date_field():
    result = classify_field(_field("Date"))
    assert result.classification == "temporal.date"


def test_classify_signature_field():
    result = classify_field(_field("Signature", field_type="signature"))
    assert result.classification == "signature"


def test_classify_price_field():
    result = classify_field(_field("Unit Price"))
    assert result.classification == "currency"


def test_classify_checkbox():
    result = classify_field(_field("", field_type="checkbox"))
    assert result.classification == "checkbox"


def test_classify_unknown_field():
    result = classify_field(_field("Zygomorphic Coefficient"))
    assert result.classification == "unknown"
    assert result.confidence < 0.5


def test_classify_fields_batch():
    fields = [_field("Name"), _field("Date"), _field("Phone")]
    results = classify_fields(fields)
    assert len(results) == 3
    assert all(isinstance(r, ClassifiedField) for r in results)
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /c/Users/colli/OneDrive/Desktop/bid-command && python -m pytest tests/test_fill_universal/test_classify.py -v`
Expected: FAIL — cannot import classify

- [ ] **Step 3: Write minimal implementation**

```python
# engine/fill_universal/classify.py
"""Rule-based + DAVA memory field classification."""

from __future__ import annotations

import re
from engine.fill_universal.models import DetectedField, ClassifiedField
from engine.fill_universal.memory import FieldMemory


# Classification rules: (pattern, classification, confidence)
_RULES: list[tuple[re.Pattern, str, float]] = [
    (re.compile(r"\b(name|offeror|contractor|company|vendor|firm)\b", re.I), "identity.name", 0.95),
    (re.compile(r"\b(cage|uei|duns|ein|tin|tax.?id)\b", re.I), "identity.code", 0.95),
    (re.compile(r"\b(address|street|city|state|zip|postal)\b", re.I), "identity.address", 0.90),
    (re.compile(r"\b(phone|tel|fax|mobile|cell)\b", re.I), "identity.phone", 0.90),
    (re.compile(r"\b(email|e-mail)\b", re.I), "identity.email", 0.95),
    (re.compile(r"\b(date|dated)\b", re.I), "temporal.date", 0.90),
    (re.compile(r"\b(signature|sign|/s/)\b", re.I), "signature", 0.90),
    (re.compile(r"\b(price|amount|total|cost|\$|dollar)\b", re.I), "currency", 0.90),
    (re.compile(r"\b(quantity|qty|number of|count)\b", re.I), "numeric", 0.85),
    (re.compile(r"\b(describe|explain|narrative|justif|experience)\b", re.I), "essay", 0.80),
]


def classify_field(
    field: DetectedField,
    memory: FieldMemory | None = None,
) -> ClassifiedField:
    """Classify a single detected field.

    Tries in order:
    1. Field type override (checkbox -> checkbox, signature -> signature)
    2. DAVA memory lookup
    3. Rule-based label matching
    4. Unknown fallback
    """
    # Checkbox/signature types are self-classifying
    if field.field_type == "checkbox":
        return ClassifiedField.from_detected(field, "checkbox", 0.95)
    if field.field_type == "signature":
        return ClassifiedField.from_detected(field, "signature", 0.90)

    # DAVA memory lookup
    if memory is not None:
        hit = memory.recall(field.label)
        if hit is not None:
            return ClassifiedField.from_detected(field, hit["classification"], hit["confidence"])

    # Rule-based matching
    label = field.label + " " + field.widget_name
    for pattern, classification, confidence in _RULES:
        if pattern.search(label):
            return ClassifiedField.from_detected(field, classification, confidence)

    # Unknown
    return ClassifiedField.from_detected(field, "unknown", 0.3)


def classify_fields(
    fields: list[DetectedField],
    memory: FieldMemory | None = None,
) -> list[ClassifiedField]:
    """Classify a batch of detected fields."""
    return [classify_field(f, memory) for f in fields]
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd /c/Users/colli/OneDrive/Desktop/bid-command && python -m pytest tests/test_fill_universal/test_classify.py -v`
Expected: 8 passed

- [ ] **Step 5: Commit**

```bash
cd /c/Users/colli/OneDrive/Desktop/bid-command
git add engine/fill_universal/classify.py tests/test_fill_universal/test_classify.py
git commit -m "feat(pdffill): add field classifier — rule-based + DAVA memory lookup"
```

---

### Task 6: FILL — 4-Level Intelligence Stack

**Files:**
- Create: `engine/fill_universal/fill.py`
- Test: `tests/test_fill_universal/test_fill.py`

- [ ] **Step 1: Write the failing test**

```python
# tests/test_fill_universal/test_fill.py
"""Tests for the 4-level fill intelligence stack."""

import tempfile
from pathlib import Path
from engine.fill_universal.fill import fill_field, fill_fields
from engine.fill_universal.models import DetectedField, ClassifiedField, FilledField
from engine.fill_universal.memory import FieldMemory


def _classified(label: str, classification: str, confidence: float = 0.9) -> ClassifiedField:
    det = DetectedField(page=0, bbox=(0, 0, 100, 20), label=label)
    return ClassifiedField.from_detected(det, classification, confidence)


def test_level1_context_lookup():
    ctx = {"identity": {"name": "Hoags Inc."}}
    field = _classified("Offeror Name", "identity.name")
    result = fill_field(field, ctx)
    assert result.value == "Hoags Inc."
    assert result.source_level == "context"
    assert result.confidence == 1.0


def test_level2_memory_recall():
    mem = FieldMemory(Path(tempfile.mktemp(suffix=".db")))
    mem.store("offeror phone", "identity.phone", "(458) 239-3215", "identity.phone", "x.pdf", True)
    field = _classified("Offeror Phone", "identity.phone")
    result = fill_field(field, {}, memory=mem)
    assert result.value == "(458) 239-3215"
    assert result.source_level == "dava_memory"


def test_context_beats_memory():
    """Context (Level 1) should take priority over memory (Level 2)."""
    mem = FieldMemory(Path(tempfile.mktemp(suffix=".db")))
    mem.store("offeror name", "identity.name", "Old Name", "identity.name", "x.pdf", True)
    ctx = {"identity": {"name": "New Name"}}
    field = _classified("Offeror Name", "identity.name")
    result = fill_field(field, ctx, memory=mem)
    assert result.value == "New Name"
    assert result.source_level == "context"


def test_unfillable_field_gets_empty():
    field = _classified("Zygomorphic Coefficient", "unknown", confidence=0.3)
    result = fill_field(field, {})
    assert result.value == ""
    assert result.confidence < 0.5


def test_fill_fields_batch():
    ctx = {"identity": {"name": "Hoags Inc."}, "bid": {"date": "04/08/2026"}}
    fields = [
        _classified("Offeror Name", "identity.name"),
        _classified("Date", "temporal.date"),
    ]
    results = fill_fields(fields, ctx)
    assert len(results) == 2
    assert all(isinstance(r, FilledField) for r in results)
    names = {r.label: r.value for r in results}
    assert names["Offeror Name"] == "Hoags Inc."


def test_date_field_uses_bid_date():
    ctx = {"bid": {"date": "04/08/2026"}}
    field = _classified("Date", "temporal.date")
    result = fill_field(field, ctx)
    assert result.value == "04/08/2026"
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /c/Users/colli/OneDrive/Desktop/bid-command && python -m pytest tests/test_fill_universal/test_fill.py -v`
Expected: FAIL — cannot import fill

- [ ] **Step 3: Write minimal implementation**

```python
# engine/fill_universal/fill.py
"""4-level fill intelligence stack: context -> DAVA memory -> DAVA reasoning -> Claude."""

from __future__ import annotations

from engine.fill_universal.models import ClassifiedField, FilledField
from engine.fill_universal.memory import FieldMemory
from engine.fill_universal.context import resolve_key


# Mapping from classification to context key paths (Level 1 lookup table)
_CLASSIFICATION_TO_KEYS: dict[str, list[str]] = {
    "identity.name": ["identity.name", "company.name"],
    "identity.code": ["identity.cage", "identity.uei", "identity.ein"],
    "identity.address": ["identity.address"],
    "identity.phone": ["identity.phone"],
    "identity.email": ["identity.email"],
    "signature": ["identity.signer"],
    "temporal.date": ["bid.date", "date"],
}


def _level1_context(field: ClassifiedField, ctx: dict) -> tuple[str, float] | None:
    """Level 1: Direct context data lookup. Confidence 1.0."""
    key_paths = _CLASSIFICATION_TO_KEYS.get(field.classification, [])
    for key_path in key_paths:
        value = resolve_key(ctx, key_path)
        if value is not None:
            return (str(value), 1.0)

    # Try the classification itself as a key path
    value = resolve_key(ctx, field.classification)
    if value is not None:
        return (str(value), 1.0)

    return None


def _level2_memory(field: ClassifiedField, memory: FieldMemory | None) -> tuple[str, float] | None:
    """Level 2: DAVA memory recall. Confidence 0.7-0.95."""
    if memory is None:
        return None
    hit = memory.recall(field.label)
    if hit is not None:
        return (hit["value"], hit["confidence"])
    return None


def _level3_dava_reason(field: ClassifiedField, ctx: dict) -> tuple[str, float] | None:
    """Level 3: DAVA local reasoning via Ollama. Not yet implemented."""
    # Placeholder — will call dava-nexus:latest via Ollama HTTP API
    # Returns None to fall through to Level 4 or empty
    return None


def _level4_claude(field: ClassifiedField, ctx: dict, full_text: str = "") -> tuple[str, float] | None:
    """Level 4: Claude API escalation. Not yet implemented."""
    # Placeholder — will call Anthropic API for complex reasoning
    return None


def fill_field(
    field: ClassifiedField,
    ctx: dict,
    memory: FieldMemory | None = None,
    full_text: str = "",
    offline: bool = False,
) -> FilledField:
    """Fill a single classified field using the 4-level intelligence stack.

    Levels tried in order:
    1. Context data lookup (deterministic)
    2. DAVA memory recall (learned patterns)
    3. DAVA local reasoning (Ollama)
    4. Claude API escalation (skipped if offline=True)
    """
    # Special handling for signatures
    if field.classification == "signature":
        signer = resolve_key(ctx, "identity.signer")
        if signer:
            return FilledField.from_classified(field, f"/s/ {signer}", "context", 1.0)

    # Level 1: Context
    result = _level1_context(field, ctx)
    if result:
        return FilledField.from_classified(field, result[0], "context", result[1])

    # Level 2: DAVA memory
    result = _level2_memory(field, memory)
    if result:
        return FilledField.from_classified(field, result[0], "dava_memory", result[1])

    # Level 3: DAVA reasoning (local)
    result = _level3_dava_reason(field, ctx)
    if result:
        return FilledField.from_classified(field, result[0], "dava_reason", result[1])

    # Level 4: Claude (skip if offline)
    if not offline:
        result = _level4_claude(field, ctx, full_text)
        if result:
            return FilledField.from_classified(field, result[0], "claude", result[1])

    # No level could fill — return empty with low confidence
    return FilledField.from_classified(field, "", "none", 0.1)


def fill_fields(
    fields: list[ClassifiedField],
    ctx: dict,
    memory: FieldMemory | None = None,
    full_text: str = "",
    offline: bool = False,
) -> list[FilledField]:
    """Fill a batch of classified fields."""
    return [fill_field(f, ctx, memory, full_text, offline) for f in fields]
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd /c/Users/colli/OneDrive/Desktop/bid-command && python -m pytest tests/test_fill_universal/test_fill.py -v`
Expected: 6 passed

- [ ] **Step 5: Commit**

```bash
cd /c/Users/colli/OneDrive/Desktop/bid-command
git add engine/fill_universal/fill.py tests/test_fill_universal/test_fill.py
git commit -m "feat(pdffill): add 4-level fill stack — context, DAVA memory, reasoning, Claude placeholders"
```

---

### Task 7: RENDER — Place Values onto PDF + Confidence Overlay

**Files:**
- Create: `engine/fill_universal/render.py`
- Test: `tests/test_fill_universal/test_render.py`

- [ ] **Step 1: Write the failing test**

```python
# tests/test_fill_universal/test_render.py
"""Tests for PDF rendering — text placement and confidence overlay."""

import fitz
import json
import tempfile
from pathlib import Path
from engine.fill_universal.render import render_filled_pdf, render_confidence_overlay, write_fill_report
from engine.fill_universal.models import DetectedField, ClassifiedField, FilledField


def _filled(label: str, value: str, confidence: float, page: int = 0) -> FilledField:
    det = DetectedField(page=page, bbox=(100, 200, 300, 215), label=label)
    clf = ClassifiedField.from_detected(det, "identity.name", 0.9)
    return FilledField.from_classified(clf, value, "context", confidence)


def _make_blank_pdf() -> Path:
    doc = fitz.open()
    doc.new_page(width=612, height=792)
    tmp = Path(tempfile.mktemp(suffix=".pdf"))
    doc.save(str(tmp))
    doc.close()
    return tmp


def test_render_filled_pdf_creates_output():
    src = _make_blank_pdf()
    out_dir = Path(tempfile.mkdtemp())
    fields = [_filled("Name", "Hoags Inc.", 1.0)]
    dst = render_filled_pdf(src, fields, out_dir)
    assert dst.exists()
    # Verify text was inserted
    doc = fitz.open(str(dst))
    text = doc[0].get_text()
    doc.close()
    assert "Hoags Inc." in text


def test_render_confidence_overlay():
    src = _make_blank_pdf()
    out_dir = Path(tempfile.mkdtemp())
    fields = [
        _filled("Name", "Hoags Inc.", 0.95),
        _filled("Unknown", "???", 0.3),
    ]
    dst = render_confidence_overlay(src, fields, out_dir)
    assert dst.exists()


def test_write_fill_report():
    out_dir = Path(tempfile.mkdtemp())
    fields = [_filled("Name", "Hoags Inc.", 1.0)]
    path = write_fill_report(fields, out_dir)
    assert path.exists()
    data = json.loads(path.read_text())
    assert len(data["fields"]) == 1
    assert data["fields"][0]["value"] == "Hoags Inc."
    assert data["summary"]["green"] == 1
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /c/Users/colli/OneDrive/Desktop/bid-command && python -m pytest tests/test_fill_universal/test_render.py -v`
Expected: FAIL — cannot import render

- [ ] **Step 3: Write minimal implementation**

```python
# engine/fill_universal/render.py
"""PDF rendering — text placement, font sizing, confidence overlay, fill report."""

from __future__ import annotations

import json
from pathlib import Path

import fitz  # PyMuPDF

from engine.fill_universal.models import FilledField


# Confidence color thresholds
_GREEN = (0.0, 0.8, 0.0)   # >= 0.85
_YELLOW = (0.9, 0.7, 0.0)  # 0.5 - 0.85
_RED = (0.9, 0.0, 0.0)     # < 0.5


def _confidence_color(confidence: float) -> tuple[float, float, float]:
    if confidence >= 0.85:
        return _GREEN
    elif confidence >= 0.5:
        return _YELLOW
    return _RED


def _auto_fontsize(value: str, bbox_width: float, base_size: float = 10.0) -> float:
    """Calculate font size to fit text within bounding box width."""
    if not value:
        return base_size
    # Approximate: each char at base_size is ~6pt wide
    estimated_width = len(value) * base_size * 0.6
    if estimated_width <= bbox_width:
        return base_size
    scaled = base_size * (bbox_width / estimated_width) * 0.95
    return max(scaled, 5.0)


def render_filled_pdf(
    src_path: Path,
    fields: list[FilledField],
    output_dir: Path,
) -> Path:
    """Render filled values onto a PDF and save to output_dir/filled.pdf.

    Args:
        src_path: Path to the original (unfilled) PDF.
        fields: List of FilledField objects with values and positions.
        output_dir: Directory where filled.pdf will be saved.

    Returns:
        Path to the filled PDF.
    """
    output_dir.mkdir(parents=True, exist_ok=True)
    dst_path = output_dir / "filled.pdf"
    doc = fitz.open(str(src_path))

    for field in fields:
        if not field.value or field.page >= len(doc):
            continue

        page = doc[field.page]
        x0, y0, x1, y1 = field.bbox
        bbox_width = x1 - x0

        if field.field_type == "checkbox":
            if field.value.lower() in ("true", "yes", "x", "checked"):
                cx = (x0 + x1) / 2
                cy = (y0 + y1) / 2
                size = min(x1 - x0, y1 - y0) * 0.6
                page.insert_text((cx - size / 3, cy + size / 3), "X", fontsize=size, fontname="helv")
        else:
            fontsize = _auto_fontsize(field.value, bbox_width)
            text_y = y0 + (y1 - y0) * 0.75  # baseline at 75% of bbox height
            page.insert_text((x0, text_y), field.value, fontsize=fontsize, fontname="helv")

    doc.save(str(dst_path))
    doc.close()
    return dst_path


def render_confidence_overlay(
    src_path: Path,
    fields: list[FilledField],
    output_dir: Path,
) -> Path:
    """Render a confidence overlay PDF with colored rectangles.

    Green: confidence >= 0.85
    Yellow: 0.5 <= confidence < 0.85
    Red: confidence < 0.5

    Args:
        src_path: Path to the original PDF (used as background).
        fields: List of FilledField objects.
        output_dir: Directory where confidence_overlay.pdf will be saved.

    Returns:
        Path to the overlay PDF.
    """
    output_dir.mkdir(parents=True, exist_ok=True)
    dst_path = output_dir / "confidence_overlay.pdf"
    doc = fitz.open(str(src_path))

    for field in fields:
        if field.page >= len(doc):
            continue

        page = doc[field.page]
        x0, y0, x1, y1 = field.bbox
        color = _confidence_color(field.confidence)
        rect = fitz.Rect(x0, y0, x1, y1)
        page.draw_rect(rect, color=color, fill=color, fill_opacity=0.25, width=0.5)

        # Small confidence label
        label = f"{field.confidence:.0%}"
        page.insert_text((x1 + 2, y0 + 8), label, fontsize=5, fontname="helv", color=color)

    doc.save(str(dst_path))
    doc.close()
    return dst_path


def write_fill_report(
    fields: list[FilledField],
    output_dir: Path,
) -> Path:
    """Write a JSON fill report with every field's details and summary.

    Args:
        fields: List of FilledField objects.
        output_dir: Directory where fill_report.json will be saved.

    Returns:
        Path to the JSON report.
    """
    output_dir.mkdir(parents=True, exist_ok=True)
    report_path = output_dir / "fill_report.json"

    report = {
        "fields": [
            {
                "page": f.page,
                "label": f.label,
                "classification": f.classification,
                "value": f.value,
                "source_level": f.source_level,
                "confidence": f.confidence,
                "bbox": list(f.bbox),
            }
            for f in fields
        ],
        "summary": {
            "total": len(fields),
            "green": sum(1 for f in fields if f.confidence >= 0.85),
            "yellow": sum(1 for f in fields if 0.5 <= f.confidence < 0.85),
            "red": sum(1 for f in fields if f.confidence < 0.5),
        },
    }

    with open(report_path, "w") as fp:
        json.dump(report, fp, indent=2)

    return report_path
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd /c/Users/colli/OneDrive/Desktop/bid-command && python -m pytest tests/test_fill_universal/test_render.py -v`
Expected: 3 passed

- [ ] **Step 5: Commit**

```bash
cd /c/Users/colli/OneDrive/Desktop/bid-command
git add engine/fill_universal/render.py tests/test_fill_universal/test_render.py
git commit -m "feat(pdffill): add renderer — text placement, auto font sizing, confidence overlay, JSON report"
```

---

### Task 8: Public API + CLI — Wire It All Together

**Files:**
- Modify: `engine/fill_universal/__init__.py`
- Create: `engine/fill_universal/cli.py`
- Test: `tests/test_fill_universal/test_cli.py`
- Test: `tests/test_fill_universal/test_integration.py`

- [ ] **Step 1: Write the failing integration test**

```python
# tests/test_fill_universal/test_integration.py
"""End-to-end integration test: PDF in -> filled PDF out."""

import fitz
import json
import tempfile
from pathlib import Path
from engine.fill_universal import fill_pdf, detect_fields


def _make_form_pdf() -> Path:
    """Create a PDF with AcroForm fields for testing."""
    doc = fitz.open()
    page = doc.new_page(width=612, height=792)
    for name, y in [("offeror_name", 100), ("offeror_phone", 140), ("date", 180)]:
        w = fitz.Widget()
        w.field_type = fitz.PDF_WIDGET_TYPE_TEXT
        w.field_name = name
        w.rect = fitz.Rect(150, y, 400, y + 18)
        page.add_widget(w)
    tmp = Path(tempfile.mktemp(suffix=".pdf"))
    doc.save(str(tmp))
    doc.close()
    return tmp


def test_fill_pdf_end_to_end():
    pdf = _make_form_pdf()
    out_dir = Path(tempfile.mkdtemp())
    ctx = {
        "identity": {
            "name": "Hoags Inc.",
            "phone": "(458) 239-3215",
        },
        "bid": {"date": "04/08/2026"},
    }
    result = fill_pdf(pdf, ctx, out_dir)
    assert Path(result.filled_pdf_path).exists()
    assert Path(result.overlay_pdf_path).exists()
    assert Path(result.report_path).exists()
    assert result.total_fields >= 3
    assert result.green_count >= 2  # name and phone should be green


def test_detect_fields_standalone():
    pdf = _make_form_pdf()
    fields = detect_fields(pdf)
    assert len(fields) >= 3
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /c/Users/colli/OneDrive/Desktop/bid-command && python -m pytest tests/test_fill_universal/test_integration.py -v`
Expected: FAIL — fill_pdf not defined

- [ ] **Step 3: Write the public API**

```python
# engine/fill_universal/__init__.py
"""fill_universal — Universal PDF field detection and filling engine.

Public API:
    fill_pdf(pdf_path, context, output_dir) -> FillResult
    detect_fields(pdf_path) -> list[DetectedField]
"""

from __future__ import annotations

from pathlib import Path

from engine.fill_universal.models import DetectedField, FillResult
from engine.fill_universal.detect import detect_all_fields
from engine.fill_universal.classify import classify_fields
from engine.fill_universal.fill import fill_fields
from engine.fill_universal.render import render_filled_pdf, render_confidence_overlay, write_fill_report
from engine.fill_universal.context import load_context
from engine.fill_universal.memory import FieldMemory


# Default memory DB location
_DEFAULT_MEMORY_PATH = Path(__file__).parent.parent.parent / "data" / "dava_memory.db"


def detect_fields(pdf_path: Path) -> list[DetectedField]:
    """Detect all fillable fields in a PDF.

    Args:
        pdf_path: Path to the PDF file.

    Returns:
        List of DetectedField objects.
    """
    return detect_all_fields(Path(pdf_path))


def fill_pdf(
    pdf_path: Path,
    context: dict | Path,
    output_dir: Path,
    memory_path: Path | None = None,
    offline: bool = False,
) -> FillResult:
    """Fill a PDF using the universal detection and intelligence pipeline.

    Args:
        pdf_path: Path to the PDF to fill.
        context: Context dict or path to a JSON context file.
        output_dir: Directory for output files (filled.pdf, overlay, report).
        memory_path: Path to DAVA's memory DB. Defaults to data/dava_memory.db.
        offline: If True, skip Claude API calls (DAVA-only mode).

    Returns:
        FillResult with paths to outputs and field details.
    """
    pdf_path = Path(pdf_path)
    output_dir = Path(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    # Load context
    ctx = load_context(context)

    # Load DAVA memory
    mem_path = memory_path or _DEFAULT_MEMORY_PATH
    mem_path.parent.mkdir(parents=True, exist_ok=True)
    memory = FieldMemory(mem_path)

    # Pipeline: DETECT -> CLASSIFY -> FILL -> RENDER
    detected = detect_all_fields(pdf_path)
    classified = classify_fields(detected, memory)
    filled = fill_fields(classified, ctx, memory, offline=offline)

    # Render outputs
    filled_path = render_filled_pdf(pdf_path, filled, output_dir)
    overlay_path = render_confidence_overlay(pdf_path, filled, output_dir)
    report_path = write_fill_report(filled, output_dir)

    # Store fills in DAVA's memory for learning (not yet approved — that comes from user)
    for field in filled:
        if field.value and field.source_level != "none":
            memory.store(
                label=field.label,
                classification=field.classification,
                value=field.value,
                context_key=field.classification,
                source_pdf=pdf_path.name,
                approved=False,  # User must approve before it counts
            )

    return FillResult(
        fields=filled,
        filled_pdf_path=str(filled_path),
        overlay_pdf_path=str(overlay_path),
        report_path=str(report_path),
    )
```

- [ ] **Step 4: Write the CLI**

```python
# engine/fill_universal/cli.py
"""CLI entry point for the universal PDF filler."""

from __future__ import annotations

import json
import sys
from pathlib import Path

from engine.fill_universal import fill_pdf, detect_fields
from engine.fill_universal.memory import FieldMemory


def cmd_fill(args: list[str]) -> None:
    """Fill a PDF: python -m engine.fill_universal fill <pdf> --context <json>"""
    if len(args) < 1:
        print("Usage: fill <pdf_path> --context <context.json> [--offline]", file=sys.stderr)
        sys.exit(1)

    pdf_path = Path(args[0])
    context_path = None
    offline = False

    i = 1
    while i < len(args):
        if args[i] == "--context" and i + 1 < len(args):
            context_path = Path(args[i + 1])
            i += 2
        elif args[i] == "--offline":
            offline = True
            i += 1
        else:
            i += 1

    if context_path is None:
        print("Error: --context is required", file=sys.stderr)
        sys.exit(1)

    output_dir = pdf_path.parent / (pdf_path.stem + "_filled")
    print(f"Filling: {pdf_path.name}")
    print(f"Context: {context_path.name}")
    print(f"Output:  {output_dir}")

    result = fill_pdf(pdf_path, context_path, output_dir, offline=offline)

    print()
    print(f"Fields detected:  {result.total_fields}")
    print(f"  Green (>=85%):  {result.green_count}")
    print(f"  Yellow (50-84%): {result.yellow_count}")
    print(f"  Red (<50%):     {result.red_count}")
    print()
    print(f"Filled PDF:       {result.filled_pdf_path}")
    print(f"Overlay:          {result.overlay_pdf_path}")
    print(f"Report:           {result.report_path}")


def cmd_detect(args: list[str]) -> None:
    """Detect fields: python -m engine.fill_universal detect <pdf>"""
    if len(args) < 1:
        print("Usage: detect <pdf_path>", file=sys.stderr)
        sys.exit(1)

    pdf_path = Path(args[0])
    fields = detect_fields(pdf_path)

    print(f"Detected {len(fields)} fields in {pdf_path.name}:")
    for f in fields:
        print(f"  p{f.page} [{f.source:10}] {f.field_type:10} {f.label!r:30} bbox=({f.bbox[0]:.0f},{f.bbox[1]:.0f},{f.bbox[2]:.0f},{f.bbox[3]:.0f})")


def cmd_memory(args: list[str]) -> None:
    """Memory operations: --stats or --search <term>"""
    mem_path = Path(__file__).parent.parent.parent / "data" / "dava_memory.db"
    if not mem_path.exists():
        print("No memory DB found yet. Fill some PDFs first.")
        return

    memory = FieldMemory(mem_path)

    if "--stats" in args:
        stats = memory.stats()
        print(f"DAVA Memory Stats:")
        print(f"  Fields learned:    {stats['total_fields']}")
        print(f"  Templates cached:  {stats['total_templates']}")
    elif "--search" in args:
        idx = args.index("--search")
        if idx + 1 < len(args):
            term = args[idx + 1]
            hit = memory.recall(term)
            if hit:
                print(f"Found: {term}")
                print(f"  Value:          {hit['value']}")
                print(f"  Classification: {hit['classification']}")
                print(f"  Confidence:     {hit['confidence']:.1%}")
                print(f"  Times seen:     {hit['times_seen']}")
                print(f"  Times approved: {hit['times_approved']}")
            else:
                print(f"No memory for: {term}")
    else:
        print("Usage: memory --stats | --search <term>")


def main() -> None:
    if len(sys.argv) < 2:
        print("Usage: python -m engine.fill_universal <command> [args]")
        print("Commands: fill, detect, memory")
        sys.exit(1)

    command = sys.argv[1]
    args = sys.argv[2:]

    if command == "fill":
        cmd_fill(args)
    elif command == "detect":
        cmd_detect(args)
    elif command == "memory":
        cmd_memory(args)
    else:
        print(f"Unknown command: {command}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
```

- [ ] **Step 5: Create `__main__.py` for module execution**

```python
# engine/fill_universal/__main__.py
"""Allow running as: python -m engine.fill_universal"""
from engine.fill_universal.cli import main
main()
```

- [ ] **Step 6: Run integration tests**

Run: `cd /c/Users/colli/OneDrive/Desktop/bid-command && python -m pytest tests/test_fill_universal/test_integration.py -v`
Expected: 2 passed

- [ ] **Step 7: Run ALL tests**

Run: `cd /c/Users/colli/OneDrive/Desktop/bid-command && python -m pytest tests/test_fill_universal/ -v`
Expected: All tests pass (models + context + memory + detect + classify + fill + render + integration)

- [ ] **Step 8: Commit**

```bash
cd /c/Users/colli/OneDrive/Desktop/bid-command
git add engine/fill_universal/ tests/test_fill_universal/
git commit -m "feat(pdffill): wire up public API, CLI, and integration — full pipeline operational"
```

---

### Task 9: Vision Escalation (Tier 3 Detection) — DAVA/Claude Vision

**Files:**
- Modify: `engine/fill_universal/detect.py`
- Test: `tests/test_fill_universal/test_detect.py` (add vision tests)

This task adds the vision escalation path. Since it requires Ollama (LLaVA) or Claude Vision API, the tests mock the API calls.

- [ ] **Step 1: Write the failing test**

```python
# Add to tests/test_fill_universal/test_detect.py

from unittest.mock import patch


def _make_blank_form_pdf() -> Path:
    """A page with text but no structural fields — should trigger vision."""
    doc = fitz.open()
    page = doc.new_page(width=612, height=792)
    page.insert_text((50, 100), "Please fill out this form completely.", fontsize=12)
    page.insert_text((50, 150), "Your information will be kept confidential.", fontsize=10)
    tmp = Path(tempfile.mktemp(suffix=".pdf"))
    doc.save(str(tmp))
    doc.close()
    return tmp


def test_tier3_vision_triggers_when_few_structural():
    """Vision should be attempted when structural finds < 3 fields on a text-heavy page."""
    pdf = _make_blank_form_pdf()
    mock_fields = [
        DetectedField(page=0, bbox=(50, 200, 300, 215), label="Full Name", field_type="text", source="vision"),
    ]
    with patch("engine.fill_universal.detect._detect_vision", return_value=mock_fields) as mock_vision:
        doc = fitz.open(str(pdf))
        fields = detect_fields_on_page(doc, 0)
        doc.close()
        mock_vision.assert_called_once()
        assert any(f.source == "vision" for f in fields)
```

- [ ] **Step 2: Add vision detection function to detect.py**

Add to `engine/fill_universal/detect.py`:

```python
def _detect_vision(page: fitz.Page, page_idx: int) -> list[DetectedField]:
    """Tier 3: Vision-based field detection via LLaVA or Claude Vision.

    Renders the page to PNG and sends to a vision model to identify
    fillable regions. Falls back to empty list if no model is available.
    """
    import json as _json
    try:
        import httpx
    except ImportError:
        return []

    # Render page to PNG bytes
    pixmap = page.get_pixmap(dpi=150)
    png_bytes = pixmap.tobytes("png")

    # Try Ollama (LLaVA) first
    import base64
    b64 = base64.b64encode(png_bytes).decode()
    prompt = (
        "This is a PDF form page. Identify every blank field, checkbox, signature line, "
        "or area that needs to be filled in. For each field return a JSON object with: "
        '"label" (the field label or nearby text), "type" (text/checkbox/signature/date/currency), '
        '"bbox" [x0, y0, x1, y1] as approximate pixel coordinates. '
        "Return a JSON array of objects. Only output the JSON array, nothing else."
    )

    try:
        resp = httpx.post(
            "http://localhost:11434/api/generate",
            json={"model": "llava", "prompt": prompt, "images": [b64], "stream": False},
            timeout=30.0,
        )
        if resp.status_code == 200:
            text = resp.json().get("response", "")
            # Try to parse JSON from response
            start = text.find("[")
            end = text.rfind("]") + 1
            if start >= 0 and end > start:
                items = _json.loads(text[start:end])
                fields = []
                for item in items:
                    fields.append(DetectedField(
                        page=page_idx,
                        bbox=tuple(item.get("bbox", [0, 0, 100, 20])),
                        label=item.get("label", ""),
                        field_type=item.get("type", "text"),
                        source="vision",
                    ))
                return fields
    except Exception:
        pass  # Fall through — no vision model available

    return []
```

Update `detect_fields_on_page` to call vision when structural finds < 3 fields:

```python
def detect_fields_on_page(doc: fitz.Document, page_idx: int) -> list[DetectedField]:
    page = doc[page_idx]

    # Tier 1
    acro_fields = _detect_acroform(page, page_idx)
    if acro_fields:
        return acro_fields

    # Tier 2
    structural_fields = _detect_structural(page, page_idx)

    # Tier 3: Vision escalation if structural found < 3 fields AND page has text
    if len(structural_fields) < 3:
        page_text = page.get_text()
        if len(page_text.strip()) > 50:
            vision_fields = _detect_vision(page, page_idx)
            if vision_fields:
                return structural_fields + vision_fields

    return structural_fields
```

- [ ] **Step 3: Run test to verify it passes**

Run: `cd /c/Users/colli/OneDrive/Desktop/bid-command && python -m pytest tests/test_fill_universal/test_detect.py -v`
Expected: 4 passed (3 original + 1 new vision test)

- [ ] **Step 4: Commit**

```bash
cd /c/Users/colli/OneDrive/Desktop/bid-command
git add engine/fill_universal/detect.py tests/test_fill_universal/test_detect.py
git commit -m "feat(pdffill): add Tier 3 vision detection via Ollama LLaVA"
```

---

### Task 10: DAVA Reasoning + Claude Escalation (Fill Levels 3 & 4)

**Files:**
- Modify: `engine/fill_universal/fill.py`
- Test: `tests/test_fill_universal/test_fill.py` (add DAVA/Claude tests)

- [ ] **Step 1: Write the failing test**

```python
# Add to tests/test_fill_universal/test_fill.py

from unittest.mock import patch, MagicMock


def test_level3_dava_reasoning():
    """DAVA reasoning should fill when context and memory miss."""
    field = _classified("Relevant Experience", "essay", confidence=0.8)
    mock_response = ("10 years wildland firefighting with USFS/BLM", 0.7)
    with patch("engine.fill_universal.fill._level3_dava_reason", return_value=mock_response):
        result = fill_field(field, {})
        assert "wildland" in result.value.lower() or result.source_level == "dava_reason"


def test_level4_claude_escalation():
    """Claude should fill when DAVA can't."""
    field = _classified("Technical Approach Narrative", "essay", confidence=0.8)
    mock_response = ("Hoags Inc. will deploy a 4-person crew...", 0.85)
    with patch("engine.fill_universal.fill._level3_dava_reason", return_value=None):
        with patch("engine.fill_universal.fill._level4_claude", return_value=mock_response):
            result = fill_field(field, {})
            assert result.source_level == "claude"
            assert result.confidence == 0.85


def test_offline_skips_claude():
    """Offline mode should not call Claude."""
    field = _classified("Narrative", "essay", confidence=0.8)
    with patch("engine.fill_universal.fill._level3_dava_reason", return_value=None):
        with patch("engine.fill_universal.fill._level4_claude") as mock_claude:
            result = fill_field(field, {}, offline=True)
            mock_claude.assert_not_called()
```

- [ ] **Step 2: Implement DAVA reasoning (Level 3)**

Replace the placeholder `_level3_dava_reason` in `fill.py`:

```python
def _level3_dava_reason(field: ClassifiedField, ctx: dict) -> tuple[str, float] | None:
    """Level 3: DAVA local reasoning via Ollama."""
    try:
        import httpx
    except ImportError:
        return None

    # Build prompt with context
    ctx_summary = "\n".join(f"  {k}: {v}" for k, v in _flatten_ctx(ctx))
    prompt = (
        f"You are DAVA, filling a PDF form for Colli and Hoags Inc.\n"
        f"Field label: {field.label}\n"
        f"Field type: {field.classification}\n"
        f"Available context:\n{ctx_summary}\n\n"
        f"What value should go in this field? Respond with ONLY the value, nothing else. "
        f"If you don't know, respond with exactly: UNKNOWN"
    )

    try:
        resp = httpx.post(
            "http://localhost:11434/api/generate",
            json={"model": "dava-nexus", "prompt": prompt, "stream": False},
            timeout=15.0,
        )
        if resp.status_code == 200:
            value = resp.json().get("response", "").strip()
            if value and value.upper() != "UNKNOWN":
                return (value, 0.65)
    except Exception:
        pass
    return None


def _flatten_ctx(ctx: dict, prefix: str = "") -> list[tuple[str, str]]:
    """Flatten a nested dict into key-value pairs for prompts."""
    items = []
    for k, v in ctx.items():
        key = f"{prefix}.{k}" if prefix else k
        if isinstance(v, dict):
            items.extend(_flatten_ctx(v, key))
        else:
            items.append((key, str(v)))
    return items
```

- [ ] **Step 3: Implement Claude escalation (Level 4)**

Replace the placeholder `_level4_claude` in `fill.py`:

```python
def _level4_claude(field: ClassifiedField, ctx: dict, full_text: str = "") -> tuple[str, float] | None:
    """Level 4: Claude API escalation for complex fields."""
    try:
        import anthropic
    except ImportError:
        return None

    ctx_summary = "\n".join(f"  {k}: {v}" for k, v in _flatten_ctx(ctx))
    text_window = full_text[:2000] if full_text else "(no document text available)"

    prompt = (
        f"You are helping DAVA fill a PDF form for Hoags Inc. (federal contractor).\n"
        f"Colli Hoag is an ex-USFS/BLM wildland firefighter. Integrity over winning, always.\n"
        f"Never fabricate or exaggerate.\n\n"
        f"Field: {field.label}\n"
        f"Type: {field.classification}\n"
        f"Context:\n{ctx_summary}\n\n"
        f"Document excerpt:\n{text_window}\n\n"
        f"What value should go in this field? "
        f"If it's a text/name/code field, give the exact value. "
        f"If it's an essay field, write a professional but concise response. "
        f"If you truly cannot determine the answer, respond with: UNKNOWN\n"
        f"Also rate your confidence 0-100 on a separate last line like: CONFIDENCE: 85"
    )

    try:
        client = anthropic.Anthropic()
        message = client.messages.create(
            model="claude-haiku-4-5-20251001",
            max_tokens=500,
            messages=[{"role": "user", "content": prompt}],
        )
        text = message.content[0].text.strip()

        # Parse confidence from last line
        lines = text.strip().split("\n")
        confidence = 0.7
        value_lines = lines
        for i, line in enumerate(lines):
            if line.startswith("CONFIDENCE:"):
                try:
                    confidence = int(line.split(":")[1].strip()) / 100.0
                except ValueError:
                    pass
                value_lines = lines[:i]
                break

        value = "\n".join(value_lines).strip()
        if value and value.upper() != "UNKNOWN":
            return (value, min(confidence, 0.95))
    except Exception:
        pass
    return None
```

- [ ] **Step 4: Run tests**

Run: `cd /c/Users/colli/OneDrive/Desktop/bid-command && python -m pytest tests/test_fill_universal/test_fill.py -v`
Expected: 9 passed (6 original + 3 new)

- [ ] **Step 5: Commit**

```bash
cd /c/Users/colli/OneDrive/Desktop/bid-command
git add engine/fill_universal/fill.py tests/test_fill_universal/test_fill.py
git commit -m "feat(pdffill): implement DAVA reasoning (Level 3) + Claude escalation (Level 4)"
```

---

### Task 11: Smoke Test on Real Solicitation

**Files:** None created — this is a manual verification step.

- [ ] **Step 1: Run the engine on the Millwood solicitation**

```bash
cd /c/Users/colli/OneDrive/Desktop/bid-command
python -m engine.fill_universal detect "C:/Users/colli/Downloads/Millwood_TriLakes_Janitorial/Combined Synopsis Solicitation (CSS)-W9127S26QA030-Updated 3.pdf"
```

Verify: fields are detected on the RFQ cover page and bid sheet.

- [ ] **Step 2: Fill with context**

```bash
python -m engine.fill_universal fill "C:/Users/colli/Downloads/Millwood_TriLakes_Janitorial/Combined Synopsis Solicitation (CSS)-W9127S26QA030-Updated 3.pdf" --context config/hoags.json
```

Verify: `filled.pdf` has company info on cover page, `confidence_overlay.pdf` shows green/yellow/red fields, `fill_report.json` lists all fields.

- [ ] **Step 3: Check DAVA's memory**

```bash
python -m engine.fill_universal memory --stats
```

Verify: fields were stored (unapproved) in memory.

- [ ] **Step 4: Commit any fixes needed**

```bash
git add -A && git commit -m "fix(pdffill): adjustments from smoke test on real solicitation"
```

---

## Summary

| Task | What It Builds | Files |
|------|---------------|-------|
| 1 | Data models | models.py |
| 2 | Context loader | context.py |
| 3 | DAVA memory | memory.py |
| 4 | 3-tier detection | detect.py |
| 5 | Classification | classify.py |
| 6 | 4-level fill | fill.py |
| 7 | Renderer + overlay | render.py |
| 8 | Public API + CLI | __init__.py, cli.py |
| 9 | Vision escalation | detect.py (update) |
| 10 | DAVA reasoning + Claude | fill.py (update) |
| 11 | Smoke test on real PDF | verification |

Each task is independently testable and commitable. Tasks 1-8 produce a working end-to-end system. Tasks 9-10 add the AI intelligence layers. Task 11 validates on real data.
