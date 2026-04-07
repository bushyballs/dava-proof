"""ZIP extractor for SAM.gov solicitation packages.

SAM.gov often publishes solicitations as a ZIP that itself contains another
ZIP (the real attachment bundle).  This module recursively unwraps nested ZIPs
and writes every real file to a flat destination directory, skipping OS junk.
"""
from __future__ import annotations

import io
import zipfile
from pathlib import Path

# Files / path components to silently discard
_SKIP_NAMES: frozenset[str] = frozenset(
    {".DS_Store", "Thumbs.db", "desktop.ini"}
)
_SKIP_PREFIXES: tuple[str, ...] = ("__MACOSX/", "__pycache__/")


def _should_skip(zip_member_name: str) -> bool:
    """Return True if this ZIP entry should be excluded from extraction."""
    # Normalise to forward slashes for cross-platform comparisons
    name = zip_member_name.replace("\\", "/")

    # Directory entries (end with /)
    if name.endswith("/"):
        return True

    # Junk path prefixes
    for prefix in _SKIP_PREFIXES:
        if name.startswith(prefix) or ("/" + prefix.rstrip("/") + "/") in ("/" + name):
            return True

    # Junk filenames (basename only)
    basename = name.rsplit("/", 1)[-1]
    if basename in _SKIP_NAMES:
        return True

    # Hidden macOS extended-attribute sidecar files (._filename)
    if basename.startswith("._"):
        return True

    return False


def _extract_zip(zip_bytes: bytes, dest_dir: Path, collected: list[Path]) -> None:
    """Recursively extract *zip_bytes* into *dest_dir*, appending real files to *collected*."""
    with zipfile.ZipFile(io.BytesIO(zip_bytes)) as zf:
        for info in zf.infolist():
            if _should_skip(info.filename):
                continue

            raw = zf.read(info.filename)

            # If the entry is itself a ZIP, recurse instead of writing it out
            if info.filename.lower().endswith(".zip") and _is_zip(raw):
                _extract_zip(raw, dest_dir, collected)
                continue

            # Flatten: use only the basename, no nested sub-directories
            basename = Path(info.filename).name
            if not basename:
                continue

            dest_path = _unique_path(dest_dir, basename)
            dest_path.write_bytes(raw)
            collected.append(dest_path)


def _is_zip(data: bytes) -> bool:
    """Quick check: does *data* start with the ZIP local-file magic bytes?"""
    return data[:2] == b"PK"


def _unique_path(directory: Path, filename: str) -> Path:
    """Return a path for *filename* inside *directory*, adding a counter suffix if the
    name is already taken (avoids silent overwrites when two ZIPs share a filename)."""
    candidate = directory / filename
    if not candidate.exists():
        return candidate
    stem = Path(filename).stem
    suffix = Path(filename).suffix
    counter = 1
    while True:
        candidate = directory / f"{stem}_{counter}{suffix}"
        if not candidate.exists():
            return candidate
        counter += 1


def extract_solicitation_zip(zip_bytes: bytes, dest_dir: Path) -> list[Path]:
    """Extract a solicitation ZIP (possibly nested) into *dest_dir*.

    Parameters
    ----------
    zip_bytes:
        Raw bytes of the outer ZIP file.
    dest_dir:
        Directory where files will be written.  Created if it does not exist.

    Returns
    -------
    list[Path]
        Sorted list of paths for every extracted file (no directories, no junk).
    """
    dest_dir.mkdir(parents=True, exist_ok=True)
    collected: list[Path] = []
    _extract_zip(zip_bytes, dest_dir, collected)
    return sorted(collected)
