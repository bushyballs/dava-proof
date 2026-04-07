"""engine.parse.clins — Extract Contract Line Item Numbers (CLINs) from schedule text."""

from __future__ import annotations

import re
from dataclasses import dataclass


@dataclass
class Clin:
    number: str       # "0001", "0001AA", "1002"
    description: str  # "Lamonta Offices - Janitorial Services"
    quantity: int     # 7, 12, 1, 2
    unit: str         # "MO", "EA", "JC", "JB", "LO", "LS", "HR"
    year: str         # "base", "oy1", "oy2", etc.


# ---------------------------------------------------------------------------
# Compiled patterns
# ---------------------------------------------------------------------------

# Year header: "BASE YEAR" or "OPTION YEAR 1" / "OPTION YEAR ONE", etc.
_BASE_YEAR_RE = re.compile(r"\bBASE\s+YEAR\b", re.IGNORECASE)
_OPTION_YEAR_RE = re.compile(
    r"\bOPTION\s+YEAR\s+(\d+|ONE|TWO|THREE|FOUR|FIVE)\b", re.IGNORECASE
)

_WORD_TO_NUM: dict[str, int] = {
    "one": 1, "two": 2, "three": 3, "four": 4, "five": 5,
}

# Valid unit tokens
_VALID_UNITS = {"MO", "EA", "JC", "JB", "LO", "LS", "HR"}

# CLIN line pattern:
#   - 4 digits + 0-2 uppercase letters (the CLIN number)
#   - whitespace (2+ spaces or tab)
#   - description (anything up to the quantity)
#   - integer quantity
#   - whitespace
#   - unit (one of the valid units, case-insensitive)
#   - rest of line (price columns, ignored)
#
# Example:
#   0001  Lamonta Offices - Janitorial Services  7  MO  $________  $__________
_CLIN_LINE_RE = re.compile(
    r"^(\d{4}[A-Z]{0,2})"          # group 1 — CLIN number
    r"[ \t]{2,}"                    # separator (2+ spaces or tabs)
    r"(.+?)"                        # group 2 — description (non-greedy)
    r"[ \t]{2,}"                    # separator
    r"(\d+)"                        # group 3 — quantity
    r"[ \t]+"                       # separator
    r"(MO|EA|JC|JB|LO|LS|HR)"      # group 4 — unit
    r"(?:[ \t]|$)",                 # must be followed by whitespace or end-of-line
    re.IGNORECASE,
)


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------

def parse_clins(text: str) -> list[Clin]:
    """Extract CLINs from raw schedule / price schedule text.

    Detects year-header lines ("BASE YEAR", "OPTION YEAR 1") to tag each
    CLIN with its performance period.  Returns an empty list for empty input.
    """
    if not text or not text.strip():
        return []

    clins: list[Clin] = []
    current_year = "base"  # default if no header seen yet

    for line in text.splitlines():
        stripped = line.strip()
        if not stripped:
            continue

        # Check for year header first
        if _BASE_YEAR_RE.search(stripped):
            current_year = "base"
            continue

        m_opt = _OPTION_YEAR_RE.search(stripped)
        if m_opt:
            raw = m_opt.group(1).lower()
            num = _WORD_TO_NUM.get(raw, None)
            if num is None:
                try:
                    num = int(raw)
                except ValueError:
                    num = 0
            current_year = f"oy{num}"
            continue

        # Try to match a CLIN line
        m = _CLIN_LINE_RE.match(stripped)
        if m:
            clins.append(
                Clin(
                    number=m.group(1).upper(),
                    description=m.group(2).strip(),
                    quantity=int(m.group(3)),
                    unit=m.group(4).upper(),
                    year=current_year,
                )
            )

    return clins
