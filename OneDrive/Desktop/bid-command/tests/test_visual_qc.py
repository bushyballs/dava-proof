"""Tests for engine.fill.visual_qc — TDD, PDF visual QC."""

import pytest
import fitz  # PyMuPDF

from engine.fill.visual_qc import render_pages, verify_text_present, qc_filled_pdf


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

@pytest.fixture
def sample_pdf(tmp_path):
    path = tmp_path / "test.pdf"
    doc = fitz.open()
    page = doc.new_page()
    page.insert_text((100, 100), "Hello $95.00", fontsize=10)
    page.insert_text((100, 200), "Hoags Inc.", fontsize=10)
    doc.save(str(path))
    doc.close()
    return path


# ---------------------------------------------------------------------------
# Test 1 — render_pages creates PNG files
# ---------------------------------------------------------------------------

def test_render_pages_creates_pngs(sample_pdf, tmp_path):
    """Render a single-page PDF and verify exactly one PNG is created."""
    out_dir = tmp_path / "renders"
    pages = render_pages(sample_pdf, out_dir)

    assert len(pages) == 1
    assert pages[0].exists()
    assert pages[0].suffix == ".png"


# ---------------------------------------------------------------------------
# Test 2 — verify_text_present finds content that is in the PDF
# ---------------------------------------------------------------------------

def test_verify_text_finds_content(sample_pdf):
    """Verify strings present in the PDF are found."""
    results = verify_text_present(sample_pdf, ["Hoags", "$95.00"])

    assert results["Hoags"] is True
    assert results["$95.00"] is True


# ---------------------------------------------------------------------------
# Test 3 — verify_text_present returns False for missing content
# ---------------------------------------------------------------------------

def test_verify_text_missing_content(sample_pdf):
    """Verify a string not in the PDF returns False."""
    results = verify_text_present(sample_pdf, ["MISSING_TEXT"])

    assert results["MISSING_TEXT"] is False
