"""Tests for engine.fill.pdf_filler — TDD, SF-1449 PDF form filling."""

import pytest
import fitz  # PyMuPDF

from engine.config import CompanyInfo
from engine.fill.pdf_filler import SF1449FillData, fill_sf1449


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

@pytest.fixture
def company():
    return CompanyInfo(
        name="Test Corp", cage="XXXXX", uei="TESTUEI123",
        address="123 Test St, Test City, OR 97000",
        phone="(555) 555-5555", email="test@test.com",
        signer_name="Test Person", signer_title="President",
    )


@pytest.fixture
def sample_sf1449(tmp_path):
    path = tmp_path / "sf1449.pdf"
    doc = fitz.open()
    page = doc.new_page(width=612, height=792)
    page.insert_text((20, 500), "0001", fontsize=8)
    page.insert_text((100, 500), "Janitorial Services  12  MO", fontsize=8)
    doc.save(str(path))
    doc.close()
    return path


# ---------------------------------------------------------------------------
# Test 1 — fill_sf1449 creates output file
# ---------------------------------------------------------------------------

def test_fill_sf1449_creates_output(sample_sf1449, company, tmp_path):
    """Fill SF-1449 and verify output file exists with size > 0."""
    dst = tmp_path / "filled.pdf"
    fill_data = SF1449FillData(
        company=company,
        prices={"0001": (100.00, 12, 1200.00)},
        date="04/06/2026",
    )
    result = fill_sf1449(sample_sf1449, dst, fill_data)
    assert result.exists()
    assert result.stat().st_size > 0


# ---------------------------------------------------------------------------
# Test 2 — Output contains company name
# ---------------------------------------------------------------------------

def test_fill_sf1449_contains_company_name(sample_sf1449, company, tmp_path):
    """Fill SF-1449 and verify company name appears in output text."""
    dst = tmp_path / "filled.pdf"
    fill_data = SF1449FillData(
        company=company,
        prices={"0001": (100.00, 12, 1200.00)},
        date="04/06/2026",
    )
    fill_sf1449(sample_sf1449, dst, fill_data)

    doc = fitz.open(str(dst))
    full_text = ""
    for page in doc:
        full_text += page.get_text()
    doc.close()

    assert "Test Corp" in full_text


# ---------------------------------------------------------------------------
# Test 3 — Output contains CLIN price
# ---------------------------------------------------------------------------

def test_fill_sf1449_contains_price(sample_sf1449, company, tmp_path):
    """Fill with price and verify price text appears in output."""
    dst = tmp_path / "filled.pdf"
    fill_data = SF1449FillData(
        company=company,
        prices={"0001": (100.00, 12, 1200.00)},
        date="04/06/2026",
    )
    fill_sf1449(sample_sf1449, dst, fill_data)

    doc = fitz.open(str(dst))
    full_text = ""
    for page in doc:
        full_text += page.get_text()
    doc.close()

    assert "100.00" in full_text
