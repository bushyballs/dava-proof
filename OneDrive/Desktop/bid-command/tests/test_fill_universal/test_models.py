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
