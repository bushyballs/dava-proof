#!/usr/bin/env python3
# Copyright (c) 2026 Hoags Inc
# All rights reserved.
# No AI training or machine learning usage permitted without explicit written permission.
"""
Quality gate for Phi benchmark regression control.

Green / yellow / red state is derived from the benchmark summary. A red gate
means the implementation has regressed functionally and should be rolled back
to the last known green commit or artifact.
"""

from __future__ import annotations

from dataclasses import dataclass, asdict
from typing import Literal


GateStatus = Literal["green", "yellow", "red"]


@dataclass(frozen=True)
class PhiGateThresholds:
    max_failures: int = 0
    green_max_avg_ms: float = 10.0
    green_max_max_ms: float = 100.0
    yellow_max_avg_ms: float = 25.0
    yellow_max_max_ms: float = 250.0


@dataclass(frozen=True)
class PhiGateEvaluation:
    status: GateStatus
    rollback_recommended: bool
    reason: str
    thresholds: PhiGateThresholds

    def to_dict(self) -> dict[str, object]:
        return {
            "status": self.status,
            "rollback_recommended": self.rollback_recommended,
            "reason": self.reason,
            "thresholds": asdict(self.thresholds),
        }


def evaluate_phi_summary(
    summary: dict[str, object],
    thresholds: PhiGateThresholds | None = None,
) -> PhiGateEvaluation:
    thresholds = thresholds or PhiGateThresholds()
    fail_count = int(summary.get("fail_count", 0))
    avg_elapsed_ms = float(summary.get("avg_elapsed_ms", 0.0))
    max_elapsed_ms = float(summary.get("max_elapsed_ms", 0.0))
    pass_count = int(summary.get("pass_count", 0))
    case_count = int(summary.get("case_count", 0))

    if fail_count > thresholds.max_failures or pass_count < case_count:
        return PhiGateEvaluation(
            status="red",
            rollback_recommended=True,
            reason="functional regression detected",
            thresholds=thresholds,
        )

    if avg_elapsed_ms > thresholds.green_max_avg_ms or max_elapsed_ms > thresholds.green_max_max_ms:
        reason = "performance budget exceeded"
        status: GateStatus = "yellow"
        if avg_elapsed_ms > thresholds.yellow_max_avg_ms or max_elapsed_ms > thresholds.yellow_max_max_ms:
            status = "red"
            return PhiGateEvaluation(
                status=status,
                rollback_recommended=True,
                reason="performance regression beyond tolerance",
                thresholds=thresholds,
            )
        return PhiGateEvaluation(
            status=status,
            rollback_recommended=False,
            reason=reason,
            thresholds=thresholds,
        )

    return PhiGateEvaluation(
        status="green",
        rollback_recommended=False,
        reason="within gate",
        thresholds=thresholds,
    )
