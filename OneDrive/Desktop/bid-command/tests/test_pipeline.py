"""Tests for engine.pipeline — integration test, TDD, written before implementation."""

import io
import json
import zipfile
from pathlib import Path

import pytest

from engine.pipeline import process_solicitation


# ---------------------------------------------------------------------------
# Helper: build a realistic solicitation ZIP
# ---------------------------------------------------------------------------

def _make_solicitation_zip(tmp_path: Path) -> bytes:
    """Build a ZIP containing a minimal SF-1449 PDF and a CLIN schedule XLSX.

    The PDF contains embedded text that the solicitation parser can pick up:
    solicitation number, NAICS 561720, due date, CO info, SMALL BUSINESS
    set-aside, location, and a CLIN line.

    The XLSX contains a sheet with CLIN tabular data.
    """
    # --- Build the PDF ---
    import fitz  # PyMuPDF

    pdf_path = tmp_path / "sf1449.pdf"
    doc = fitz.open()
    page = doc.new_page(width=612, height=792)

    # Insert solicitation text that the parser will find
    lines = [
        "SOLICITATION/CONTRACT/ORDER FOR COMMERCIAL ITEMS",
        "SOLICITATION NUMBER 1240BF26Q0027",
        "04/15/2026 1630 PT",
        "LORENZO MONTOYA",
        "541-225-6334",
        "561720",
        "SMALL BUSINESS",
        "PROSPECT OR 97536",
        "Product/Service Code: S201",
        "Questions shall be submitted via email to lorenzo.montoya@usda.gov",
        "",
        "0001  Janitorial Services                    12  MO",
    ]
    y = 72
    for line in lines:
        page.insert_text((72, y), line, fontsize=10, fontname="helv")
        y += 14

    doc.save(str(pdf_path))
    doc.close()

    # --- Build the XLSX ---
    import openpyxl

    xlsx_path = tmp_path / "schedule.xlsx"
    wb = openpyxl.Workbook()
    ws = wb.active
    ws.title = "Price Schedule"
    ws.append(["CLIN", "Description", "Qty", "Unit"])
    ws.append(["0001", "Janitorial Services", 12, "MO"])
    ws.append(["0002", "Supply Materials", 12, "MO"])
    wb.save(str(xlsx_path))

    # --- Pack into a ZIP ---
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, "w", compression=zipfile.ZIP_DEFLATED) as zf:
        zf.write(str(pdf_path), "sf1449.pdf")
        zf.write(str(xlsx_path), "schedule.xlsx")
    return buf.getvalue()


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------

class TestProcessSolicitation:
    def test_process_solicitation_returns_result(self, tmp_path: Path):
        """Full pipeline: ZIP in -> parsed solicitation data out."""
        zip_bytes = _make_solicitation_zip(tmp_path)
        work_dir = tmp_path / "work"

        result = process_solicitation(zip_bytes, work_dir)

        # -- status --
        assert result["status"] == "parsed"

        # -- solicitation number --
        assert result["sol_number"], "sol_number must be non-empty"

        # -- NAICS --
        assert result["naics"] == "561720"

        # -- CLINs --
        assert len(result["clins"]) >= 1
        clin_numbers = [c["number"] for c in result["clins"]]
        assert "0001" in clin_numbers

        # -- documents --
        assert len(result["documents"]) >= 2
        doc_types = {d["file_type"] for d in result["documents"]}
        assert "pdf" in doc_types
        assert "xlsx" in doc_types

        # -- total_pages --
        assert result["total_pages"] >= 1

        # -- work_dir present --
        assert result["work_dir"] == str(work_dir)

    def test_process_solicitation_creates_extracted_dir(self, tmp_path: Path):
        """The pipeline must create work_dir/extracted with the extracted files."""
        zip_bytes = _make_solicitation_zip(tmp_path)
        work_dir = tmp_path / "work"

        process_solicitation(zip_bytes, work_dir)

        extract_dir = work_dir / "extracted"
        assert extract_dir.exists()
        assert any(extract_dir.iterdir()), "extracted dir should contain files"

    def test_process_solicitation_location_fields(self, tmp_path: Path):
        """State and city should be populated from the PDF text."""
        zip_bytes = _make_solicitation_zip(tmp_path)
        work_dir = tmp_path / "work"

        result = process_solicitation(zip_bytes, work_dir)

        assert result["state"] == "OR"
        assert "PROSPECT" in result["city"].upper()
