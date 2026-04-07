"""engine.fill.column_detect — Smart PDF column detection.

Instead of hardcoded x-coordinates, this module detects column positions by:
1. Searching for column header text ("UNIT PRICE", "AMOUNT", "(E)", "(F)")
2. Searching for vertical rule lines that divide columns
3. Falling back to known hardcoded positions if detection fails

This eliminates the column bleed issue where prices land in the wrong column
because different SF-1449 PDFs have slightly different layouts.
"""

from __future__ import annotations

from dataclasses import dataclass

import fitz  # PyMuPDF


# ---------------------------------------------------------------------------
# Fallback positions — used when detection fails
# ---------------------------------------------------------------------------

FALLBACK_SF1449: tuple[float, float] = (458.0, 528.0)       # pages 0-1
FALLBACK_CONTINUATION: tuple[float, float] = (425.0, 500.0)  # pages 2+


# ---------------------------------------------------------------------------
# Data classes
# ---------------------------------------------------------------------------

@dataclass
class ColumnPositions:
    """Detected column X positions for UNIT PRICE and AMOUNT."""

    unit_price_x: float = 0.0   # X coordinate for UNIT PRICE column
    amount_x: float = 0.0       # X coordinate for AMOUNT column
    source: str = "fallback"    # "headers", "lines", or "fallback"


# ---------------------------------------------------------------------------
# Header-based detection
# ---------------------------------------------------------------------------

# Column header patterns for SF-1449
_SF1449_HEADERS = {
    "unit_price": ["UNIT PRICE", "Unit Price", "UNIT\nPRICE"],
    "amount": ["AMOUNT", "Amount", "EXTENDED AMOUNT"],
}

# Column header patterns for OF-336 continuation sheets
_CONTINUATION_HEADERS = {
    "unit_price": ["(E)", "(e)"],
    "amount": ["(F)", "(f)"],
}


def _search_page_for_text(page: fitz.Page, patterns: list[str]) -> float | None:
    """Search a page for any of the given text patterns.

    Returns the x0 coordinate of the first match found, or None.
    """
    for pattern in patterns:
        rects = page.search_for(pattern)
        if rects:
            return rects[0].x0
    return None


def detect_columns_from_headers(page: fitz.Page) -> ColumnPositions | None:
    """Detect column positions by finding header text on the page.

    Searches for "UNIT PRICE" / "AMOUNT" (SF-1449) and "(E)" / "(F)"
    (OF-336 continuation sheet) column headers.

    Args:
        page: PyMuPDF page to search.

    Returns:
        ColumnPositions if both columns found, None otherwise.
    """
    # Try SF-1449 headers first
    up_x = _search_page_for_text(page, _SF1449_HEADERS["unit_price"])
    amt_x = _search_page_for_text(page, _SF1449_HEADERS["amount"])

    if up_x is not None and amt_x is not None:
        return ColumnPositions(
            unit_price_x=up_x,
            amount_x=amt_x,
            source="headers",
        )

    # Try OF-336 continuation sheet headers
    up_x = _search_page_for_text(page, _CONTINUATION_HEADERS["unit_price"])
    amt_x = _search_page_for_text(page, _CONTINUATION_HEADERS["amount"])

    if up_x is not None and amt_x is not None:
        return ColumnPositions(
            unit_price_x=up_x,
            amount_x=amt_x,
            source="headers",
        )

    return None


# ---------------------------------------------------------------------------
# Line-based detection
# ---------------------------------------------------------------------------

def detect_columns_from_lines(page: fitz.Page) -> ColumnPositions | None:
    """Detect column positions from vertical rule lines on the page.

    Looks for vertical lines in the right half of the page (x > 300)
    that span the CLIN area (y between 400 and 700 for SF-1449).
    Assumes the last 3 vertical lines define:
        line[-3] = left edge of UNIT PRICE
        line[-2] = between UNIT PRICE and AMOUNT
        line[-1] = right edge of AMOUNT

    Args:
        page: PyMuPDF page to search.

    Returns:
        ColumnPositions if enough vertical lines found, None otherwise.
    """
    drawings = page.get_drawings()

    # Filter to vertical lines in the right half of the page
    vertical_xs: list[float] = []
    for drawing in drawings:
        for item in drawing.get("items", []):
            # Each item is a tuple like ("l", Point, Point) for line
            if len(item) >= 3 and item[0] == "l":
                p1, p2 = item[1], item[2]
                # Vertical line: x coords are close, y spans > 50
                if (
                    abs(p1.x - p2.x) < 2.0
                    and abs(p1.y - p2.y) > 50
                    and p1.x > 300
                ):
                    vertical_xs.append(round(p1.x, 1))

    if not vertical_xs:
        return None

    # Deduplicate and sort
    vertical_xs = sorted(set(vertical_xs))

    # Need at least 2 lines to define column boundaries
    if len(vertical_xs) < 2:
        return None

    # If 3+ lines: use [-3] and [-2] midpoints for columns
    if len(vertical_xs) >= 3:
        up_x = round((vertical_xs[-3] + vertical_xs[-2]) / 2, 1)
        amt_x = round((vertical_xs[-2] + vertical_xs[-1]) / 2, 1)
    else:
        # 2 lines: first is UNIT PRICE left edge, second is AMOUNT left edge
        up_x = vertical_xs[0] + 5
        amt_x = vertical_xs[1] + 5

    return ColumnPositions(
        unit_price_x=up_x,
        amount_x=amt_x,
        source="lines",
    )


# ---------------------------------------------------------------------------
# Top-level detection
# ---------------------------------------------------------------------------

def detect_columns(
    doc: fitz.Document,
    page_idx: int,
) -> ColumnPositions:
    """Detect UNIT PRICE and AMOUNT column positions for a given page.

    Strategy (in order):
        1. Try header text detection on the page
        2. Try vertical line detection on the page
        3. Fall back to hardcoded positions based on page type

    Args:
        doc: PyMuPDF document.
        page_idx: Zero-based page index.

    Returns:
        ColumnPositions (always returns something — fallback is guaranteed).
    """
    # Determine fallback based on page type
    if page_idx <= 1:
        fallback = FALLBACK_SF1449
    else:
        fallback = FALLBACK_CONTINUATION

    # Out-of-range page: return fallback
    if page_idx >= len(doc):
        return ColumnPositions(
            unit_price_x=fallback[0],
            amount_x=fallback[1],
            source="fallback",
        )

    page = doc[page_idx]

    # Strategy 1: Header text
    result = detect_columns_from_headers(page)
    if result is not None:
        return result

    # Strategy 2: Vertical lines
    result = detect_columns_from_lines(page)
    if result is not None:
        return result

    # Strategy 3: Fallback
    return ColumnPositions(
        unit_price_x=fallback[0],
        amount_x=fallback[1],
        source="fallback",
    )
