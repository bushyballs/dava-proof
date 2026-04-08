"""Rule-based + DAVA memory field classification."""
from __future__ import annotations

import re

from engine.fill_universal.models import DetectedField, ClassifiedField
from engine.fill_universal.memory import FieldMemory

# Rules ordered most-specific first — a label like "Offeror Telephone"
# must match identity.phone (keyword "telephone") before identity.name.
_RULES: list[tuple[re.Pattern, str, float]] = [
    (re.compile(r"\b(cage)\b", re.I), "identity.code", 0.95),
    (re.compile(r"\b(uei)\b", re.I), "identity.code", 0.95),
    (re.compile(r"\b(duns)\b", re.I), "identity.code", 0.95),
    (re.compile(r"\b(tin|ein|tax.?id)\b", re.I), "identity.code", 0.95),
    (re.compile(r"\b(phone|tel(?:ephone)?|fax|mobile|cell)\b", re.I), "identity.phone", 0.90),
    (re.compile(r"\b(email|e-mail)\b", re.I), "identity.email", 0.95),
    (re.compile(r"\b(address|street|city|state|zip|postal)\b", re.I), "identity.address", 0.90),
    (re.compile(r"\b(signature|sign|/s/)\b", re.I), "signature", 0.90),
    (re.compile(r"\b(point of contact|poc)\b", re.I), "identity.name", 0.90),
    (re.compile(r"\b(date|dated)\b", re.I), "temporal.date", 0.90),
    (re.compile(r"\b(price|amount|total|cost|\$|dollar)\b", re.I), "currency", 0.90),
    (re.compile(r"\b(quantity|qty|number of|count)\b", re.I), "numeric", 0.85),
    (re.compile(r"\b(describe|explain|narrative|justif|experience)\b", re.I), "essay", 0.80),
    (re.compile(r"\b(name|offeror|contractor|company|vendor|firm)\b", re.I), "identity.name", 0.95),
]


def classify_field(field: DetectedField, memory: FieldMemory | None = None) -> ClassifiedField:
    """Classify a detected field using rules and optional memory lookup.

    Args:
        field: The detected field to classify.
        memory: Optional DAVA field memory for learned patterns.

    Returns:
        A ClassifiedField with classification and confidence.
    """
    if field.field_type == "checkbox":
        return ClassifiedField.from_detected(field, "checkbox", 0.95)
    if field.field_type == "signature":
        return ClassifiedField.from_detected(field, "signature", 0.90)
    if memory is not None:
        hit = memory.recall(field.label)
        if hit is not None:
            return ClassifiedField.from_detected(field, hit["classification"], hit["confidence"])
    label = field.label + " " + field.widget_name
    for pattern, classification, confidence in _RULES:
        if pattern.search(label):
            return ClassifiedField.from_detected(field, classification, confidence)
    return ClassifiedField.from_detected(field, "unknown", 0.3)


def classify_fields(fields: list[DetectedField], memory: FieldMemory | None = None) -> list[ClassifiedField]:
    """Classify multiple detected fields.

    Args:
        fields: List of detected fields to classify.
        memory: Optional DAVA field memory for learned patterns.

    Returns:
        List of ClassifiedField objects.
    """
    return [classify_field(f, memory) for f in fields]
