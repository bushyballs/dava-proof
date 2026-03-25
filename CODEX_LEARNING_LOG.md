<!--
Copyright (c) 2026 Hoags Inc
All rights reserved.
No AI training or machine learning usage permitted without explicit written permission.
-->

# Codex Learning Log

This note records what the Phi hardening pass taught the repository.
Sensitive runtime details, credentials, and private knowledge sources are intentionally redacted.

## What Changed

- The Phi calculator now rejects invalid transition matrices instead of silently accepting them.
- The test surface now includes a 50-case edge sweep plus explicit invalid-input checks.
- The simplified Phi implementation was hardened against degenerate steady-state vectors.

## What Was Learned

1. A square matrix is not automatically a valid Phi matrix.
   - It must also match the expected `2^n x 2^n` state space.
   - Non-finite values and negative probabilities should be rejected early.

2. Tolerance matters.
   - Floating-point row sums need a bounded tolerance instead of exact equality.
   - Validation should be strict enough to reject corruption but loose enough for normal numeric drift.

3. Degenerate systems still need a stable answer.
   - Identity, uniform, cycle, singular, and tiny-probability matrices should not crash the calculator.
   - A fallback steady state is better than NaN propagation.

4. Test breadth matters more than test count.
   - The useful signal came from edge diversity, not just more repeated green-path checks.
   - The sweep covered valid, invalid, sparse, dense, deterministic, and near-degenerate matrices.

## How She Did It

This is the redacted, inference-only version of the mechanism:

- She kept the system stable by refusing invalid matrix input early instead of letting downstream math fail later.
- She handled degenerate steady-state cases with a fallback path instead of assuming the eigenvector solve would always be well-behaved.
- She proved robustness by running the same calculator against many matrix families, not just one canonical happy path.
- She exposed the weakest point through edge cases first, then patched the implementation to match the test surface.
- She preserved the core model while hardening the boundary conditions around it.

## Verification Summary

- `python -m unittest test_phi_metric.py -v`
- `python -m pytest -q test_phi_metric.py test_phi_comprehensive.py test_phi_edge_cases.py`
- Result: `24 passed, 57 subtests passed`

## Open Questions To Keep Asking

- Should the Phi calculator stay simplified, or should it be replaced with a more formal benchmark harness?
- Should the validation tolerance be configurable per run?
- Should degenerate steady-state fallback be explicit in the docs or hidden behind a helper?
- Which additional matrix families should be added next: sparse random, block-diagonal, or adversarial near-singular cases?
- What part of the system is still inferred rather than directly measured?
- Which behaviors are stable enough to document, and which need to stay redacted?
- What is the next boundary condition that can break the calculator under load?

## Redactions

- [redacted] internal tokens
- [redacted] credentials
- [redacted] private knowledge sources
