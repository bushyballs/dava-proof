"""engine.fill.pdf_filler — SF-1449 PDF form filler.

Overlays company information, CLIN pricing, and signature blocks onto
government SF-1449 solicitation PDFs using PyMuPDF (fitz).
"""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path

import fitz  # PyMuPDF

from engine.config import CompanyInfo
from engine.fill.column_detect import detect_columns


@dataclass
class SF1449FillData:
    """Data required to fill an SF-1449 PDF form.

    Attributes:
        company: Company information (name, address, CAGE, UEI, etc.).
        prices: CLIN pricing dict mapping CLIN number to
                (unit_price, quantity, total_amount) tuples.
        date: Date string for signature block (e.g. "04/06/2026").
        font: Font name for text insertion.
        fontsize: Default font size for CLIN pricing text.
    """

    company: CompanyInfo
    prices: dict  # {"0001": (unit_price, qty, amount), ...}
    date: str = ""
    font: str = "helv"
    fontsize: float = 7.5


def _find_clin_positions(page: fitz.Page, clin: str) -> list[fitz.Rect]:
    """Find all positions of a CLIN number on a page, filtered to left column.

    Only returns matches where x0 < 120 (the CLIN number column on SF-1449
    and OF-336 continuation sheets).

    Args:
        page: PyMuPDF page to search.
        clin: CLIN number string (e.g. "0001").

    Returns:
        List of fitz.Rect bounding boxes for matching CLIN text.
    """
    rects = page.search_for(clin)
    return [r for r in rects if r.x0 < 120]


def _detect_column_positions(
    doc: fitz.Document,
    page_idx: int,
) -> tuple[float, float]:
    """Determine unit-price and amount column X positions using smart detection.

    Tries header-based and line-based detection first, then falls back
    to hardcoded positions based on page type.

    Args:
        doc: PyMuPDF document (needed for smart detection).
        page_idx: Zero-based page index in the document.

    Returns:
        Tuple of (unit_price_x, amount_x) column positions.
    """
    positions = detect_columns(doc, page_idx)
    return (positions.unit_price_x, positions.amount_x)


def fill_sf1449(
    src_path: Path,
    dst_path: Path,
    fill_data: SF1449FillData,
) -> Path:
    """Fill an SF-1449 PDF with company info and CLIN pricing.

    Opens the source PDF, overlays company information on page 1,
    places CLIN pricing on all pages where CLINs are found, adds
    signature block on page 1, and saves to dst_path.

    Uses smart column detection to find UNIT PRICE and AMOUNT column
    positions dynamically, falling back to hardcoded positions if
    detection fails.

    Args:
        src_path: Path to the original solicitation PDF.
        dst_path: Path where the filled PDF will be saved.
        fill_data: SF1449FillData with company info and prices.

    Returns:
        Path to the saved filled PDF.
    """
    doc = fitz.open(str(src_path))
    co = fill_data.company

    # --- Page 1: Company info block (Block 17a) ---
    if len(doc) > 0:
        page0 = doc[0]

        # Block 17a — Company name and address (~y=308)
        y = 308.0
        page0.insert_text(
            (30, y), co.name, fontsize=9, fontname=fill_data.font,
        )
        # Split address into lines (street, city/state/zip)
        addr_parts = [p.strip() for p in co.address.split(",", 1)]
        page0.insert_text(
            (30, y + 12), addr_parts[0] if addr_parts else co.address,
            fontsize=fill_data.fontsize, fontname=fill_data.font,
        )
        if len(addr_parts) > 1:
            page0.insert_text(
                (30, y + 22), addr_parts[1],
                fontsize=fill_data.fontsize, fontname=fill_data.font,
            )
        page0.insert_text(
            (30, y + 32), co.phone,
            fontsize=6.5, fontname=fill_data.font,
        )
        page0.insert_text(
            (30, y + 42), co.email,
            fontsize=6.5, fontname=fill_data.font,
        )

        # CAGE code and UEI (~x=195)
        page0.insert_text(
            (195, y), co.cage,
            fontsize=fill_data.fontsize, fontname=fill_data.font,
        )
        page0.insert_text(
            (195, y + 12), co.uei,
            fontsize=fill_data.fontsize, fontname=fill_data.font,
        )

        # Block 12 — Discount terms (~y=447)
        page0.insert_text(
            (30, 447), co.discount_terms,
            fontsize=fill_data.fontsize, fontname=fill_data.font,
        )

        # Block 30 — Signature (~y=735)
        page0.insert_text(
            (30, 735), f"/s/ {co.signer_name}",
            fontsize=fill_data.fontsize, fontname=fill_data.font,
        )
        page0.insert_text(
            (215, 735), f"{co.signer_name}, {co.signer_title}",
            fontsize=fill_data.fontsize, fontname=fill_data.font,
        )
        if fill_data.date:
            page0.insert_text(
                (435, 735), fill_data.date,
                fontsize=fill_data.fontsize, fontname=fill_data.font,
            )

    # --- All pages: CLIN pricing (with smart column detection) ---
    max_pages = min(len(doc), 20)
    for page_idx in range(max_pages):
        page = doc[page_idx]
        up_x, amt_x = _detect_column_positions(doc, page_idx)

        for clin, price_tuple in fill_data.prices.items():
            unit_price, _qty, amount = price_tuple
            rects = _find_clin_positions(page, clin)
            for rect in rects:
                text_y = rect.y0 + 9
                page.insert_text(
                    (up_x, text_y), f"{unit_price:.2f}",
                    fontsize=fill_data.fontsize, fontname=fill_data.font,
                )
                page.insert_text(
                    (amt_x, text_y), f"{amount:.2f}",
                    fontsize=fill_data.fontsize, fontname=fill_data.font,
                )

        # Company name on continuation sheets (pages 2+)
        if page_idx >= 2:
            page.insert_text(
                (100, 60), co.name,
                fontsize=fill_data.fontsize, fontname=fill_data.font,
            )

    doc.save(str(dst_path))
    doc.close()

    return Path(dst_path)
