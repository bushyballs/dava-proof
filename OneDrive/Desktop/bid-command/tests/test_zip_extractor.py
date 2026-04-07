"""Tests for engine.ingest.zip_extractor — TDD, written before implementation."""
import io
import zipfile
from pathlib import Path

import pytest

from engine.ingest.zip_extractor import extract_solicitation_zip


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _make_zip(files: dict[str, bytes]) -> bytes:
    """Build an in-memory ZIP with the given filename -> content mapping."""
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, "w", compression=zipfile.ZIP_DEFLATED) as zf:
        for name, data in files.items():
            zf.writestr(name, data)
    return buf.getvalue()


def _make_nested_zip() -> bytes:
    """Build an outer ZIP that contains an inner ZIP (SAM.gov pattern).

    Outer ZIP:
      README.txt
      attachments.zip  <-- inner ZIP containing the real solicitation docs
    Inner ZIP:
      solicitation.pdf
      pricing.xlsx
    """
    inner_zip_bytes = _make_zip(
        {
            "solicitation.pdf": b"%PDF-1.4 fake pdf content",
            "pricing.xlsx": b"PK fake xlsx content",
        }
    )
    outer_zip_bytes = _make_zip(
        {
            "README.txt": b"See attachments.zip for documents.",
            "attachments.zip": inner_zip_bytes,
        }
    )
    return outer_zip_bytes


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------

class TestExtractFlatZip:
    def test_extract_flat_zip(self, tmp_path: Path):
        """A flat ZIP with PDFs and XLSX extracts all files to dest_dir."""
        zip_bytes = _make_zip(
            {
                "solicitation.pdf": b"%PDF-1.4 fake pdf",
                "sf1449.pdf": b"%PDF-1.4 form pdf",
                "pricing.xlsx": b"PK fake xlsx",
            }
        )

        extracted = extract_solicitation_zip(zip_bytes, tmp_path)

        # All three files should be present
        names = {p.name for p in extracted}
        assert names == {"solicitation.pdf", "sf1449.pdf", "pricing.xlsx"}

        # Files must actually exist on disk
        for path in extracted:
            assert path.exists(), f"{path} was returned but does not exist"

        # Content must be preserved
        assert (tmp_path / "solicitation.pdf").read_bytes() == b"%PDF-1.4 fake pdf"


class TestExtractNestedZip:
    def test_extract_nested_zip(self, tmp_path: Path):
        """A ZIP containing another ZIP extracts the inner files (SAM.gov pattern)."""
        zip_bytes = _make_nested_zip()

        extracted = extract_solicitation_zip(zip_bytes, tmp_path)

        names = {p.name for p in extracted}
        # Inner ZIP contents should be extracted; README from outer also kept
        assert "solicitation.pdf" in names
        assert "pricing.xlsx" in names
        assert "README.txt" in names
        # The inner ZIP itself should NOT appear as a plain file
        assert "attachments.zip" not in names

        for path in extracted:
            assert path.exists(), f"{path} was returned but does not exist"


class TestExtractSkipsMacOSJunk:
    def test_extract_skips_macos_junk(self, tmp_path: Path):
        """Skips __MACOSX metadata directories and .DS_Store files."""
        zip_bytes = _make_zip(
            {
                "solicitation.pdf": b"%PDF-1.4 real doc",
                "__MACOSX/._solicitation.pdf": b"mac metadata garbage",
                ".DS_Store": b"mac finder junk",
                "Thumbs.db": b"windows thumbs junk",
                "__pycache__/something.pyc": b"python cache junk",
            }
        )

        extracted = extract_solicitation_zip(zip_bytes, tmp_path)

        names = {p.name for p in extracted}
        assert names == {"solicitation.pdf"}, f"Expected only real doc, got: {names}"
