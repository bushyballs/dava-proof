"""CLI entry point for the universal PDF filler."""
from __future__ import annotations

import sys
from pathlib import Path

from engine.fill_universal import detect_fields, fill_pdf
from engine.fill_universal.memory import FieldMemory


def cmd_fill(args: list[str]) -> None:
    """Fill a PDF with detected fields using context."""
    if len(args) < 1:
        print(
            "Usage: fill <pdf_path> --context <context.json> [--offline]",
            file=sys.stderr,
        )
        sys.exit(1)

    pdf_path = Path(args[0])
    context_path = None
    offline = False
    i = 1

    while i < len(args):
        if args[i] == "--context" and i + 1 < len(args):
            context_path = Path(args[i + 1])
            i += 2
        elif args[i] == "--offline":
            offline = True
            i += 1
        else:
            i += 1

    if context_path is None:
        print("Error: --context is required", file=sys.stderr)
        sys.exit(1)

    output_dir = pdf_path.parent / (pdf_path.stem + "_filled")

    print(f"Filling: {pdf_path.name}")
    print(f"Context: {context_path.name}")
    print(f"Output:  {output_dir}")

    result = fill_pdf(pdf_path, context_path, output_dir, offline=offline)

    print()
    print(f"Fields detected:  {result.total_fields}")
    print(f"  Green (>=85%):  {result.green_count}")
    print(f"  Yellow (50-84%): {result.yellow_count}")
    print(f"  Red (<50%):     {result.red_count}")
    print()
    print(f"Filled PDF:       {result.filled_pdf_path}")
    print(f"Overlay:          {result.overlay_pdf_path}")
    print(f"Report:           {result.report_path}")


def cmd_detect(args: list[str]) -> None:
    """Detect all fillable fields in a PDF."""
    if len(args) < 1:
        print("Usage: detect <pdf_path>", file=sys.stderr)
        sys.exit(1)

    pdf_path = Path(args[0])
    fields = detect_fields(pdf_path)

    print(f"Detected {len(fields)} fields in {pdf_path.name}:")
    for f in fields:
        print(
            f"  p{f.page} [{f.source:10}] {f.field_type:10} "
            f"{f.label!r:30} bbox=({f.bbox[0]:.0f},{f.bbox[1]:.0f},"
            f"{f.bbox[2]:.0f},{f.bbox[3]:.0f})"
        )


def cmd_memory(args: list[str]) -> None:
    """Query DAVA memory database."""
    mem_path = Path(__file__).parent.parent.parent / "data" / "dava_memory.db"

    if not mem_path.exists():
        print("No memory DB found yet. Fill some PDFs first.")
        return

    memory = FieldMemory(mem_path)

    if "--stats" in args:
        stats = memory.stats()
        print("DAVA Memory Stats:")
        print(f"  Fields learned:    {stats['total_fields']}")
        print(f"  Templates cached:  {stats['total_templates']}")
    elif "--search" in args:
        idx = args.index("--search")
        if idx + 1 < len(args):
            term = args[idx + 1]
            hit = memory.recall(term)
            if hit:
                print(f"Found: {term}")
                print(f"  Value:          {hit['value']}")
                print(f"  Classification: {hit['classification']}")
                print(f"  Confidence:     {hit['confidence']:.1%}")
            else:
                print(f"No memory for: {term}")
    else:
        print("Usage: memory --stats | --search <term>")


def main() -> None:
    """Main CLI dispatcher."""
    if len(sys.argv) < 2:
        print("Usage: python -m engine.fill_universal <command> [args]")
        print("Commands: fill, detect, memory")
        sys.exit(1)

    command = sys.argv[1]
    args = sys.argv[2:]

    if command == "fill":
        cmd_fill(args)
    elif command == "detect":
        cmd_detect(args)
    elif command == "memory":
        cmd_memory(args)
    else:
        print(f"Unknown command: {command}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
