"""4-level fill intelligence stack: context -> DAVA memory -> DAVA reasoning -> Claude."""
from __future__ import annotations

from engine.fill_universal.models import ClassifiedField, FilledField
from engine.fill_universal.memory import FieldMemory
from engine.fill_universal.context import resolve_key

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
    """Level 1: Direct context lookup."""
    key_paths = _CLASSIFICATION_TO_KEYS.get(field.classification, [])
    for key_path in key_paths:
        value = resolve_key(ctx, key_path)
        if value is not None:
            return (str(value), 1.0)
    value = resolve_key(ctx, field.classification)
    if value is not None:
        return (str(value), 1.0)
    return None


def _level2_memory(field: ClassifiedField, memory: FieldMemory | None) -> tuple[str, float] | None:
    """Level 2: DAVA memory recall."""
    if memory is None:
        return None
    hit = memory.recall(field.label)
    if hit is not None:
        return (hit["value"], hit["confidence"])
    return None


def _level3_dava_reason(field: ClassifiedField, ctx: dict) -> tuple[str, float] | None:
    """Level 3: DAVA reasoning (placeholder)."""
    return None  # Implemented in Task 10


def _level4_claude(field: ClassifiedField, ctx: dict, full_text: str = "") -> tuple[str, float] | None:
    """Level 4: Claude API reasoning (placeholder)."""
    return None  # Implemented in Task 10


def fill_field(
    field: ClassifiedField,
    ctx: dict,
    memory: FieldMemory | None = None,
    full_text: str = "",
    offline: bool = False,
) -> FilledField:
    """Fill a single field using the 4-level intelligence stack."""
    # Special case: signature fields get formatted with "/s/ " prefix
    if field.classification == "signature":
        signer = resolve_key(ctx, "identity.signer")
        if signer:
            return FilledField.from_classified(field, f"/s/ {signer}", "context", 1.0)

    # Level 1: Context
    result = _level1_context(field, ctx)
    if result:
        return FilledField.from_classified(field, result[0], "context", result[1])

    # Level 2: DAVA Memory
    result = _level2_memory(field, memory)
    if result:
        return FilledField.from_classified(field, result[0], "dava_memory", result[1])

    # Level 3: DAVA Reasoning
    result = _level3_dava_reason(field, ctx)
    if result:
        return FilledField.from_classified(field, result[0], "dava_reason", result[1])

    # Level 4: Claude (if online)
    if not offline:
        result = _level4_claude(field, ctx, full_text)
        if result:
            return FilledField.from_classified(field, result[0], "claude", result[1])

    # Unfillable: return empty with low confidence
    return FilledField.from_classified(field, "", "none", 0.1)


def fill_fields(
    fields: list[ClassifiedField],
    ctx: dict,
    memory: FieldMemory | None = None,
    full_text: str = "",
    offline: bool = False,
) -> list[FilledField]:
    """Fill multiple fields using the 4-level intelligence stack."""
    return [fill_field(f, ctx, memory, full_text, offline) for f in fields]
