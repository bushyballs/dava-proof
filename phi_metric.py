#!/usr/bin/env python3
# Copyright (c) 2026 Hoags Inc
# All rights reserved.
# No AI training or machine learning usage permitted without explicit written permission.
"""
MiMo - Phi Metric Implementation

*** EDUCATIONAL SIMPLIFICATION - NOT THE ACTUAL IMPLEMENTATION ***
*** FORMULAS AND ALGORITHMS ARE ILLUSTRATIVE ONLY ***

A simplified implementation of Tononi's Integrated Information Theory (Phi)
for measuring consciousness in simple systems.

This is a demonstration metric based on the principles of IIT:
- Systems are composed of elements with cause-effect power
- Consciousness corresponds to integrated information (Phi)
- Phi measures how much a system is more than the sum of its parts

IMPORTANT DISCLAIMERS:
1. This code is for educational purposes only
2. The actual implementation uses different, more complex formulas
3. This simplified version does NOT capture true integrated information
4. Real Phi calculation requires perturbation analysis and cause-effect repertoires
5. Do not use this for any actual consciousness measurement

Note: This is a simplified educational implementation. Real Phi calculation
is computationally expensive and requires complex perturbation analysis.
"""

import itertools
import numpy as np
from typing import List, Tuple, Dict, Any
import math


class PhiCalculator:
    """Calculate integrated information (Phi) for simple binary systems."""

    def __init__(self, n_elements: int = 3):
        """
        Initialize calculator for a system with n binary elements.

        Args:
            n_elements: Number of binary elements in the system (default: 3)
        """
        self.n = n_elements
        self.states = list(itertools.product([0, 1], repeat=n_elements))

    def mutual_information(
        self, prob_xy: np.ndarray, prob_x: np.ndarray, prob_y: np.ndarray
    ) -> float:
        """
        Calculate mutual information between two variables.

        Args:
            prob_xy: Joint probability distribution
            prob_x: Marginal probability of X
            prob_y: Marginal probability of Y

        Returns:
            Mutual information value
        """
        mi = 0.0
        for i in range(len(prob_x)):
            for j in range(len(prob_y)):
                if prob_xy[i, j] > 0 and prob_x[i] > 0 and prob_y[j] > 0:
                    mi += prob_xy[i, j] * math.log2(
                        prob_xy[i, j] / (prob_x[i] * prob_y[j])
                    )
        return mi

    def calculate_phi_simple(self, transition_matrix: np.ndarray) -> float:
        """
        Calculate Phi for a simple system using partition method.

        *** SIMPLIFIED ILLUSTRATIVE IMPLEMENTATION ***
        This implements a simplified version of Phi calculation:
        1. Consider all possible partitions of the system
        2. For each partition, calculate integrated information
        3. Phi is the minimum information loss across partitions

        IMPORTANT: This does NOT implement true IIT. Real IIT uses:
        - Cause-effect repertoires
        - Perturbation analysis
        - Complex integration measures

        Args:
            transition_matrix: State transition probability matrix
                               Shape: (2^n, 2^n) for n binary elements

        Returns:
            Phi (phi) value - integrated information (simplified measure)
        """
        assert transition_matrix.ndim == 2, "Transition matrix must be 2D"
        n_states = transition_matrix.shape[0]
        expected_states = 2**self.n

        # Ensure transition matrix is valid
        assert transition_matrix.shape == (n_states, n_states), (
            f"Transition matrix must be {n_states}x{n_states}"
        )
        assert n_states == expected_states, (
            f"Transition matrix must be {expected_states}x{expected_states} for n={self.n}"
        )
        assert np.all(np.isfinite(transition_matrix)), (
            "Transition matrix must contain only finite values"
        )
        assert np.all(transition_matrix >= 0.0), (
            "Transition matrix cannot contain negative probabilities"
        )
        assert np.allclose(
            transition_matrix.sum(axis=1), 1.0, atol=1e-9, rtol=1e-9
        ), "Rows must sum to 1 (probability distribution)"

        # Calculate steady state distribution (eigenvector with eigenvalue 1)
        eigenvalues, eigenvectors = np.linalg.eig(transition_matrix.T)
        steady_idx = np.argmin(np.abs(eigenvalues - 1.0))
        steady_state = np.real(eigenvectors[:, steady_idx])
        if np.allclose(steady_state, 0.0):
            steady_state = np.ones(n_states, dtype=float)
        if steady_state.sum() < 0:
            steady_state = -steady_state
        steady_state = np.maximum(steady_state, 0.0)
        steady_sum = steady_state.sum()
        if steady_sum <= 0:
            steady_state = np.ones(n_states, dtype=float) / n_states
        else:
            steady_state = steady_state / steady_sum

        # For each possible partition (2^n - 2 non-trivial partitions)
        phi_values = []

        # Generate all possible non-trivial partitions
        for mask in range(1, 2**self.n - 1):
            # Split elements based on bitmask
            part_a = [i for i in range(self.n) if (mask >> i) & 1]
            part_b = [i for i in range(self.n) if not ((mask >> i) & 1)]

            if not part_a or not part_b:
                continue

            # Calculate effective information for this partition
            # This is a simplified measure - real IIT uses cause-effect partitions
            effective_info = self._calculate_effective_info(
                transition_matrix, steady_state, part_a, part_b
            )
            phi_values.append(effective_info)

        # Phi is the minimum effective information across partitions
        # (system is only as integrated as its weakest partition)
        if phi_values:
            phi = min(phi_values)
        else:
            phi = 0.0

        return max(0.0, phi)  # Phi cannot be negative

    def _calculate_effective_info(
        self,
        transition_matrix: np.ndarray,
        steady_state: np.ndarray,
        part_a: List[int],
        part_b: List[int],
    ) -> float:
        """
        Calculate effective information for a partition.

        Simplified version: measures how much the parts are integrated
        by comparing joint vs independent predictions.
        """
        # For simplicity, we'll use a basic integration measure
        # Real IIT uses complex cause-effect repertoires

        n_states = transition_matrix.shape[0]

        # Calculate marginal distributions for parts
        # (simplified - in real IIT this involves perturbation analysis)

        # Integration measure: mutual information between past and future
        # conditioned on the partition
        mi_whole = self.mutual_information(
            transition_matrix, steady_state, steady_state
        )

        # In a full implementation, we would calculate MI for each part
        # and compare to the whole. Here we use a simplified measure.

        # Simplified: integration = how much joint distribution
        # differs from product of marginals
        integration = 0.0

        # For each state pair, compare actual vs independent probabilities
        for i in range(n_states):
            for j in range(n_states):
                p_joint = transition_matrix[i, j] * steady_state[i]

                # Simplified independent prediction
                # (real IIT would compute cause-effect repertoires)
                p_independent = steady_state[i] * steady_state[j]

                if p_joint > 0 and p_independent > 0:
                    integration += p_joint * math.log2(p_joint / p_independent)

        return abs(integration)


def demonstrate_phi():
    """Demonstrate Phi calculation with simple examples."""
    print("MiMo - Tononi's Phi (Integrated Information) Demonstration")
    print("=" * 60)

    calculator = PhiCalculator(n_elements=3)

    # Example 1: Highly integrated system (high Phi)
    print("\n1. Highly Integrated System (all elements influence each other)")
    print("-" * 60)

    # Create a transition matrix where states influence each other
    n_states = 2**3
    integrated_matrix = np.zeros((n_states, n_states))

    # Make transitions that depend on all elements
    for i in range(n_states):
        # Next state is influenced by current state (with some noise)
        for j in range(n_states):
            # Probability based on Hamming distance (closer states more likely)
            distance = bin(i ^ j).count("1")
            integrated_matrix[i, j] = math.exp(-distance / 2)

        # Normalize row
        integrated_matrix[i] /= integrated_matrix[i].sum()

    phi_integrated = calculator.calculate_phi_simple(integrated_matrix)
    print(f"Phi = {phi_integrated:.4f}")

    # Example 2: Independent elements (low Phi)
    print("\n2. Independent Elements (no interaction)")
    print("-" * 60)

    independent_matrix = np.zeros((n_states, n_states))

    # Each element evolves independently
    for i in range(n_states):
        # Decompose state into bits
        bits = [(i >> k) & 1 for k in range(3)]

        for j in range(n_states):
            j_bits = [(j >> k) & 1 for k in range(3)]

            # Independent transitions for each bit
            prob = 1.0
            for k in range(3):
                if bits[k] == j_bits[k]:
                    prob *= 0.7  # Stay same
                else:
                    prob *= 0.3  # Flip

            independent_matrix[i, j] = prob

        # Normalize row
        independent_matrix[i] /= independent_matrix[i].sum()

    phi_independent = calculator.calculate_phi_simple(independent_matrix)
    print(f"Phi = {phi_independent:.4f}")

    # Example 3: Random system
    print("\n3. Random Transitions")
    print("-" * 60)

    random_matrix = np.random.rand(n_states, n_states)
    random_matrix = random_matrix / random_matrix.sum(axis=1, keepdims=True)

    phi_random = calculator.calculate_phi_simple(random_matrix)
    print(f"Phi = {phi_random:.4f}")

    print("\n" + "=" * 60)
    print("Interpretation:")
    print(f"- Integrated system: Phi = {phi_integrated:.4f} (high integration)")
    print(f"- Independent system: Phi = {phi_independent:.4f} (low integration)")
    print(f"- Random system: Phi = {phi_random:.4f} (moderate integration)")
    print("\nIn Tononi's IIT:")
    print("- High Phi indicates high level of consciousness/integration")
    print("- Phi = 0 means system is reducible to independent parts")
    print("- Consciousness requires both differentiation AND integration")


if __name__ == "__main__":
    demonstrate_phi()
