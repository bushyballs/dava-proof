"""Tests for 3-tier field detection."""

import fitz  # PyMuPDF
import tempfile
from pathlib import Path
from engine.fill_universal.detect import detect_fields_on_page, detect_all_fields
from engine.fill_universal.models import DetectedField


def _make_pdf_with_form_field() -> Path:
    doc = fitz.open()
    page = doc.new_page(width=612, height=792)
    widget = fitz.Widget()
    widget.field_type = fitz.PDF_WIDGET_TYPE_TEXT
    widget.field_name = "offeror_name"
    widget.rect = fitz.Rect(100, 200, 400, 220)
    page.add_widget(widget)
    tmp = Path(tempfile.mktemp(suffix=".pdf"))
    doc.save(str(tmp))
    doc.close()
    return tmp


def _make_pdf_with_underscores() -> Path:
    doc = fitz.open()
    page = doc.new_page(width=612, height=792)
    page.insert_text((50, 100), "Name:", fontsize=12)
    page.draw_line((120, 102), (350, 102), width=0.5)
    page.insert_text((50, 140), "Date:", fontsize=12)
    page.draw_line((120, 142), (250, 142), width=0.5)
    tmp = Path(tempfile.mktemp(suffix=".pdf"))
    doc.save(str(tmp))
    doc.close()
    return tmp


def test_tier1_acroform_detection():
    pdf = _make_pdf_with_form_field()
    doc = fitz.open(str(pdf))
    fields = detect_fields_on_page(doc, 0)
    doc.close()
    assert len(fields) >= 1
    acro = [f for f in fields if f.source == "acroform"]
    assert len(acro) == 1
    assert acro[0].widget_name == "offeror_name"


def test_tier2_structural_underscores():
    pdf = _make_pdf_with_underscores()
    doc = fitz.open(str(pdf))
    fields = detect_fields_on_page(doc, 0)
    doc.close()
    structural = [f for f in fields if f.source == "structural"]
    assert len(structural) >= 2
    labels = {f.label.lower() for f in structural}
    assert "name" in labels or any("name" in l for l in labels)


def test_detect_all_fields_returns_all_pages():
    pdf = _make_pdf_with_form_field()
    fields = detect_all_fields(pdf)
    assert len(fields) >= 1
    assert all(isinstance(f, DetectedField) for f in fields)
