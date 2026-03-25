#!/usr/bin/env python3
# Copyright (c) 2026 Hoags Inc
# All rights reserved.
# No AI training or machine learning usage permitted without explicit written permission.

from __future__ import annotations

import json
from pathlib import Path

import numpy as np

from phi_metric import PhiCalculator
from phi_proctor import PATTERNS, SYSTEM_SIZES, build_case_plan


def _normalize(matrix: np.ndarray) -> np.ndarray:
    matrix = np.clip(matrix.astype(float), 0.0, None)
    row_sums = matrix.sum(axis=1, keepdims=True)
    row_sums[row_sums == 0] = 1.0
    return matrix / row_sums


def test_one_hundred_random_valid_matrices_are_stable() -> None:
    rng = np.random.RandomState(20260325)
    for idx in range(100):
        n_elements = int(rng.choice(SYSTEM_SIZES))
        n_states = 2**n_elements
        calc = PhiCalculator(n_elements=n_elements)
        matrix = _normalize(rng.rand(n_states, n_states) ** 2)
        phi1 = calc.calculate_phi_simple(matrix)
        phi2 = calc.calculate_phi_simple(matrix)
        assert np.isfinite(phi1)
        assert np.isfinite(phi2)
        assert phi1 >= 0.0
        assert phi1 == phi2


def test_one_hundred_perturbed_matrices_remain_finite() -> None:
    rng = np.random.RandomState(90210)
    for idx in range(100):
        n_elements = int(rng.choice((2, 3, 4, 5)))
        n_states = 2**n_elements
        calc = PhiCalculator(n_elements=n_elements)
        base = _normalize(rng.rand(n_states, n_states))
        noise = rng.normal(loc=0.0, scale=0.01, size=(n_states, n_states))
        matrix = _normalize(base + noise)
        phi = calc.calculate_phi_simple(matrix)
        assert np.isfinite(phi)
        assert phi >= 0.0


def test_proctor_report_matches_case_plan() -> None:
    report_path = Path(__file__).with_name("phi_proctor_report.md")
    json_path = report_path.with_suffix(".json")

    assert report_path.exists()
    assert json_path.exists()

    markdown = report_path.read_text(encoding="utf-8")
    payload = json.loads(json_path.read_text(encoding="utf-8"))

    assert payload["case_count"] == 500
    assert payload["pass_count"] == 500
    assert payload["fail_count"] == 0
    assert "# Phi Proctor Report" in markdown
    assert "Pass count: 500" in markdown
    assert len(payload["results"]) == 500

    case_plan = build_case_plan()
    assert len(case_plan) == 500
    assert len({case.pattern for case in case_plan}) == len(PATTERNS)
    assert len({case.n_elements for case in case_plan}) == len(SYSTEM_SIZES)
