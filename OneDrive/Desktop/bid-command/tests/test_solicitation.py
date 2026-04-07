"""Tests for engine.parse.solicitation — TDD, written before implementation."""

import pytest

from engine.parse.solicitation import parse_solicitation, SolicitationData


# ---------------------------------------------------------------------------
# Sample solicitation text (mirrors a real SF1449 layout)
# ---------------------------------------------------------------------------

SAMPLE_SF1449_TEXT = """
SOLICITATION/CONTRACT/ORDER FOR COMMERCIAL ITEMS
SOLICITATION NUMBER 1240BF26Q0027
04/15/2026 1630 PT
LORENZO MONTOYA
541-225-6334
561720
SMALL BUSINESS
USDA-FS CSA NORTHWEST 4
ROGUE RIVER-SISKIYOU NATL FOREST
PROSPECT OR 97536
OFFEROR TO COMPLETE BLOCKS 12, 17, 23, 24, & 30
Period of Performance: 05/01/2026 to 04/30/2027
Janitorial Services for High Cascades Ranger District
Product/Service Code: S201
Questions shall be submitted via email to lorenzo.montoya@usda.gov
"""


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

@pytest.fixture
def parsed() -> SolicitationData:
    return parse_solicitation(SAMPLE_SF1449_TEXT)


# ---------------------------------------------------------------------------
# Test 1 — Solicitation number
# ---------------------------------------------------------------------------

class TestParseSolNumber:
    def test_parse_sol_number(self, parsed: SolicitationData):
        """Sol number must be extracted exactly."""
        assert parsed.sol_number == "1240BF26Q0027"


# ---------------------------------------------------------------------------
# Test 2 — NAICS code
# ---------------------------------------------------------------------------

class TestParseNaics:
    def test_parse_naics(self, parsed: SolicitationData):
        """6-digit NAICS code must be captured."""
        assert parsed.naics == "561720"


# ---------------------------------------------------------------------------
# Test 3 — Due date
# ---------------------------------------------------------------------------

class TestParseDueDate:
    def test_parse_due_date(self, parsed: SolicitationData):
        """Due date must contain the date portion MM/DD/YYYY."""
        assert "04/15/2026" in parsed.due_date


# ---------------------------------------------------------------------------
# Test 4 — CO name and email
# ---------------------------------------------------------------------------

class TestParseCoInfo:
    def test_parse_co_name(self, parsed: SolicitationData):
        """CO last name 'MONTOYA' must appear in co_name (case-insensitive check)."""
        assert "MONTOYA" in parsed.co_name.upper()

    def test_parse_co_email(self, parsed: SolicitationData):
        """CO email must be the .gov address from the text."""
        assert parsed.co_email == "lorenzo.montoya@usda.gov"


# ---------------------------------------------------------------------------
# Test 5 — Location
# ---------------------------------------------------------------------------

class TestParseLocation:
    def test_parse_state(self, parsed: SolicitationData):
        """State abbreviation must be OR."""
        assert parsed.state == "OR"

    def test_parse_city(self, parsed: SolicitationData):
        """City must contain PROSPECT."""
        assert "PROSPECT" in parsed.city.upper()


# ---------------------------------------------------------------------------
# Test 6 — Set-aside
# ---------------------------------------------------------------------------

class TestParseSetAside:
    def test_parse_set_aside(self, parsed: SolicitationData):
        """Set-aside must contain 'SMALL BUSINESS'."""
        assert "SMALL BUSINESS" in parsed.set_aside.upper()


# ---------------------------------------------------------------------------
# Additional edge-case tests
# ---------------------------------------------------------------------------

class TestParsePscCode:
    def test_parse_psc_code(self, parsed: SolicitationData):
        """PSC / Product Service Code S201 must be extracted."""
        assert parsed.psc_code == "S201"


class TestParseSubmissionEmail:
    def test_parse_submission_email(self, parsed: SolicitationData):
        """Submission email must be extracted from 'submitted via email to' context."""
        assert parsed.submission_email == "lorenzo.montoya@usda.gov"


class TestParsePhone:
    def test_parse_phone(self, parsed: SolicitationData):
        """CO phone must contain the digits from the sample."""
        assert "541" in parsed.co_phone
        assert "225" in parsed.co_phone


class TestParseAlternativeSolFormat:
    """Verify W-prefix solicitation numbers (PIEE/USACE pattern) are also captured."""

    def test_w_prefix_sol_number(self):
        text = "Solicitation W912DQ26QA045\nNAICS 561730\n"
        result = parse_solicitation(text)
        assert result.sol_number == "W912DQ26QA045"

    def test_naics_561730(self):
        text = "Solicitation W912DQ26QA045\nNAICS 561730\n"
        result = parse_solicitation(text)
        assert result.naics == "561730"


class TestParseSetAsideSdvosb:
    def test_sdvosb_detected(self):
        text = "Set-Aside: SDVOSB\nSolicitation 1240BF26Q9999\n"
        result = parse_solicitation(text)
        assert result.set_aside == "SDVOSB"


class TestParseEmptyText:
    def test_empty_text_returns_defaults(self):
        """Parsing empty text must return a SolicitationData with all empty strings."""
        result = parse_solicitation("")
        assert result == SolicitationData()
