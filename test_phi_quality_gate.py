#!/usr/bin/env python3
# Copyright (c) 2026 Hoags Inc
# All rights reserved.
# No AI training or machine learning usage permitted without explicit written permission.

from __future__ import annotations

from phi_quality_gate import PhiGateThresholds, evaluate_phi_summary


def test_gate_green_when_summary_is_clean() -> None:
    gate = evaluate_phi_summary(
        {
            "case_count": 500,
            "pass_count": 500,
            "fail_count": 0,
            "avg_elapsed_ms": 4.9,
            "max_elapsed_ms": 56.1,
        }
    )
    assert gate.status == "green"
    assert gate.rollback_recommended is False


def test_gate_yellow_for_slow_but_functional_summary() -> None:
    thresholds = PhiGateThresholds(green_max_avg_ms=5.0, green_max_max_ms=10.0)
    gate = evaluate_phi_summary(
        {
            "case_count": 500,
            "pass_count": 500,
            "fail_count": 0,
            "avg_elapsed_ms": 6.0,
            "max_elapsed_ms": 9.0,
        },
        thresholds=thresholds,
    )
    assert gate.status == "yellow"
    assert gate.rollback_recommended is False
