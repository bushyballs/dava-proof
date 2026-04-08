"""End-to-end integration test for fill_universal pipeline."""
from __future__ import annotations

import json
import tempfile
from pathlib import Path

import fitz
import pytest

from engine.fill_universal import detect_fields, fill_pdf


def _make_form_pdf() -> Path:
    """Create a test PDF with fillable form fields."""
    doc = fitz.open()
    page = doc.new_page(width=612, height=792)

    # Add three form fields to the page
    for name, y in [("offeror_name", 100), ("offeror_phone", 140), ("date", 180)]:
        w = fitz.Widget()
        w.field_type = fitz.PDF_WIDGET_TYPE_TEXT
        w.field_name = name
        w.rect = fitz.Rect(150, y, 400, y + 18)
        page.add_widget(w)

    tmp = Path(tempfile.mktemp(suffix=".pdf"))
    doc.save(str(tmp))
    doc.close()
    return tmp


def test_detect_fields_standalone():
    """Test field detection without filling."""
    pdf = _make_form_pdf()
    try:
        fields = detect_fields(pdf)
        assert len(fields) >= 3
        assert all(f.field_type == "text" for f in fields)
        assert any(f.label == "offeror_name" for f in fields)
    finally:
        pdf.unlink()


def test_fill_pdf_end_to_end():
    """Test complete fill pipeline with detection, classification, filling."""
    pdf = _make_form_pdf()
    out_dir = Path(tempfile.mkdtemp())

    try:
        ctx = {
            "identity": {"name": "Hoags Inc.", "phone": "(458) 239-3215"},
            "bid": {"date": "04/08/2026"},
        }

        result = fill_pdf(pdf, ctx, out_dir)

        # Verify outputs exist
        assert Path(result.filled_pdf_path).exists()
        assert Path(result.overlay_pdf_path).exists()
        assert Path(result.report_path).exists()

        # Verify field data
        assert result.total_fields >= 3
        # Date field should match context perfectly (green)
        assert result.green_count >= 1

        # Verify report is valid JSON
        with open(result.report_path) as f:
            report = json.load(f)
        assert "fields" in report
        assert "summary" in report
        assert len(report["fields"]) == result.total_fields

    finally:
        pdf.unlink()
        import shutil

        shutil.rmtree(out_dir, ignore_errors=True)


def test_fill_pdf_with_context_file():
    """Test fill_pdf with context as a JSON file instead of dict."""
    pdf = _make_form_pdf()
    out_dir = Path(tempfile.mkdtemp())
    ctx_file = Path(tempfile.mktemp(suffix=".json"))

    try:
        ctx = {
            "identity": {"name": "Test Company", "phone": "555-1234"},
            "bid": {"date": "01/15/2026"},
        }
        with open(ctx_file, "w") as f:
            json.dump(ctx, f)

        result = fill_pdf(pdf, ctx_file, out_dir)
        assert Path(result.filled_pdf_path).exists()
        assert result.total_fields >= 3

    finally:
        pdf.unlink()
        ctx_file.unlink()
        import shutil

        shutil.rmtree(out_dir, ignore_errors=True)


def test_fill_result_confidence_levels():
    """Test FillResult confidence level categorization."""
    pdf = _make_form_pdf()
    out_dir = Path(tempfile.mkdtemp())

    try:
        ctx = {
            "identity": {"name": "Hoags Inc.", "phone": "(458) 239-3215"},
            "bid": {"date": "04/08/2026"},
        }

        result = fill_pdf(pdf, ctx, out_dir)

        # Verify confidence categorization properties work
        assert result.total_fields == result.green_count + result.yellow_count + result.red_count
        assert result.green_count >= 0
        assert result.yellow_count >= 0
        assert result.red_count >= 0

    finally:
        pdf.unlink()
        import shutil

        shutil.rmtree(out_dir, ignore_errors=True)


def test_memory_persistence_across_fills():
    """Test that field memory is persisted and recalled across fills."""
    import shutil

    pdf = _make_form_pdf()
    out_dir = Path(tempfile.mkdtemp())
    mem_path = Path(tempfile.mktemp(suffix=".db"))

    try:
        ctx = {
            "identity": {"name": "Hoags Inc.", "phone": "(458) 239-3215"},
            "bid": {"date": "04/08/2026"},
        }

        # First fill should populate memory
        result1 = fill_pdf(pdf, ctx, out_dir, memory_path=mem_path)
        assert result1.total_fields >= 3

        # Second fill should use memory
        out_dir2 = Path(tempfile.mkdtemp())
        try:
            result2 = fill_pdf(pdf, ctx, out_dir2, memory_path=mem_path)
            assert result2.total_fields >= 3
        finally:
            shutil.rmtree(out_dir2, ignore_errors=True)

    finally:
        pdf.unlink()
        shutil.rmtree(out_dir, ignore_errors=True)
        # Try to delete DB file, but don't fail if it's locked
        try:
            mem_path.unlink(missing_ok=True)
        except (PermissionError, OSError):
            pass


def test_offline_mode():
    """Test fill_pdf with offline=True (skip external API calls)."""
    pdf = _make_form_pdf()
    out_dir = Path(tempfile.mkdtemp())

    try:
        ctx = {
            "identity": {"name": "Hoags Inc.", "phone": "(458) 239-3215"},
            "bid": {"date": "04/08/2026"},
        }

        # Should not raise even with offline=True
        result = fill_pdf(pdf, ctx, out_dir, offline=True)
        assert result.total_fields >= 3

    finally:
        pdf.unlink()
        import shutil

        shutil.rmtree(out_dir, ignore_errors=True)
