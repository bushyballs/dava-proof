"""4-level fill intelligence stack: context -> DAVA memory -> DAVA reasoning -> Claude."""
from __future__ import annotations

from engine.fill_universal.models import ClassifiedField, FilledField
from engine.fill_universal.memory import FieldMemory
from engine.fill_universal.context import resolve_key


def _flatten_ctx(ctx: dict, prefix: str = "") -> list[tuple[str, str]]:
    """Flatten nested context dict into list of (key, value) tuples."""
    items = []
    for k, v in ctx.items():
        key = f"{prefix}.{k}" if prefix else k
        if isinstance(v, dict):
            items.extend(_flatten_ctx(v, key))
        else:
            items.append((key, str(v)))
    return items

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
    """Level 3: DAVA local reasoning via Ollama."""
    try:
        import httpx
    except ImportError:
        return None
    ctx_summary = "\n".join(f"  {k}: {v}" for k, v in _flatten_ctx(ctx))
    prompt = (
        f"You are DAVA, filling a PDF form for Colli and Hoags Inc.\n"
        f"Field label: {field.label}\nField type: {field.classification}\n"
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
        f"Field: {field.label}\nType: {field.classification}\n"
        f"Context:\n{ctx_summary}\n\nDocument excerpt:\n{text_window}\n\n"
        f"What value should go in this field? If it's a text field, give the exact value. "
        f"If it's an essay field, write a professional but concise response. "
        f"If you cannot determine the answer, respond with: UNKNOWN\n"
        f"Also rate your confidence 0-100 on a separate last line like: CONFIDENCE: 85"
    )
    try:
        client = anthropic.Anthropic()
        message = client.messages.create(
            model="claude-haiku-4-5-20251001", max_tokens=500,
            messages=[{"role": "user", "content": prompt}],
        )
        text = message.content[0].text.strip()
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
