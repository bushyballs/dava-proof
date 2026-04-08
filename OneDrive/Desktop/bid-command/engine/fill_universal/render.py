"""PDF rendering — text placement, font sizing, confidence overlay, fill report."""
from __future__ import annotations
import json
from pathlib import Path
import fitz
from engine.fill_universal.models import FilledField

_GREEN = (0.0, 0.8, 0.0)
_YELLOW = (0.9, 0.7, 0.0)
_RED = (0.9, 0.0, 0.0)

def _confidence_color(confidence: float) -> tuple[float, float, float]:
    if confidence >= 0.85:
        return _GREEN
    elif confidence >= 0.5:
        return _YELLOW
    return _RED

def _auto_fontsize(value: str, bbox_width: float, base_size: float = 10.0) -> float:
    if not value:
        return base_size
    estimated_width = len(value) * base_size * 0.6
    if estimated_width <= bbox_width:
        return base_size
    scaled = base_size * (bbox_width / estimated_width) * 0.95
    return max(scaled, 5.0)

def render_filled_pdf(src_path: Path, fields: list[FilledField], output_dir: Path) -> Path:
    output_dir.mkdir(parents=True, exist_ok=True)
    dst_path = output_dir / "filled.pdf"
    doc = fitz.open(str(src_path))
    for field in fields:
        if not field.value or field.page >= len(doc):
            continue
        page = doc[field.page]
        x0, y0, x1, y1 = field.bbox
        bbox_width = x1 - x0
        if field.field_type == "checkbox":
            if field.value.lower() in ("true", "yes", "x", "checked"):
                cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
                size = min(x1 - x0, y1 - y0) * 0.6
                page.insert_text((cx - size / 3, cy + size / 3), "X", fontsize=size, fontname="helv")
        else:
            fontsize = _auto_fontsize(field.value, bbox_width)
            text_y = y0 + (y1 - y0) * 0.75
            page.insert_text((x0, text_y), field.value, fontsize=fontsize, fontname="helv")
    doc.save(str(dst_path))
    doc.close()
    return dst_path

def render_confidence_overlay(src_path: Path, fields: list[FilledField], output_dir: Path) -> Path:
    output_dir.mkdir(parents=True, exist_ok=True)
    dst_path = output_dir / "confidence_overlay.pdf"
    doc = fitz.open(str(src_path))
    for field in fields:
        if field.page >= len(doc):
            continue
        page = doc[field.page]
        x0, y0, x1, y1 = field.bbox
        color = _confidence_color(field.confidence)
        rect = fitz.Rect(x0, y0, x1, y1)
        page.draw_rect(rect, color=color, fill=color, fill_opacity=0.25, width=0.5)
        page.insert_text((x1 + 2, y0 + 8), f"{field.confidence:.0%}", fontsize=5, fontname="helv", color=color)
    doc.save(str(dst_path))
    doc.close()
    return dst_path

def write_fill_report(fields: list[FilledField], output_dir: Path) -> Path:
    output_dir.mkdir(parents=True, exist_ok=True)
    report_path = output_dir / "fill_report.json"
    report = {
        "fields": [
            {"page": f.page, "label": f.label, "classification": f.classification,
             "value": f.value, "source_level": f.source_level, "confidence": f.confidence,
             "bbox": list(f.bbox)}
            for f in fields
        ],
        "summary": {
            "total": len(fields),
            "green": sum(1 for f in fields if f.confidence >= 0.85),
            "yellow": sum(1 for f in fields if 0.5 <= f.confidence < 0.85),
            "red": sum(1 for f in fields if f.confidence < 0.5),
        },
    }
    with open(report_path, "w") as fp:
        json.dump(report, fp, indent=2)
    return report_path
