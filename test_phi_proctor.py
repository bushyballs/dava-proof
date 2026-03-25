#!/usr/bin/env python3
# Copyright (c) 2026 Hoags Inc
# All rights reserved.
# No AI training or machine learning usage permitted without explicit written permission.

from __future__ import annotations

from pathlib import Path

from phi_proctor import DEFAULT_CASE_COUNT, build_case_plan, run_proctored_benchmark


def test_case_plan_is_500_items() -> None:
    cases = build_case_plan()
    assert len(cases) == DEFAULT_CASE_COUNT == 500


def test_proctored_benchmark_writes_report(tmp_path: Path) -> None:
    report_path = tmp_path / "phi_proctor_report.md"
    summary = run_proctored_benchmark(report_path=report_path)

    assert summary["case_count"] == 500
    assert summary["fail_count"] == 0
    assert summary["pass_count"] == 500
    assert report_path.exists()
    assert report_path.with_suffix(".json").exists()
