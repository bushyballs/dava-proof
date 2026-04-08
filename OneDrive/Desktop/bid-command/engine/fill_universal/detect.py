"""3-tier field detection: AcroForm -> structural analysis -> vision escalation."""

from __future__ import annotations

import re
from pathlib import Path

import fitz  # PyMuPDF

from engine.fill_universal.models import DetectedField


def _detect_acroform(page: fitz.Page, page_idx: int) -> list[DetectedField]:
    """Extract native form widgets from a PDF page."""
    fields: list[DetectedField] = []
    for widget in page.widgets():
        if widget.field_type in (fitz.PDF_WIDGET_TYPE_TEXT, fitz.PDF_WIDGET_TYPE_CHECKBOX):
            field_type = "checkbox" if widget.field_type == fitz.PDF_WIDGET_TYPE_CHECKBOX else "text"
            rect = widget.rect
            fields.append(DetectedField(
                page=page_idx,
                bbox=(rect.x0, rect.y0, rect.x1, rect.y1),
                label=widget.field_name or "",
                field_type=field_type,
                source="acroform",
                widget_name=widget.field_name or "",
            ))
    return fields


_LABEL_RE = re.compile(r"^(.+?)\s*:\s*$")


def _detect_structural(page: fitz.Page, page_idx: int) -> list[DetectedField]:
    """Detect fillable regions from page geometry."""
    fields: list[DetectedField] = []
    text_dict = page.get_text("dict", flags=fitz.TEXT_PRESERVE_WHITESPACE)
    drawings = page.get_drawings()

    # Collect horizontal lines
    h_lines: list[tuple[float, float, float, float]] = []
    for drawing in drawings:
        for item in drawing.get("items", []):
            if len(item) >= 3 and item[0] == "l":
                p1, p2 = item[1], item[2]
                if abs(p1.y - p2.y) < 3.0 and abs(p1.x - p2.x) > 30:
                    x0, x1 = min(p1.x, p2.x), max(p1.x, p2.x)
                    y = (p1.y + p2.y) / 2
                    h_lines.append((x0, y, x1, y))

    # Collect text blocks
    text_blocks: list[dict] = []
    for block in text_dict.get("blocks", []):
        if block.get("type") == 0:
            for line in block.get("lines", []):
                text = "".join(span["text"] for span in line.get("spans", []))
                if text.strip():
                    text_blocks.append({"text": text.strip(), "bbox": line["bbox"]})

    # Match labels to nearby horizontal lines
    for line in h_lines:
        lx0, ly, lx1, _ = line
        best_label = ""
        best_dist = 999.0
        for tb in text_blocks:
            tx0, ty0, tx1, ty1 = tb["bbox"]
            if tx1 < lx0 + 10 and abs((ty0 + ty1) / 2 - ly) < 15:
                dist = lx0 - tx1
                if 0 < dist < best_dist:
                    best_dist = dist
                    best_label = tb["text"].rstrip(": ")
        if best_label:
            fields.append(DetectedField(
                page=page_idx, bbox=(lx0, ly - 12, lx1, ly + 2),
                label=best_label, field_type="text", source="structural",
            ))

    # Detect checkboxes (small squares)
    for drawing in drawings:
        for item in drawing.get("items", []):
            if len(item) >= 3 and item[0] == "re":
                rect = item[1]
                w, h = abs(rect.width), abs(rect.height)
                if 6 < w < 20 and 6 < h < 20 and abs(w - h) < 4:
                    fields.append(DetectedField(
                        page=page_idx, bbox=(rect.x0, rect.y0, rect.x1, rect.y1),
                        label="", field_type="checkbox", source="structural",
                    ))

    # Detect "Label:" patterns
    for tb in text_blocks:
        m = _LABEL_RE.match(tb["text"])
        if m:
            label = m.group(1).strip()
            tx0, ty0, tx1, ty1 = tb["bbox"]
            if tx1 < 400:
                fields.append(DetectedField(
                    page=page_idx, bbox=(tx1 + 5, ty0, min(tx1 + 250, 560), ty1),
                    label=label, field_type="text", source="structural",
                ))

    return fields


def _detect_vision(page: fitz.Page, page_idx: int) -> list[DetectedField]:
    """Tier 3: Vision-based detection. Placeholder for Ollama LLaVA."""
    return []


def detect_fields_on_page(doc: fitz.Document, page_idx: int) -> list[DetectedField]:
    """Detect all fillable fields on a single page using tiered approach."""
    page = doc[page_idx]
    acro_fields = _detect_acroform(page, page_idx)
    if acro_fields:
        return acro_fields
    structural_fields = _detect_structural(page, page_idx)
    if len(structural_fields) < 3:
        page_text = page.get_text()
        if len(page_text.strip()) > 50:
            vision_fields = _detect_vision(page, page_idx)
            if vision_fields:
                return structural_fields + vision_fields
    return structural_fields


def detect_all_fields(pdf_path: Path) -> list[DetectedField]:
    """Detect fillable fields across all pages of a PDF."""
    doc = fitz.open(str(pdf_path))
    all_fields: list[DetectedField] = []
    try:
        for page_idx in range(len(doc)):
            fields = detect_fields_on_page(doc, page_idx)
            all_fields.extend(fields)
    finally:
        doc.close()
    return all_fields
