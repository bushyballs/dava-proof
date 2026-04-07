"""Tests for engine.ingest.doc_reader — TDD, written before implementation."""
from pathlib import Path

import pytest


# ---------------------------------------------------------------------------
# Test 1: PDF reading
# ---------------------------------------------------------------------------

class TestReadPdf:
    def test_read_pdf(self, tmp_path: Path):
        """Create a real PDF with reportlab, read it, verify content and metadata."""
        # Build a simple single-page PDF with reportlab
        from reportlab.pdfgen import canvas as rl_canvas

        pdf_path = tmp_path / "test_doc.pdf"
        c = rl_canvas.Canvas(str(pdf_path))
        c.drawString(100, 750, "Hello World")
        c.save()

        from engine.ingest.doc_reader import read_document, DocContent

        result = read_document(pdf_path)

        assert isinstance(result, DocContent)
        assert result.file_type == "pdf"
        assert result.page_count >= 1
        assert "Hello World" in result.text
        assert result.filename == "test_doc.pdf"
        assert result.file_path == pdf_path


# ---------------------------------------------------------------------------
# Test 2: XLSX reading
# ---------------------------------------------------------------------------

class TestReadXlsx:
    def test_read_xlsx(self, tmp_path: Path):
        """Create a real XLSX with openpyxl, verify content in result."""
        import openpyxl

        xlsx_path = tmp_path / "test_sheet.xlsx"
        wb = openpyxl.Workbook()
        ws = wb.active
        ws.title = "Bids"
        # Write header row
        ws.append(["Solicitation", "Description", "Value"])
        # Write a data row that must appear in output
        ws.append(["0001", "Janitorial", "50000"])
        wb.save(str(xlsx_path))

        from engine.ingest.doc_reader import read_document, DocContent

        result = read_document(xlsx_path)

        assert isinstance(result, DocContent)
        assert result.file_type == "xlsx"
        assert "0001" in result.text
        assert "Janitorial" in result.text
        assert result.filename == "test_sheet.xlsx"
        assert result.file_path == xlsx_path


# ---------------------------------------------------------------------------
# Test 3: Unknown file type
# ---------------------------------------------------------------------------

class TestReadUnknownType:
    def test_read_unknown_type(self, tmp_path: Path):
        """A .dwg file should return file_type='unknown', empty text, page_count=0."""
        dwg_path = tmp_path / "drawing.dwg"
        dwg_path.write_bytes(b"\x41\x43\x31\x30\x31\x35binary cad data")

        from engine.ingest.doc_reader import read_document, DocContent

        result = read_document(dwg_path)

        assert isinstance(result, DocContent)
        assert result.file_type == "unknown"
        assert result.text == ""
        assert result.page_count == 0
        assert result.filename == "drawing.dwg"
        assert result.file_path == dwg_path
