"""fill_universal — Universal PDF field detection and filling engine."""
from __future__ import annotations

from pathlib import Path

from engine.fill_universal.classify import classify_fields
from engine.fill_universal.context import load_context
from engine.fill_universal.detect import detect_all_fields
from engine.fill_universal.fill import fill_fields
from engine.fill_universal.memory import FieldMemory
from engine.fill_universal.models import DetectedField, FillResult
from engine.fill_universal.render import (
    render_confidence_overlay,
    render_filled_pdf,
    write_fill_report,
)

_DEFAULT_MEMORY_PATH = Path(__file__).parent.parent.parent / "data" / "dava_memory.db"


def detect_fields(pdf_path: Path) -> list[DetectedField]:
    """Detect all fillable fields in a PDF."""
    return detect_all_fields(Path(pdf_path))


def fill_pdf(
    pdf_path: Path,
    context: dict | Path,
    output_dir: Path,
    memory_path: Path | None = None,
    offline: bool = False,
) -> FillResult:
    """Fill a PDF with detected fields using context and memory.

    Args:
        pdf_path: Path to the input PDF file
        context: Dict or path to JSON context file with fill values
        output_dir: Directory where outputs will be written
        memory_path: Path to DAVA memory DB (default: data/dava_memory.db)
        offline: If True, skip external API calls for classification

    Returns:
        FillResult with filled PDF path, overlay, report, and field data
    """
    pdf_path = Path(pdf_path)
    output_dir = Path(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    ctx = load_context(context)
    mem_path = memory_path or _DEFAULT_MEMORY_PATH
    mem_path.parent.mkdir(parents=True, exist_ok=True)
    memory = FieldMemory(mem_path)

    detected = detect_all_fields(pdf_path)
    classified = classify_fields(detected, memory)
    filled = fill_fields(classified, ctx, memory, offline=offline)

    filled_path = render_filled_pdf(pdf_path, filled, output_dir)
    overlay_path = render_confidence_overlay(pdf_path, filled, output_dir)
    report_path = write_fill_report(filled, output_dir)

    for field in filled:
        if field.value and field.source_level != "none":
            memory.store(
                label=field.label,
                classification=field.classification,
                value=field.value,
                context_key=field.classification,
                source_pdf=pdf_path.name,
                approved=False,
            )

    return FillResult(
        fields=filled,
        filled_pdf_path=str(filled_path),
        overlay_pdf_path=str(overlay_path),
        report_path=str(report_path),
    )


__all__ = [
    "detect_fields",
    "fill_pdf",
    "detect_all_fields",
    "classify_fields",
    "fill_fields",
    "render_filled_pdf",
    "render_confidence_overlay",
    "write_fill_report",
    "load_context",
    "FieldMemory",
    "DetectedField",
    "FillResult",
]
