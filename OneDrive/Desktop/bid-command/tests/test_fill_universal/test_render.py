"""Tests for PDF rendering."""
import fitz
import json
import tempfile
from pathlib import Path
from engine.fill_universal.render import render_filled_pdf, render_confidence_overlay, write_fill_report
from engine.fill_universal.models import DetectedField, ClassifiedField, FilledField

def _filled(label: str, value: str, confidence: float, page: int = 0) -> FilledField:
    det = DetectedField(page=page, bbox=(100, 200, 300, 215), label=label)
    clf = ClassifiedField.from_detected(det, "identity.name", 0.9)
    return FilledField.from_classified(clf, value, "context", confidence)

def _make_blank_pdf() -> Path:
    doc = fitz.open()
    doc.new_page(width=612, height=792)
    tmp = Path(tempfile.mktemp(suffix=".pdf"))
    doc.save(str(tmp))
    doc.close()
    return tmp

def test_render_filled_pdf_creates_output():
    src = _make_blank_pdf()
    out_dir = Path(tempfile.mkdtemp())
    fields = [_filled("Name", "Hoags Inc.", 1.0)]
    dst = render_filled_pdf(src, fields, out_dir)
    assert dst.exists()
    doc = fitz.open(str(dst))
    text = doc[0].get_text()
    doc.close()
    assert "Hoags Inc." in text

def test_render_confidence_overlay():
    src = _make_blank_pdf()
    out_dir = Path(tempfile.mkdtemp())
    fields = [_filled("Name", "Hoags Inc.", 0.95), _filled("Unknown", "???", 0.3)]
    dst = render_confidence_overlay(src, fields, out_dir)
    assert dst.exists()

def test_write_fill_report():
    out_dir = Path(tempfile.mkdtemp())
    fields = [_filled("Name", "Hoags Inc.", 1.0)]
    path = write_fill_report(fields, out_dir)
    assert path.exists()
    data = json.loads(path.read_text())
    assert len(data["fields"]) == 1
    assert data["fields"][0]["value"] == "Hoags Inc."
    assert data["summary"]["green"] == 1
