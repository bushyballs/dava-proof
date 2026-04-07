"""Tests for engine.parse.clins — TDD, written against CLIN parser spec."""

import pytest

from engine.parse.clins import Clin, parse_clins


# ---------------------------------------------------------------------------
# Sample schedule text (mirrors a real USFS price schedule layout)
# ---------------------------------------------------------------------------

SCHEDULE_TEXT = """
BASE YEAR 05/01/2026-12/31/2026
0001  Lamonta Offices - Janitorial Services  7  MO  $________  $__________
0002  Heli-Base - Janitorial Services  7  MO  $________  $__________
0003  Strip and Wax floors Lamonta Hotshot Buildings  1  JC  $________  $__________
0004  Carpet Cleaning Heli-Base  2  JC  $________  $__________
OPTION YEAR 1 01/01/2027- 12/31/2027
1001  Lamonta Offices - Janitorial Services  12  MO  $________  $__________
1002  Fire Ops Buildings-Janitorial Services  12  MO  $________  $__________
"""


# ---------------------------------------------------------------------------
# Test 1 — All CLINs are found
# ---------------------------------------------------------------------------

class TestParseClinsFindAll:
    def test_parse_clins_finds_all(self):
        """Parser must find at least 6 CLINs and return Clin instances."""
        result = parse_clins(SCHEDULE_TEXT)
        assert len(result) >= 6
        for item in result:
            assert isinstance(item, Clin)


# ---------------------------------------------------------------------------
# Test 2 — CLIN 0001 field values
# ---------------------------------------------------------------------------

class TestParseClinFields:
    def test_parse_clin_fields(self):
        """CLIN 0001 must have quantity=7, unit='MO', description with 'Lamonta' or 'Janitorial'."""
        result = parse_clins(SCHEDULE_TEXT)
        clin_0001 = next((c for c in result if c.number == "0001"), None)
        assert clin_0001 is not None, "CLIN 0001 not found"
        assert clin_0001.quantity == 7
        assert clin_0001.unit == "MO"
        desc_upper = clin_0001.description.upper()
        assert "LAMONTA" in desc_upper or "JANITORIAL" in desc_upper


# ---------------------------------------------------------------------------
# Test 3 — Option Year CLIN year tagging and quantity
# ---------------------------------------------------------------------------

class TestParseClinsOptionYear:
    def test_parse_clins_option_year(self):
        """CLIN 1001 must be tagged as option year 1 and have quantity=12."""
        result = parse_clins(SCHEDULE_TEXT)
        clin_1001 = next((c for c in result if c.number == "1001"), None)
        assert clin_1001 is not None, "CLIN 1001 not found"
        assert clin_1001.quantity == 12
        assert clin_1001.year == "oy1"


# ---------------------------------------------------------------------------
# Test 4 — Empty text returns empty list
# ---------------------------------------------------------------------------

class TestParseClinsEmptyText:
    def test_parse_clins_empty_text(self):
        """parse_clins('') must return an empty list."""
        assert parse_clins("") == []

    def test_parse_clins_whitespace_only(self):
        """parse_clins with only whitespace must return an empty list."""
        assert parse_clins("   \n\t  \n") == []
