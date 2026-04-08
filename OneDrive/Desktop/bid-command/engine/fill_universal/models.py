"""Data models for the fill_universal pipeline."""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path


@dataclass
class DetectedField:
    """A detected fillable region on a PDF page."""
    page: int
    bbox: tuple[float, float, float, float]  # x0, y0, x1, y1
    label: str
    field_type: str = "text"
    source: str = "structural"
    widget_name: str = ""


@dataclass
class ClassifiedField:
    """A detected field enriched with classification."""
    page: int
    bbox: tuple[float, float, float, float]
    label: str
    field_type: str
    source: str
    widget_name: str
    classification: str
    confidence: float

    @classmethod
    def from_detected(cls, det: DetectedField, classification: str, confidence: float) -> ClassifiedField:
        return cls(
            page=det.page, bbox=det.bbox, label=det.label,
            field_type=det.field_type, source=det.source, widget_name=det.widget_name,
            classification=classification, confidence=confidence,
        )


@dataclass
class FilledField:
    """A classified field with a generated value."""
    page: int
    bbox: tuple[float, float, float, float]
    label: str
    field_type: str
    source: str
    widget_name: str
    classification: str
    value: str
    source_level: str
    confidence: float

    @classmethod
    def from_classified(cls, clf: ClassifiedField, value: str, source_level: str, confidence: float) -> FilledField:
        return cls(
            page=clf.page, bbox=clf.bbox, label=clf.label,
            field_type=clf.field_type, source=clf.source, widget_name=clf.widget_name,
            classification=clf.classification,
            value=value, source_level=source_level, confidence=confidence,
        )


@dataclass
class FillResult:
    """Result of filling a PDF."""
    fields: list[FilledField] = field(default_factory=list)
    filled_pdf_path: str = ""
    overlay_pdf_path: str = ""
    report_path: str = ""

    @property
    def total_fields(self) -> int:
        return len(self.fields)

    @property
    def green_count(self) -> int:
        return sum(1 for f in self.fields if f.confidence >= 0.85)

    @property
    def yellow_count(self) -> int:
        return sum(1 for f in self.fields if 0.5 <= f.confidence < 0.85)

    @property
    def red_count(self) -> int:
        return sum(1 for f in self.fields if f.confidence < 0.5)
