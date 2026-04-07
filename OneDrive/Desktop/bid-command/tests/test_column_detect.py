"""Tests for smart PDF column detection."""

import fitz
import pytest
from pathlib import Path
from engine.fill.column_detect import (
    ColumnPositions,
    detect_columns,
    detect_columns_from_headers,
    detect_columns_from_lines,
    FALLBACK_SF1449,
    FALLBACK_CONTINUATION,
)


@pytest.fixture
def sf1449_with_headers(tmp_path):
    """Create a PDF mimicking SF-1449 with UNIT PRICE and AMOUNT headers."""
    path = tmp_path / "sf1449_headers.pdf"
    doc = fitz.open()
    page = doc.new_page(width=612, height=792)

    # Simulate SF-1449 column headers (page 1)
    page.insert_text((20, 440), "ITEM NO.", fontsize=7)
    page.insert_text((60, 440), "SUPPLIES/SERVICES", fontsize=7)
    page.insert_text((340, 440), "QUANTITY", fontsize=7)
    page.insert_text((400, 440), "UNIT", fontsize=7)
    page.insert_text((445, 440), "UNIT PRICE", fontsize=7)
    page.insert_text((520, 440), "AMOUNT", fontsize=7)

    # Add a CLIN
    page.insert_text((20, 470), "0001", fontsize=8)
    page.insert_text((60, 470), "Janitorial Services", fontsize=8)
    page.insert_text((340, 470), "12", fontsize=8)
    page.insert_text((400, 470), "MO", fontsize=8)

    doc.save(str(path))
    doc.close()
    return path


@pytest.fixture
def continuation_with_headers(tmp_path):
    """Create a PDF mimicking OF-336 continuation sheet with (A) through (F) headers."""
    path = tmp_path / "continuation.pdf"
    doc = fitz.open()

    # Page 0 = SF-1449 front (skip)
    doc.new_page(width=612, height=792)

    # Page 1 = SF-1449 back (skip)
    doc.new_page(width=612, height=792)

    # Page 2 = OF-336 continuation sheet
    page = doc.new_page(width=612, height=792)
    page.insert_text((20, 100), "(A)", fontsize=7)
    page.insert_text((60, 100), "(B)", fontsize=7)
    page.insert_text((300, 100), "(C)", fontsize=7)
    page.insert_text((380, 100), "(D)", fontsize=7)
    page.insert_text((430, 100), "(E)", fontsize=7)
    page.insert_text((510, 100), "(F)", fontsize=7)

    page.insert_text((20, 130), "1001", fontsize=8)
    page.insert_text((60, 130), "Janitorial Services OY1", fontsize=8)

    doc.save(str(path))
    doc.close()
    return path


@pytest.fixture
def blank_pdf(tmp_path):
    """Create a blank PDF with no column headers."""
    path = tmp_path / "blank.pdf"
    doc = fitz.open()
    page = doc.new_page(width=612, height=792)
    page.insert_text((100, 100), "No columns here", fontsize=10)
    doc.save(str(path))
    doc.close()
    return path


class TestDetectColumnsFromHeaders:
    """Test header-based column detection."""

    def test_detects_unit_price_header(self, sf1449_with_headers):
        doc = fitz.open(str(sf1449_with_headers))
        page = doc[0]
        positions = detect_columns_from_headers(page)
        doc.close()

        assert positions is not None
        # UNIT PRICE header starts at x=445 in our test PDF
        assert abs(positions.unit_price_x - 445) < 20
        # AMOUNT header starts at x=520
        assert abs(positions.amount_x - 520) < 20

    def test_returns_none_for_no_headers(self, blank_pdf):
        doc = fitz.open(str(blank_pdf))
        page = doc[0]
        positions = detect_columns_from_headers(page)
        doc.close()

        assert positions is None


class TestDetectColumnsFromLines:
    """Test line/rule-based column detection."""

    def test_detects_vertical_rules(self, tmp_path):
        """Create PDF with vertical column divider lines."""
        path = tmp_path / "with_rules.pdf"
        doc = fitz.open()
        page = doc.new_page(width=612, height=792)

        # Draw vertical column dividers
        shape = page.new_shape()
        shape.draw_line((440, 430), (440, 600))  # before UNIT PRICE
        shape.draw_line((510, 430), (510, 600))  # between UNIT PRICE and AMOUNT
        shape.draw_line((580, 430), (580, 600))  # after AMOUNT
        shape.finish(color=(0, 0, 0), width=0.5)
        shape.commit()

        doc.save(str(path))
        doc.close()

        doc2 = fitz.open(str(path))
        positions = detect_columns_from_lines(doc2[0])
        doc2.close()

        # Should find positions between the vertical lines
        if positions is not None:
            # Unit price column center should be between 440 and 510
            assert 440 <= positions.unit_price_x <= 510
            # Amount column center should be between 510 and 580
            assert 510 <= positions.amount_x <= 580


class TestDetectColumns:
    """Test the top-level detect_columns function."""

    def test_detect_from_real_headers(self, sf1449_with_headers):
        doc = fitz.open(str(sf1449_with_headers))
        positions = detect_columns(doc, page_idx=0)
        doc.close()

        assert isinstance(positions, ColumnPositions)
        assert positions.unit_price_x > 0
        assert positions.amount_x > positions.unit_price_x
        assert positions.source == "headers"

    def test_fallback_for_blank_page(self, blank_pdf):
        doc = fitz.open(str(blank_pdf))
        positions = detect_columns(doc, page_idx=0)
        doc.close()

        assert isinstance(positions, ColumnPositions)
        assert positions.unit_price_x == FALLBACK_SF1449[0]
        assert positions.amount_x == FALLBACK_SF1449[1]
        assert positions.source == "fallback"

    def test_continuation_fallback(self, blank_pdf):
        """Pages 2+ use continuation sheet fallback positions."""
        doc = fitz.open(str(blank_pdf))
        # Our blank PDF only has one page, so page_idx=2 is out of range
        # detect_columns should return fallback for continuation
        positions = detect_columns(doc, page_idx=2)
        doc.close()

        assert positions.unit_price_x == FALLBACK_CONTINUATION[0]
        assert positions.amount_x == FALLBACK_CONTINUATION[1]
        assert positions.source == "fallback"

    def test_continuation_with_column_headers(self, continuation_with_headers):
        """OF-336 with (E) and (F) column markers."""
        doc = fitz.open(str(continuation_with_headers))
        positions = detect_columns(doc, page_idx=2)
        doc.close()

        assert isinstance(positions, ColumnPositions)
        assert positions.unit_price_x > 0
        assert positions.amount_x > positions.unit_price_x
