"""engine.parse.wage_det — Parse SCA Wage Determination documents.

Handles two sources:
  1. Standard WD PDF text (dol.gov Wage Determination format)
  2. FAR 52.222-42 clause fallback (common when WD PDF is garbled)
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field


@dataclass
class WageData:
    wd_number: str = ""
    state: str = ""
    county: str = ""
    janitor_rate: float = 0.0
    hw_fringe: float = 0.0
    loaded_floor: float = 0.0   # janitor_rate + hw_fringe (computed in __post_init__)
    vacation_weeks: int = 0
    holidays: int = 0
    all_rates: dict = None      # {code: {"title": str, "rate": float}}

    def __post_init__(self) -> None:
        if self.all_rates is None:
            self.all_rates = {}
        self.loaded_floor = round(self.janitor_rate + self.hw_fringe, 2)


# ---------------------------------------------------------------------------
# Compiled patterns — standard WD format
# ---------------------------------------------------------------------------

_WD_NUMBER_RE = re.compile(
    r"Wage\s+Determination\s+No\.?:\s*([\w\-]+)",
    re.IGNORECASE,
)
_STATE_RE = re.compile(r"^State:\s*(.+)$", re.IGNORECASE | re.MULTILINE)
_COUNTY_RE = re.compile(r"County\s+of\s+(\w[\w\s]*)", re.IGNORECASE)

# Occupation rate line: "11150 - Janitor   17.32"
_OCCUPATION_RE = re.compile(
    r"^(\d{5})\s*-\s*(.+?)\s{2,}(\d+\.\d+)\s*$",
    re.IGNORECASE | re.MULTILINE,
)

# Janitor specifically (code 11150)
_JANITOR_CODE = "11150"

# H&W fringe: "HEALTH & WELFARE: $5.55 per hour"
_HW_RE = re.compile(
    r"HEALTH\s*&\s*WELFARE[:\s]+\$?([\d.]+)\s*per\s*hour",
    re.IGNORECASE,
)

# Vacation: "2 weeks paid vacation"
_VACATION_RE = re.compile(
    r"(\d+)\s+weeks?\s+paid\s+vacation",
    re.IGNORECASE,
)

# Holidays: "11 paid holidays per year"
_HOLIDAYS_RE = re.compile(
    r"(\d+)\s+paid\s+holidays?\s+per\s+year",
    re.IGNORECASE,
)

# ---------------------------------------------------------------------------
# FAR 52.222-42 fallback pattern
# "Janitor   $18.84--$5.55"  or  "Janitor  $18.84 -- $5.55"
# ---------------------------------------------------------------------------

_FAR_JANITOR_RE = re.compile(
    r"Janitor\s+\$?([\d.]+)\s*--\s*\$?([\d.]+)",
    re.IGNORECASE,
)


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------

def parse_wage_determination(text: str) -> WageData:
    """Parse SCA Wage Determination text and return a WageData instance.

    Falls back to FAR 52.222-42 clause parsing when the standard WD format
    yields no janitor rate (common when the WD PDF is garbled).

    Returns a zeroed WageData for empty input.
    """
    if not text or not text.strip():
        return WageData()

    # --- WD number ---
    m = _WD_NUMBER_RE.search(text)
    wd_number = m.group(1).strip() if m else ""

    # --- State ---
    m = _STATE_RE.search(text)
    state = m.group(1).strip() if m else ""

    # --- County ---
    m = _COUNTY_RE.search(text)
    county = m.group(1).strip() if m else ""

    # --- All occupation rates ---
    all_rates: dict = {}
    for m in _OCCUPATION_RE.finditer(text):
        code = m.group(1)
        title = m.group(2).strip()
        rate = float(m.group(3))
        all_rates[code] = {"title": title, "rate": rate}

    # --- Janitor rate (code 11150) ---
    janitor_rate: float = 0.0
    if _JANITOR_CODE in all_rates:
        janitor_rate = all_rates[_JANITOR_CODE]["rate"]

    # --- H&W fringe ---
    hw_fringe: float = 0.0
    m = _HW_RE.search(text)
    if m:
        hw_fringe = float(m.group(1))

    # --- FAR 52.222-42 fallback when WD parsing yields no janitor rate ---
    if janitor_rate == 0.0:
        m = _FAR_JANITOR_RE.search(text)
        if m:
            janitor_rate = float(m.group(1))
            if hw_fringe == 0.0:
                hw_fringe = float(m.group(2))

    # --- Vacation ---
    m = _VACATION_RE.search(text)
    vacation_weeks = int(m.group(1)) if m else 0

    # --- Holidays ---
    m = _HOLIDAYS_RE.search(text)
    holidays = int(m.group(1)) if m else 0

    return WageData(
        wd_number=wd_number,
        state=state,
        county=county,
        janitor_rate=janitor_rate,
        hw_fringe=hw_fringe,
        vacation_weeks=vacation_weeks,
        holidays=holidays,
        all_rates=all_rates,
    )
