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
