"""Tests for engine.config — TDD, company configuration loader."""

import pytest

from engine.config import CompanyInfo, load_company_config


# ---------------------------------------------------------------------------
# Test 1 — Load default company config and verify all fields
# ---------------------------------------------------------------------------

def test_load_company_config():
    """Load company config from default path and verify all fields."""
    info = load_company_config()
    assert isinstance(info, CompanyInfo)
    assert info.name == "Hoags Inc."
    assert info.cage == "15XV5"
    assert info.uei == "DUHWVUXFNPV5"
    assert "Eugene" in info.address
    assert "collinhoag" in info.email
    assert info.signer_name == "Collin Hoag"
    assert info.signer_title == "President"


# ---------------------------------------------------------------------------
# Test 2 — Verify complete address
# ---------------------------------------------------------------------------

class TestCompanyAddressComplete:
    def test_address_has_street_city_state_zip(self):
        """Address must include street, city, state, and zip."""
        info = load_company_config()
        assert "4075 Aerial Way" in info.address
        assert "Eugene" in info.address
        assert "OR" in info.address
        assert "97402" in info.address


# ---------------------------------------------------------------------------
# Test 3 — Verify contact information
# ---------------------------------------------------------------------------

class TestCompanyContactInfo:
    def test_phone_format(self):
        """Phone must be present and formatted."""
        info = load_company_config()
        assert len(info.phone) > 0
        assert "(" in info.phone and ")" in info.phone

    def test_email_is_valid_format(self):
        """Email must contain @ symbol."""
        info = load_company_config()
        assert "@" in info.email
        assert "hoagsandfamily.com" in info.email


# ---------------------------------------------------------------------------
# Test 4 — Verify SAM and UEI identifiers
# ---------------------------------------------------------------------------

class TestCompanySamAndUei:
    def test_cage_code_length(self):
        """CAGE code must be 5 characters."""
        info = load_company_config()
        assert len(info.cage) == 5

    def test_uei_code_length(self):
        """UEI must be 12 characters."""
        info = load_company_config()
        assert len(info.uei) == 12

    def test_sam_api_key_present(self):
        """SAM API key must be present."""
        info = load_company_config()
        assert len(info.sam_api_key) > 0
        assert info.sam_api_key.startswith("SAM-")


# ---------------------------------------------------------------------------
# Test 5 — Verify signer information
# ---------------------------------------------------------------------------

class TestCompanySignerInfo:
    def test_signer_name_present(self):
        """Signer name must be present."""
        info = load_company_config()
        assert len(info.signer_name) > 0

    def test_signer_title_present(self):
        """Signer title must be present."""
        info = load_company_config()
        assert len(info.signer_title) > 0
        assert info.signer_title == "President"


# ---------------------------------------------------------------------------
# Test 6 — Verify discount terms default
# ---------------------------------------------------------------------------

class TestCompanyTerms:
    def test_discount_terms_default(self):
        """Discount terms must default to 'Net 30'."""
        info = load_company_config()
        assert info.discount_terms == "Net 30"
