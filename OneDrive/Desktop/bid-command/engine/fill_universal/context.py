"""Context file loader — reads JSON context into a nested dict."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any


def load_context(source: Path | dict) -> dict:
    """Load context from a JSON file path or a raw dict."""
    if isinstance(source, dict):
        return source
    path = Path(source)
    with open(path, "r") as f:
        return json.load(f)


def resolve_key(ctx: dict, dotted_key: str) -> Any | None:
    """Resolve a dotted key path against a nested context dict."""
    parts = dotted_key.split(".")
    current = ctx
    for part in parts:
        if not isinstance(current, dict) or part not in current:
            return None
        current = current[part]
    return current
