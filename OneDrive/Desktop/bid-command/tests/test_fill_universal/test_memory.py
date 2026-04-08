"""Tests for DAVA's field memory."""

import tempfile
from pathlib import Path
from engine.fill_universal.memory import FieldMemory


def _tmp_memory() -> FieldMemory:
    tmp = tempfile.mktemp(suffix=".db")
    return FieldMemory(Path(tmp))


def test_empty_recall_returns_none():
    mem = _tmp_memory()
    assert mem.recall("offeror name") is None


def test_store_and_recall():
    mem = _tmp_memory()
    mem.store(
        label="offeror name", classification="identity.name",
        value="Hoags Inc.", context_key="identity.name",
        source_pdf="test.pdf", approved=True,
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
    mem.store("field a", "identity.name", "val", "key", "a.pdf", True)
    hit1 = mem.recall("field a")
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
    mem.store_template(pdf_hash="abc123", form_name="SF-1449", fields_json='[{"label": "Name"}]')
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
