"""engine.cli — Command-line interface for the bid-command engine.

Usage:
    python -m engine.cli process <zip_path>

Reads a solicitation ZIP, runs the full pipeline, prints a summary,
and saves the result as work_dir/parsed.json.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

from engine.pipeline import process_solicitation


def _print_summary(result: dict) -> None:
    """Print a human-readable summary of the parsed solicitation."""
    print()
    print("=" * 60)
    print("  SOLICITATION PARSED")
    print("=" * 60)
    print(f"  Sol Number:    {result['sol_number']}")
    print(f"  NAICS:         {result['naics']}")
    print(f"  Set-Aside:     {result['set_aside']}")
    print(f"  Due Date:      {result['due_date']}")
    print(f"  Location:      {result['city']}, {result['state']}")
    print()
    print(f"  CO Name:       {result['co_name']}")
    print(f"  CO Email:      {result['co_email']}")
    print(f"  CO Phone:      {result['co_phone']}")
    print(f"  Submit Email:  {result['submission_email']}")
    print()
    print(f"  Documents:     {len(result['documents'])}")
    print(f"  Total Pages:   {result['total_pages']}")
    print()

    # Wage data
    wd = result["wage_data"]
    if wd["janitor_rate"] > 0:
        print(f"  SCA Floor:     ${wd['janitor_rate']:.2f}/hr + ${wd['hw_fringe']:.2f} H&W = ${wd['loaded_floor']:.2f}/hr loaded")
        if wd["wd_number"]:
            print(f"  WD Number:     {wd['wd_number']}")
    else:
        print("  SCA Floor:     (no wage determination found)")
    print()

    # CLINs
    if result["clins"]:
        print("  CLINs:")
        for c in result["clins"]:
            print(f"    {c['number']}  {c['description'][:45]:<45}  {c['quantity']:>3} {c['unit']}  [{c['year']}]")
    else:
        print("  CLINs:         (none found)")
    print()

    # Documents
    print("  Documents:")
    for d in result["documents"]:
        print(f"    {d['filename']:<40}  {d['file_type']:<6}  {d['page_count']} pg")

    print()
    print("=" * 60)


def cmd_process(zip_path_str: str) -> None:
    """Process a solicitation ZIP file."""
    zip_path = Path(zip_path_str)
    if not zip_path.exists():
        print(f"Error: ZIP file not found: {zip_path}", file=sys.stderr)
        sys.exit(1)

    # Work directory: alongside the ZIP, named after it (minus extension)
    work_dir = zip_path.parent / zip_path.stem
    work_dir.mkdir(parents=True, exist_ok=True)

    print(f"Processing: {zip_path.name}")
    print(f"Work dir:   {work_dir}")

    zip_bytes = zip_path.read_bytes()
    result = process_solicitation(zip_bytes, work_dir)

    # Save JSON
    json_path = work_dir / "parsed.json"
    with open(json_path, "w") as f:
        json.dump(result, f, indent=2)
    print(f"Saved:      {json_path}")

    _print_summary(result)


def main() -> None:
    """CLI entry point: dispatch subcommand."""
    if len(sys.argv) < 2:
        print("Usage: python -m engine.cli process <zip_path>", file=sys.stderr)
        sys.exit(1)

    command = sys.argv[1]

    if command == "process":
        if len(sys.argv) < 3:
            print("Usage: python -m engine.cli process <zip_path>", file=sys.stderr)
            sys.exit(1)
        cmd_process(sys.argv[2])
    else:
        print(f"Unknown command: {command}", file=sys.stderr)
        print("Available commands: process", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
