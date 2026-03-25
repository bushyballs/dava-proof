#!/usr/bin/env python3
# Copyright (c) 2026 Hoags Inc
# All rights reserved.
"""
Stress test and benchmark for Phi Metric implementation.
Generates reports on performance, distribution, and robustness.
"""

import numpy as np
import time
import sys
import os
import matplotlib.pyplot as plt
from collections import defaultdict

sys.path.insert(0, os.path.dirname(__file__))
from phi_metric import PhiCalculator


def generate_system_matrix(
    system_type: str, n_elements: int, rng: np.random.RandomState
) -> np.ndarray:
    """
    Generate transition matrix for different system types.

    Args:
        system_type: One of ['random', 'integrated', 'independent', 'identity', 'uniform']
        n_elements: Number of binary elements
        rng: Random state for reproducibility

    Returns:
        Valid transition matrix
    """
    n_states = 2**n_elements

    if system_type == "random":
        matrix = rng.rand(n_states, n_states)

    elif system_type == "integrated":
        # Create matrix where states influence each other based on Hamming distance
        matrix = np.zeros((n_states, n_states))
        for i in range(n_states):
            for j in range(n_states):
                distance = bin(i ^ j).count("1")
                matrix[i, j] = np.exp(-distance / 2)

    elif system_type == "independent":
        # Each element evolves independently
        matrix = np.zeros((n_states, n_states))
        for i in range(n_states):
            for j in range(n_states):
                prob = 1.0
                for bit in range(n_elements):
                    i_bit = (i >> bit) & 1
                    j_bit = (j >> bit) & 1
                    if i_bit == j_bit:
                        prob *= 0.7
                    else:
                        prob *= 0.3
                matrix[i, j] = prob

    elif system_type == "identity":
        matrix = np.eye(n_states)

    elif system_type == "uniform":
        matrix = np.ones((n_states, n_states))

    else:
        raise ValueError(f"Unknown system type: {system_type}")

    # Normalize rows
    matrix = matrix / matrix.sum(axis=1, keepdims=True)
    return matrix


def run_benchmarks():
    """Run performance benchmarks for different system sizes."""
    print("Phi Metric Performance Benchmarks")
    print("=" * 60)

    rng = np.random.RandomState(42)
    results = []

    for n_elements in [2, 3, 4, 5]:
        calc = PhiCalculator(n_elements=n_elements)
        n_states = 2**n_elements

        print(f"\nSystem size: {n_elements} elements ({n_states} states)")
        print("-" * 40)

        for system_type in ["random", "integrated", "independent"]:
            matrix = generate_system_matrix(system_type, n_elements, rng)

            start_time = time.time()
            phi = calc.calculate_phi_simple(matrix)
            elapsed = time.time() - start_time

            results.append(
                {
                    "elements": n_elements,
                    "states": n_states,
                    "type": system_type,
                    "phi": phi,
                    "time_ms": elapsed * 1000,
                }
            )

            print(
                f"  {system_type:12s}: Phi = {phi:.6f}, time = {elapsed * 1000:.2f} ms"
            )

    return results


def analyze_phi_distribution():
    """Analyze distribution of Phi values for random systems."""
    print("\n\nPhi Distribution Analysis (1000 random systems)")
    print("=" * 60)

    rng = np.random.RandomState(123)
    phi_values = []

    for i in range(1000):
        n_elements = rng.randint(2, 4)  # 2 or 3 elements
        calc = PhiCalculator(n_elements=n_elements)
        matrix = generate_system_matrix("random", n_elements, rng)
        phi = calc.calculate_phi_simple(matrix)
        phi_values.append(phi)

    phi_array = np.array(phi_values)

    print(f"Number of samples: {len(phi_values)}")
    print(f"Mean Phi: {phi_array.mean():.6f}")
    print(f"Std Phi: {phi_array.std():.6f}")
    print(f"Min Phi: {phi_array.min():.6f}")
    print(f"Max Phi: {phi_array.max():.6f}")
    print(f"Median Phi: {np.median(phi_array):.6f}")

    # Create histogram
    try:
        plt.figure(figsize=(10, 6))
        plt.hist(phi_array, bins=50, alpha=0.7, color="blue", edgecolor="black")
        plt.xlabel("Phi (Integrated Information)")
        plt.ylabel("Frequency")
        plt.title("Distribution of Phi Values for Random Systems")
        plt.grid(True, alpha=0.3)
        plt.savefig("phi_distribution.png", dpi=150, bbox_inches="tight")
        print("\nHistogram saved as 'phi_distribution.png'")
    except ImportError:
        print("\nMatplotlib not available for histogram")

    return phi_array


def stress_test_edge_cases():
    """Stress test with edge cases and extreme values."""
    print("\n\nEdge Case Stress Testing")
    print("=" * 60)

    rng = np.random.RandomState(456)
    test_cases = []

    # Test 1: Very small probabilities
    print("\n1. Very small probabilities (1e-100 scale)")
    calc = PhiCalculator(n_elements=2)
    matrix = rng.rand(4, 4) * 1e-100
    matrix = matrix / matrix.sum(axis=1, keepdims=True)
    try:
        phi = calc.calculate_phi_simple(matrix)
        print(
            f"   Result: Phi = {phi:.6e}, Status: {'PASS' if not np.isnan(phi) else 'FAIL'}"
        )
        test_cases.append(("small_probs", phi, not np.isnan(phi)))
    except Exception as e:
        print(f"   Error: {e}")
        test_cases.append(("small_probs", None, False))

    # Test 2: Very large probabilities (near 1.0)
    print("\n2. Very large probabilities (0.999999)")
    matrix = np.eye(4) * 0.999999
    for i in range(4):
        matrix[i, (i + 1) % 4] = 0.000001
    try:
        phi = calc.calculate_phi_simple(matrix)
        print(
            f"   Result: Phi = {phi:.6e}, Status: {'PASS' if not np.isnan(phi) else 'FAIL'}"
        )
        test_cases.append(("large_probs", phi, not np.isnan(phi)))
    except Exception as e:
        print(f"   Error: {e}")
        test_cases.append(("large_probs", None, False))

    # Test 3: Nearly singular matrix
    print("\n3. Nearly singular matrix (rank deficient)")
    matrix = np.ones((4, 4)) / 4  # All rows identical
    try:
        phi = calc.calculate_phi_simple(matrix)
        print(
            f"   Result: Phi = {phi:.6e}, Status: {'PASS' if not np.isnan(phi) else 'FAIL'}"
        )
        test_cases.append(("singular", phi, not np.isnan(phi)))
    except Exception as e:
        print(f"   Error: {e}")
        test_cases.append(("singular", None, False))

    # Test 4: Maximum system size test
    print("\n4. Maximum system size (5 elements, 32 states)")
    calc5 = PhiCalculator(n_elements=5)
    matrix = generate_system_matrix("random", 5, rng)
    try:
        start = time.time()
        phi = calc5.calculate_phi_simple(matrix)
        elapsed = time.time() - start
        print(f"   Result: Phi = {phi:.6f}, Time = {elapsed:.2f}s, Status: PASS")
        test_cases.append(("max_size", phi, True))
    except Exception as e:
        print(f"   Error: {e}")
        test_cases.append(("max_size", None, False))

    return test_cases


def verify_mathematical_properties():
    """Verify mathematical properties of Phi."""
    print("\n\nMathematical Property Verification")
    print("=" * 60)

    properties = []

    # Property 1: Phi is non-negative
    print("\n1. Non-negativity: Phi >= 0 for all systems")
    rng = np.random.RandomState(789)
    all_non_negative = True
    for _ in range(100):
        n_elements = rng.randint(2, 4)
        calc = PhiCalculator(n_elements=n_elements)
        matrix = generate_system_matrix("random", n_elements, rng)
        phi = calc.calculate_phi_simple(matrix)
        if phi < 0:
            all_non_negative = False
            break
    print(f"   Result: {'PASS' if all_non_negative else 'FAIL'}")
    properties.append(("non_negative", all_non_negative))

    # Property 2: Phi of identity matrix
    print("\n2. Identity matrix Phi (each state maps to itself)")
    calc = PhiCalculator(n_elements=3)
    matrix = np.eye(8)
    phi = calc.calculate_phi_simple(matrix)
    print(f"   Result: Phi = {phi:.6f}")
    properties.append(("identity", phi))

    # Property 3: Phi of uniform matrix
    print("\n3. Uniform matrix Phi (maximum entropy)")
    matrix = np.ones((8, 8)) / 8
    phi = calc.calculate_phi_simple(matrix)
    print(f"   Result: Phi = {phi:.6f}")
    properties.append(("uniform", phi))

    # Property 4: Monotonicity with integration (simplified test)
    print("\n4. Integration effect (comparing random vs integrated)")
    integrated_matrix = generate_system_matrix("integrated", 3, rng)
    random_matrix = generate_system_matrix("random", 3, rng)

    phi_integrated = calc.calculate_phi_simple(integrated_matrix)
    phi_random = calc.calculate_phi_simple(random_matrix)

    print(f"   Integrated Phi: {phi_integrated:.6f}")
    print(f"   Random Phi: {phi_random:.6f}")
    print(f"   Integrated > Random: {phi_integrated > phi_random}")
    properties.append(("integration_effect", phi_integrated > phi_random))

    return properties


def generate_report():
    """Generate comprehensive test report."""
    print("PHI METRIC COMPREHENSIVE TEST REPORT")
    print("=" * 60)
    print(f"Generated: {time.ctime()}")
    print(f"Python: {sys.version}")
    print(f"NumPy: {np.__version__}")

    # Run all tests
    benchmarks = run_benchmarks()
    distribution = analyze_phi_distribution()
    edge_cases = stress_test_edge_cases()
    properties = verify_mathematical_properties()

    # Summary
    print("\n\nTEST SUMMARY")
    print("=" * 60)

    # Benchmark summary
    print("\nPerformance Summary:")
    for result in benchmarks[:6]:  # Show first 6
        print(
            f"  {result['elements']} elements, {result['type']:12s}: {result['time_ms']:.2f} ms"
        )

    # Distribution summary
    print("\nDistribution Summary:")
    print(f"  Random systems tested: {len(distribution)}")
    print(f"  Mean Phi: {distribution.mean():.6f}")
    print(f"  Phi always non-negative: {np.all(distribution >= 0)}")

    # Edge case summary
    print("\nEdge Case Summary:")
    passed = sum(1 for _, _, success in edge_cases if success)
    total = len(edge_cases)
    print(f"  Passed: {passed}/{total}")

    # Property summary
    print("\nMathematical Properties:")
    for prop_name, value in properties:
        if isinstance(value, bool):
            print(f"  {prop_name}: {'PASS' if value else 'FAIL'}")
        else:
            print(f"  {prop_name}: {value:.6f}")

    # Final verdict
    print("\n" + "=" * 60)
    print("FINAL VERDICT: ", end="")

    # Check critical failures
    failures = []
    if not np.all(distribution >= 0):
        failures.append("Negative Phi values detected")
    if passed < total:
        failures.append(f"Edge cases failed: {total - passed}")

    if not failures:
        print("ALL TESTS PASSED")
        print("The phi metric implementation is robust and mathematically consistent.")
    else:
        print("ISSUES DETECTED")
        for failure in failures:
            print(f"  - {failure}")


if __name__ == "__main__":
    generate_report()
