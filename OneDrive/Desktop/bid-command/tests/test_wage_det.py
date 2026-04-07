"""Tests for engine.parse.wage_det — TDD, written against WD parser spec."""

import pytest

from engine.parse.wage_det import WageData, parse_wage_determination


# ---------------------------------------------------------------------------
# Sample texts
# ---------------------------------------------------------------------------

SAMPLE_WD_TEXT = """
Wage Determination No.: 2015-5571
Revision No.: 27
Date Of Last Revision: 12/03/2025
State: Oregon
Area: Oregon County of Jackson

OCCUPATION CODE - TITLE                          RATE
11150 - Janitor                                  17.32
11210 - Laborer, Grounds Maintenance             18.71
11360 - Window Cleaner                           18.85

HEALTH & WELFARE: $5.55 per hour, up to 40 hours per week
VACATION: 2 weeks paid vacation after 1 year
HOLIDAYS: 11 paid holidays per year
"""

SAMPLE_FAR_TEXT = """
52.222-42 Statement of Equivalent Rates for Federal Hires (May 2014)
This Statement is for Information Only:
It is not a Wage Determination
Employee Class    Monetary Wage -- Fringe Benefits
Janitor           $18.84--$5.55
"""


# ---------------------------------------------------------------------------
# Test 1 — Janitor rate from standard WD text
# ---------------------------------------------------------------------------

class TestParseWdJanitorRate:
    def test_parse_wd_janitor_rate(self):
        """Parser must extract the janitor rate 17.32 from occupation code 11150."""
        result = parse_wage_determination(SAMPLE_WD_TEXT)
        assert result.janitor_rate == 17.32


# ---------------------------------------------------------------------------
# Test 2 — H&W fringe from standard WD text
# ---------------------------------------------------------------------------

class TestParseWdHwFringe:
    def test_parse_wd_hw_fringe(self):
        """Parser must extract the H&W fringe of $5.55/hr from the HEALTH & WELFARE line."""
        result = parse_wage_determination(SAMPLE_WD_TEXT)
        assert result.hw_fringe == 5.55


# ---------------------------------------------------------------------------
# Test 3 — loaded_floor is computed as janitor_rate + hw_fringe
# ---------------------------------------------------------------------------

class TestParseWdLoadedRate:
    def test_parse_wd_loaded_rate(self):
        """loaded_floor must equal janitor_rate + hw_fringe = 17.32 + 5.55 = 22.87."""
        result = parse_wage_determination(SAMPLE_WD_TEXT)
        assert result.loaded_floor == pytest.approx(22.87, abs=0.01)


# ---------------------------------------------------------------------------
# Test 4 — State extraction
# ---------------------------------------------------------------------------

class TestParseWdState:
    def test_parse_wd_state(self):
        """Parser must extract state 'Oregon' from the State: line."""
        result = parse_wage_determination(SAMPLE_WD_TEXT)
        assert result.state == "Oregon"


# ---------------------------------------------------------------------------
# Test 5 — FAR 52.222-42 fallback
# ---------------------------------------------------------------------------

class TestParseFarFallback:
    def test_parse_far_fallback(self):
        """When WD PDF is garbled, parse janitor_rate and hw_fringe from FAR 52.222-42 clause."""
        result = parse_wage_determination(SAMPLE_FAR_TEXT)
        assert result.janitor_rate == 18.84
        assert result.hw_fringe == 5.55


# ---------------------------------------------------------------------------
# Test 6 — Empty text returns zeroed WageData
# ---------------------------------------------------------------------------

class TestParseEmpty:
    def test_parse_empty(self):
        """Empty text must return WageData with janitor_rate=0.0 and hw_fringe=0.0."""
        result = parse_wage_determination("")
        assert result.janitor_rate == 0.0
        assert result.hw_fringe == 0.0

    def test_parse_whitespace_only(self):
        """Whitespace-only text must return zeroed WageData."""
        result = parse_wage_determination("   \n\t  \n")
        assert result.janitor_rate == 0.0
        assert result.hw_fringe == 0.0
        assert result.loaded_floor == 0.0
