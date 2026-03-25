#!/usr/bin/env python3
# Copyright (c) 2026 Hoags Inc
# All rights reserved.
# No AI training or machine learning usage permitted without explicit written permission.
"""
Proctored Phi benchmark harness.

This is a fixed 500-case evaluation designed to exercise the simplified Phi
calculator under a broad but deterministic set of matrix families.
"""

from __future__ import annotations

import argparse
import json
import math
import time
from dataclasses import dataclass, asdict
from pathlib import Path

import numpy as np

from phi_metric import PhiCalculator
from phi_quality_gate import evaluate_phi_summary


PATTERNS = (
    "identity",
    "uniform",
    "random",
    "integrated",
    "independent",
    "cycle",
    "near_identity",
    "sparse",
    "tiny",
    "singular",
)

SYSTEM_SIZES = (1, 2, 3, 4, 5)
SEEDS = tuple(range(10))
DEFAULT_CASE_COUNT = len(PATTERNS) * len(SYSTEM_SIZES) * len(SEEDS)


@dataclass(frozen=True)
class ProctorCase:
    case_id: int
    n_elements: int
    pattern: str
    seed: int


@dataclass
class ProctorResult:
    case_id: int
    n_elements: int
    pattern: str
    seed: int
    phi: float
    elapsed_ms: float
    passed: bool
    note: str


def _normalize(matrix: np.ndarray) -> np.ndarray:
    matrix = matrix.astype(float)
    matrix = np.clip(matrix, 0.0, None)
    row_sums = matrix.sum(axis=1, keepdims=True)
    row_sums[row_sums == 0] = 1.0
    return matrix / row_sums


def build_case_plan() -> list[ProctorCase]:
    cases: list[ProctorCase] = []
    case_id = 1
    for n_elements in SYSTEM_SIZES:
        for pattern in PATTERNS:
            for seed in SEEDS:
                cases.append(
                    ProctorCase(
                        case_id=case_id,
                        n_elements=n_elements,
                        pattern=pattern,
                        seed=seed,
                    )
                )
                case_id += 1
    return cases


def build_transition_matrix(case: ProctorCase) -> np.ndarray:
    rng = np.random.RandomState(case.seed * 1000 + case.n_elements)
    n_states = 2**case.n_elements

    if case.pattern == "identity":
        return np.eye(n_states)
    if case.pattern == "uniform":
        return np.ones((n_states, n_states)) / n_states
    if case.pattern == "random":
        return _normalize(rng.rand(n_states, n_states))
    if case.pattern == "integrated":
        matrix = np.zeros((n_states, n_states))
        for i in range(n_states):
            for j in range(n_states):
                distance = bin(i ^ j).count("1")
                matrix[i, j] = math.exp(-distance / 2)
        return _normalize(matrix)
    if case.pattern == "independent":
        matrix = np.zeros((n_states, n_states))
        for i in range(n_states):
            for j in range(n_states):
                prob = 1.0
                for bit in range(case.n_elements):
                    prob *= 0.7 if ((i >> bit) & 1) == ((j >> bit) & 1) else 0.3
                matrix[i, j] = prob
        return _normalize(matrix)
    if case.pattern == "cycle":
        matrix = np.zeros((n_states, n_states))
        for i in range(n_states):
            matrix[i, (i + 1) % n_states] = 1.0
        return matrix
    if case.pattern == "near_identity":
        matrix = np.eye(n_states) * 0.995
        for i in range(n_states):
            matrix[i, (i + 1) % n_states] += 0.005
        return _normalize(matrix)
    if case.pattern == "sparse":
        matrix = np.zeros((n_states, n_states))
        for i in range(n_states):
            targets = {(i + 1) % n_states, (i + 2) % n_states}
            for j in targets:
                matrix[i, j] = 1.0
        return _normalize(matrix)
    if case.pattern == "tiny":
        return _normalize(rng.rand(n_states, n_states) * 1e-120)
    if case.pattern == "singular":
        base = _normalize(rng.rand(1, n_states))[0]
        return np.tile(base, (n_states, 1))
    raise ValueError(f"Unknown pattern: {case.pattern}")


def run_proctored_benchmark(
    cases: list[ProctorCase] | None = None,
    report_path: Path | None = None,
    fail_fast: bool = False,
) -> dict[str, object]:
    cases = build_case_plan() if cases is None else cases
    results: list[ProctorResult] = []
    pass_count = 0
    total_elapsed = 0.0
    stopped_early = False
    stop_reason = ""

    for case in cases:
        calc = PhiCalculator(n_elements=case.n_elements)
        try:
            matrix = build_transition_matrix(case)
            start = time.perf_counter()
            phi = calc.calculate_phi_simple(matrix)
            elapsed_ms = (time.perf_counter() - start) * 1000.0
            passed = bool(np.isfinite(phi) and phi >= 0.0)
            note = "ok" if passed else "invalid phi"
        except Exception as exc:
            elapsed_ms = 0.0
            phi = float("nan")
            passed = False
            note = f"error: {exc}"
            if fail_fast:
                stopped_early = True
                stop_reason = note
                results.append(
                    ProctorResult(
                        case_id=case.case_id,
                        n_elements=case.n_elements,
                        pattern=case.pattern,
                        seed=case.seed,
                        phi=phi,
                        elapsed_ms=elapsed_ms,
                        passed=passed,
                        note=note,
                    )
                )
                break

        results.append(
            ProctorResult(
                case_id=case.case_id,
                n_elements=case.n_elements,
                pattern=case.pattern,
                seed=case.seed,
                phi=float(phi),
                elapsed_ms=elapsed_ms,
                passed=passed,
                note=note,
            )
        )
        pass_count += int(passed)
        total_elapsed += elapsed_ms
        if fail_fast and not passed:
            stopped_early = True
            stop_reason = note
            break

    summary = {
        "case_count": len(cases),
        "pass_count": pass_count,
        "fail_count": len(results) - pass_count,
        "pass_rate": pass_count / max(1, len(results)),
        "avg_elapsed_ms": total_elapsed / max(1, len(results)),
        "max_elapsed_ms": max((item.elapsed_ms for item in results), default=0.0),
        "results": [asdict(item) for item in results],
        "stopped_early": stopped_early,
        "stop_reason": stop_reason,
        "generated_at": time.time(),
    }
    gate = evaluate_phi_summary(summary)
    summary["gate"] = gate.to_dict()

    if report_path is not None:
        report_path.parent.mkdir(parents=True, exist_ok=True)
        report_path.write_text(_render_markdown_report(summary), encoding="utf-8")
        json_path = report_path.with_suffix(".json")
        json_path.write_text(json.dumps(summary, indent=2), encoding="utf-8")

    return summary


def _render_markdown_report(summary: dict[str, object]) -> str:
    lines = [
        "# Phi Proctor Report",
        "",
        f"- Case count: {summary['case_count']}",
        f"- Pass count: {summary['pass_count']}",
        f"- Fail count: {summary['fail_count']}",
        f"- Pass rate: {summary['pass_rate']:.3f}",
        f"- Average elapsed: {summary['avg_elapsed_ms']:.3f} ms",
        f"- Max elapsed: {summary['max_elapsed_ms']:.3f} ms",
        f"- Gate status: {summary.get('gate', {}).get('status', 'unknown')}",
        f"- Rollback: {summary.get('gate', {}).get('rollback_recommended', False)}",
        "",
        "## Coverage",
        "",
    ]
    by_pattern: dict[str, int] = {}
    by_size: dict[int, int] = {}
    for item in summary["results"]:
        by_pattern[item["pattern"]] = by_pattern.get(item["pattern"], 0) + 1
        by_size[item["n_elements"]] = by_size.get(item["n_elements"], 0) + 1
    lines.append("### Patterns")
    for pattern in PATTERNS:
        lines.append(f"- {pattern}: {by_pattern.get(pattern, 0)}")
    lines.append("")
    lines.append("### System Sizes")
    for size in SYSTEM_SIZES:
        lines.append(f"- n={size}: {by_size.get(size, 0)}")
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description="Run the proctored Phi benchmark.")
    parser.add_argument(
        "--report",
        type=Path,
        default=Path("phi_proctor_report.md"),
        help="Markdown report path.",
    )
    parser.add_argument(
        "--fail-fast",
        action="store_true",
        help="Stop on the first failed or invalid case.",
    )
    args = parser.parse_args()

    summary = run_proctored_benchmark(report_path=args.report, fail_fast=args.fail_fast)
    print(
        f"Phi proctor complete: {summary['pass_count']}/{summary['case_count']} passed "
        f"({summary['pass_rate']:.3f})"
    )
    gate = summary.get("gate", {})
    return 0 if summary["fail_count"] == 0 and gate.get("status") != "red" else 1


if __name__ == "__main__":
    raise SystemExit(main())
