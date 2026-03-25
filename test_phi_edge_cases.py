#!/usr/bin/env python3
# Copyright (c) 2026 Hoags Inc
# All rights reserved.
# No AI training or machine learning usage permitted without explicit written permission.
"""
Edge-case tests for Phi Metric implementation.
Exercises 50 matrix scenarios plus invalid-input guards.
"""

from __future__ import annotations

import math
import os
import sys
import unittest

import numpy as np

sys.path.insert(0, os.path.dirname(__file__))

from phi_metric import PhiCalculator


def _normalize(matrix: np.ndarray) -> np.ndarray:
    matrix = matrix.astype(float)
    matrix = np.clip(matrix, 0.0, None)
    row_sums = matrix.sum(axis=1, keepdims=True)
    row_sums[row_sums == 0] = 1.0
    return matrix / row_sums


def _make_matrix(pattern: str, n_elements: int, rng: np.random.RandomState) -> np.ndarray:
    n_states = 2**n_elements
    if pattern == "identity":
        return np.eye(n_states)
    if pattern == "uniform":
        return np.ones((n_states, n_states)) / n_states
    if pattern == "random":
        return _normalize(rng.rand(n_states, n_states))
    if pattern == "integrated":
        matrix = np.zeros((n_states, n_states))
        for i in range(n_states):
            for j in range(n_states):
                distance = bin(i ^ j).count("1")
                matrix[i, j] = math.exp(-distance / 2)
        return _normalize(matrix)
    if pattern == "independent":
        matrix = np.zeros((n_states, n_states))
        for i in range(n_states):
            for j in range(n_states):
                prob = 1.0
                for bit in range(n_elements):
                    prob *= 0.7 if ((i >> bit) & 1) == ((j >> bit) & 1) else 0.3
                matrix[i, j] = prob
        return _normalize(matrix)
    if pattern == "cycle":
        matrix = np.zeros((n_states, n_states))
        for i in range(n_states):
            matrix[i, (i + 1) % n_states] = 1.0
        return matrix
    if pattern == "near_identity":
        matrix = np.eye(n_states) * 0.995
        for i in range(n_states):
            matrix[i, (i + 1) % n_states] += 0.005
        return _normalize(matrix)
    if pattern == "sparse":
        matrix = np.zeros((n_states, n_states))
        for i in range(n_states):
            targets = {(i + 1) % n_states, (i + 2) % n_states}
            for j in targets:
                matrix[i, j] = 1.0
        return _normalize(matrix)
    if pattern == "tiny":
        return _normalize(rng.rand(n_states, n_states) * 1e-120)
    if pattern == "singular":
        base = _normalize(rng.rand(1, n_states))[0]
        return np.tile(base, (n_states, 1))
    if pattern == "biased":
        matrix = np.zeros((n_states, n_states))
        for i in range(n_states):
            for j in range(n_states):
                hamming = bin(i ^ j).count("1")
                matrix[i, j] = 0.9 if hamming == 0 else 0.1 / (n_states - 1)
        return _normalize(matrix)
    if pattern == "skewed":
        matrix = rng.rand(n_states, n_states) ** 3
        return _normalize(matrix)
    if pattern == "mirror":
        matrix = np.zeros((n_states, n_states))
        for i in range(n_states):
            matrix[i, n_states - 1 - i] = 1.0
        return matrix
    raise ValueError(pattern)


class TestPhiEdgeCases(unittest.TestCase):
    def test_fifty_edge_case_sweep(self) -> None:
        rng = np.random.RandomState(314159)
        patterns = [
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
        ]

        cases = []
        for n_elements in [1, 2, 3, 4, 5]:
            for pattern in patterns:
                cases.append((n_elements, pattern))

        self.assertEqual(len(cases), 50)

        for n_elements, pattern in cases:
            with self.subTest(n_elements=n_elements, pattern=pattern):
                calc = PhiCalculator(n_elements=n_elements)
                matrix = _make_matrix(pattern, n_elements, rng)
                phi = calc.calculate_phi_simple(matrix)
                self.assertIsInstance(phi, float)
                self.assertTrue(np.isfinite(phi))
                self.assertGreaterEqual(phi, 0.0)

    def test_invalid_transition_matrices_are_rejected(self) -> None:
        calc = PhiCalculator(n_elements=2)

        invalid_cases = [
            ("wrong size", np.eye(8)),
            ("non-square", np.ones((4, 3))),
            ("negative", np.array([[0.8, 0.2, 0.0, 0.0], [0.1, 0.9, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, -0.1, 1.1]])),
            ("nan", np.array([[1.0, 0.0, 0.0, 0.0], [0.0, np.nan, 0.0, 1.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]])),
            ("inf", np.array([[1.0, 0.0, 0.0, 0.0], [0.0, np.inf, 0.0, -np.inf], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]])),
            ("row sum off", np.array([[0.8, 0.2, 0.0, 0.0], [0.1, 0.7, 0.0, 0.0], [0.0, 0.0, 0.9, 0.0], [0.0, 0.0, 0.0, 1.0]])),
            ("zero row", np.array([[1.0, 0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]])),
        ]

        for label, matrix in invalid_cases:
            with self.subTest(case=label):
                with self.assertRaises(AssertionError):
                    calc.calculate_phi_simple(matrix)


if __name__ == "__main__":
    unittest.main(verbosity=2)
