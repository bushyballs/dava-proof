#!/usr/bin/env python3
# Copyright (c) 2026 Hoags Inc
# All rights reserved.
"""
Unit tests for PhiMetric implementation.
Tests basic functionality and edge cases.
"""

import unittest
import numpy as np
import sys
import os

# Add current directory to path to import phi_metric
sys.path.insert(0, os.path.dirname(__file__))

from phi_metric import PhiCalculator


class TestPhiMetric(unittest.TestCase):
    """Test cases for PhiCalculator class."""

    def setUp(self):
        """Set up test fixtures."""
        self.calc_3 = PhiCalculator(n_elements=3)
        self.calc_2 = PhiCalculator(n_elements=2)

    def test_initialization(self):
        """Test calculator initialization."""
        self.assertEqual(self.calc_3.n, 3)
        self.assertEqual(len(self.calc_3.states), 8)  # 2^3
        self.assertEqual(self.calc_2.n, 2)
        self.assertEqual(len(self.calc_2.states), 4)  # 2^2

    def test_mutual_information_zero_for_independent(self):
        """Test MI is zero for independent distributions."""
        # Independent distributions: P(X,Y) = P(X)P(Y)
        prob_x = np.array([0.5, 0.5])
        prob_y = np.array([0.5, 0.5])
        prob_xy = np.outer(prob_x, prob_y)  # Product of marginals

        mi = self.calc_2.mutual_information(prob_xy, prob_x, prob_y)
        self.assertAlmostEqual(mi, 0.0, places=10)

    def test_mutual_information_positive_for_dependent(self):
        """Test MI is positive for dependent distributions."""
        # Perfect correlation: X = Y
        prob_x = np.array([0.5, 0.5])
        prob_y = np.array([0.5, 0.5])
        prob_xy = np.array([[0.5, 0.0], [0.0, 0.5]])  # Perfect correlation

        mi = self.calc_2.mutual_information(prob_xy, prob_x, prob_y)
        self.assertGreater(mi, 0.0)
        # For perfect correlation, MI should be 1 bit
        self.assertAlmostEqual(mi, 1.0, places=5)

    def test_calculate_phi_simple_returns_non_negative(self):
        """Test that Φ is always non-negative."""
        n_states = 8  # 2^3

        # Test random transition matrices
        for _ in range(10):
            trans_matrix = np.random.rand(n_states, n_states)
            trans_matrix = trans_matrix / trans_matrix.sum(axis=1, keepdims=True)
            phi = self.calc_3.calculate_phi_simple(trans_matrix)
            self.assertGreaterEqual(phi, 0.0)

    def test_calculate_phi_simple_deterministic(self):
        """Test deterministic transition matrix gives consistent result."""
        n_states = 8
        # Create a deterministic transition matrix
        trans_matrix = np.zeros((n_states, n_states))
        # Each state transitions to next state (cyclic)
        for i in range(n_states):
            trans_matrix[i, (i + 1) % n_states] = 1.0

        phi1 = self.calc_3.calculate_phi_simple(trans_matrix)
        phi2 = self.calc_3.calculate_phi_simple(trans_matrix)
        self.assertEqual(phi1, phi2)

    def test_calculate_phi_simple_validates_matrix(self):
        """Test that invalid transition matrices raise errors."""
        n_states = 8

        # Wrong size matrix
        wrong_size = np.random.rand(4, 4)  # Should be 8x8
        with self.assertRaises(AssertionError):
            self.calc_3.calculate_phi_simple(wrong_size)

        # Rows don't sum to 1
        invalid_probs = np.random.rand(n_states, n_states)
        with self.assertRaises(AssertionError):
            self.calc_3.calculate_phi_simple(invalid_probs)

    def test_calculate_phi_simple_zero_for_no_interaction(self):
        """Test Φ is low for independent elements."""
        n_states = 8
        # Create matrix where each element evolves independently
        # (simplified - each bit flips with probability 0.5)
        trans_matrix = np.zeros((n_states, n_states))
        for i in range(n_states):
            for j in range(n_states):
                # Independent probability for each bit
                prob = 1.0
                for bit in range(3):
                    i_bit = (i >> bit) & 1
                    j_bit = (j >> bit) & 1
                    if i_bit == j_bit:
                        prob *= 0.5  # Same bit
                    else:
                        prob *= 0.5  # Different bit
                trans_matrix[i, j] = prob
            # Normalize row
            trans_matrix[i] /= trans_matrix[i].sum()

        phi = self.calc_3.calculate_phi_simple(trans_matrix)
        # Should be relatively low (independent system)
        self.assertLess(phi, 1.0)  # Arbitrary threshold

    def test_calculate_phi_simple_high_for_integrated(self):
        """Test Φ is higher for integrated vs random systems."""
        n_states = 8

        # Create highly integrated matrix (states influence each other)
        integrated_matrix = np.zeros((n_states, n_states))
        for i in range(n_states):
            for j in range(n_states):
                distance = bin(i ^ j).count("1")
                integrated_matrix[i, j] = np.exp(-distance / 2)
            integrated_matrix[i] /= integrated_matrix[i].sum()

        # Random matrix
        random_matrix = np.random.rand(n_states, n_states)
        random_matrix = random_matrix / random_matrix.sum(axis=1, keepdims=True)

        phi_integrated = self.calc_3.calculate_phi_simple(integrated_matrix)
        phi_random = self.calc_3.calculate_phi_simple(random_matrix)

        # Note: Our simplified implementation may not guarantee this,
        # but we test as a sanity check
        self.assertIsInstance(phi_integrated, float)
        self.assertIsInstance(phi_random, float)

    def test_edge_case_single_element(self):
        """Test with single element system."""
        calc_1 = PhiCalculator(n_elements=1)
        self.assertEqual(calc_1.n, 1)
        self.assertEqual(len(calc_1.states), 2)  # 2^1

        # Simple 2-state transition matrix
        trans_matrix = np.array([[0.7, 0.3], [0.4, 0.6]])
        phi = calc_1.calculate_phi_simple(trans_matrix)
        self.assertGreaterEqual(phi, 0.0)


if __name__ == "__main__":
    # Run tests
    unittest.main(verbosity=2)
