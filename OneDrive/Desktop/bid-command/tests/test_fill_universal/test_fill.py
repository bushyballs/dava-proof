"""Tests for the 4-level fill intelligence stack."""
import tempfile
from pathlib import Path

from engine.fill_universal.fill import fill_field, fill_fields
from engine.fill_universal.models import ClassifiedField, DetectedField, FilledField
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
    fields = [_classified("Offeror Name", "identity.name"), _classified("Date", "temporal.date")]
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
