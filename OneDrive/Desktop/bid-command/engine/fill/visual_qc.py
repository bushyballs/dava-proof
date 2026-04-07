"""engine.fill.visual_qc — Visual quality control for filled PDFs.

Renders filled PDF pages as PNG images and verifies that expected
text strings (company name, prices) are present in the document.
"""

from __future__ import annotations

from pathlib import Path

import fitz  # PyMuPDF


def render_pages(
    pdf_path: Path,
    output_dir: Path,
    dpi: int = 150,
) -> list[Path]:
    """Render every page of a PDF as a PNG image.

    Args:
        pdf_path: Path to the PDF to render.
        output_dir: Directory where PNG files will be saved.
        dpi: Resolution for rendering (default 150).

    Returns:
        List of Paths to the generated PNG files, in page order.
    """
    output_dir = Path(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    doc = fitz.open(str(pdf_path))
    png_paths: list[Path] = []

    try:
        for page_num, page in enumerate(doc, start=1):
            pixmap = page.get_pixmap(dpi=dpi)
            png_name = f"page_{page_num:03d}.png"
            png_path = output_dir / png_name
            pixmap.save(str(png_path))
            png_paths.append(png_path)
    finally:
        doc.close()

    return png_paths


def verify_text_present(
    pdf_path: Path,
    expected_strings: list[str],
) -> dict[str, bool]:
    """Check whether each expected string appears anywhere in the PDF text.

    Extracts all text from every page and does a simple substring check
    for each expected string.

    Args:
        pdf_path: Path to the PDF to inspect.
        expected_strings: List of strings to search for.

    Returns:
        Mapping of each expected string to True (found) or False (missing).
    """
    doc = fitz.open(str(pdf_path))
    full_text = ""
    try:
        for page in doc:
            full_text += page.get_text()
    finally:
        doc.close()

    return {s: (s in full_text) for s in expected_strings}


def qc_filled_pdf(
    pdf_path: Path,
    prices: dict,
    company_name: str,
    output_dir: Path | None = None,
) -> dict:
    """Run QC on a filled PDF: verify text presence and optionally render pages.

    Builds expected strings from the company name and all price values,
    then checks each one is present in the PDF text. Optionally renders
    pages to PNG if output_dir is provided.

    Price values expected in the PDF:
    - unit_price formatted as "X.XX" (e.g. "95.00")
    - amount formatted as "X,XXX.XX" with comma separators (e.g. "1,200.00")
      or plain "X.XX" for amounts under 1000

    Args:
        pdf_path: Path to the filled PDF.
        prices: CLIN pricing dict mapping CLIN number to
                (unit_price, quantity, total_amount) tuples.
        company_name: Company name that should appear in the PDF.
        output_dir: If provided, render pages as PNGs into this directory.

    Returns:
        dict with keys:
            - "passed": bool — all checks passed
            - "total_checks": int — number of strings checked
            - "passed_checks": int — number of strings found
            - "missing": list[str] — strings not found
            - "pages_rendered": int — number of PNG pages written (0 if no output_dir)
    """
    # Build expected strings list
    expected: list[str] = [company_name]
    for _clin, price_tuple in prices.items():
        unit_price, _qty, amount = price_tuple
        expected.append(f"{unit_price:.2f}")
        # Format amount with comma separators matching Python's {:,.2f}
        expected.append(f"{amount:,.2f}")

    # Verify text presence
    results = verify_text_present(pdf_path, expected)
    missing = [s for s, found in results.items() if not found]
    passed_checks = sum(1 for found in results.values() if found)

    # Optionally render pages
    pages_rendered = 0
    if output_dir is not None:
        rendered = render_pages(pdf_path, output_dir)
        pages_rendered = len(rendered)

    return {
        "passed": len(missing) == 0,
        "total_checks": len(expected),
        "passed_checks": passed_checks,
        "missing": missing,
        "pages_rendered": pages_rendered,
    }
