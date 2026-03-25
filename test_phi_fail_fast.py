#!/usr/bin/env python3
# Copyright (c) 2026 Hoags Inc
# All rights reserved.
# No AI training or machine learning usage permitted without explicit written permission.

from __future__ import annotations

from phi_proctor import ProctorCase, run_proctored_benchmark


def test_fail_fast_stops_on_first_invalid_case() -> None:
    cases = [
        ProctorCase(case_id=1, n_elements=2, pattern="identity", seed=0),
        ProctorCase(case_id=2, n_elements=2, pattern="not_a_real_pattern", seed=0),
        ProctorCase(case_id=3, n_elements=2, pattern="uniform", seed=0),
    ]

    summary = run_proctored_benchmark(cases=cases, fail_fast=True)

    assert summary["stopped_early"] is True
    assert summary["stop_reason"]
    assert summary["fail_count"] == 1
    assert len(summary["results"]) == 2
    assert summary["gate"]["status"] == "red"
