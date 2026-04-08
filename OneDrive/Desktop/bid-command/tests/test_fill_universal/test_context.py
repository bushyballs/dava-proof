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
