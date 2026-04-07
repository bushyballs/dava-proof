"""engine.pipeline — Full solicitation processing pipeline.

Takes raw ZIP bytes, extracts documents, reads them, parses solicitation data,
CLINs, and wage determinations, and returns a structured result dict.
"""

from __future__ import annotations

from pathlib import Path

from engine.ingest.zip_extractor import extract_solicitation_zip
from engine.ingest.doc_reader import read_document, DocContent
from engine.parse.solicitation import parse_solicitation
from engine.parse.clins import parse_clins
from engine.parse.wage_det import parse_wage_determination


def process_solicitation(zip_bytes: bytes, work_dir: Path) -> dict:
    """Process a solicitation ZIP through the full pipeline.

    Steps:
        1. Create work_dir/extracted and extract ZIP contents
        2. Read each extracted document
        3. Concatenate all text
        4. Parse solicitation fields, CLINs, and wage determination
        5. Return structured result dict

    Parameters
    ----------
    zip_bytes:
        Raw bytes of the solicitation ZIP file.
    work_dir:
        Working directory for extraction and output artifacts.

    Returns
    -------
    dict
        Parsed solicitation data with status, fields, CLINs, wage data,
        and document metadata.
    """
    # 1. Extract ZIP
    extract_dir = work_dir / "extracted"
    extract_dir.mkdir(parents=True, exist_ok=True)
    file_paths = extract_solicitation_zip(zip_bytes, extract_dir)

    # 2. Read each document
    documents: list[DocContent] = []
    for path in file_paths:
        doc = read_document(path)
        documents.append(doc)

    # 3. Concatenate all text
    full_text = "\n\n".join(doc.text for doc in documents if doc.text)

    # 4. Parse
    sol_data = parse_solicitation(full_text)
    clins = parse_clins(full_text)
    wage_data = parse_wage_determination(full_text)

    # 5. Build result
    return {
        "status": "parsed",
        "sol_number": sol_data.sol_number,
        "title": sol_data.title,
        "naics": sol_data.naics,
        "set_aside": sol_data.set_aside,
        "due_date": sol_data.due_date,
        "co_name": sol_data.co_name,
        "co_email": sol_data.co_email,
        "co_phone": sol_data.co_phone,
        "state": sol_data.state,
        "city": sol_data.city,
        "submission_email": sol_data.submission_email,
        "clins": [
            {
                "number": c.number,
                "description": c.description,
                "quantity": c.quantity,
                "unit": c.unit,
                "year": c.year,
            }
            for c in clins
        ],
        "wage_data": {
            "wd_number": wage_data.wd_number,
            "janitor_rate": wage_data.janitor_rate,
            "hw_fringe": wage_data.hw_fringe,
            "loaded_floor": wage_data.loaded_floor,
        },
        "documents": [
            {
                "filename": d.filename,
                "file_type": d.file_type,
                "page_count": d.page_count,
                "path": str(d.file_path),
            }
            for d in documents
        ],
        "total_pages": sum(d.page_count for d in documents),
        "work_dir": str(work_dir),
    }
