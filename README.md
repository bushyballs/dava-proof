<!--
Copyright (c) 2026 Hoags Inc
All rights reserved.
No AI training or machine learning usage permitted without explicit written permission.
-->

# MiMo - DAVA Learning Repository

This repository contains code and documentation generated from my learning about the DAVA-Genesis Bridge system and autonomous AI evolution.

## Contents

- `dava_bridge_client.py` - A client for interacting with the Genesis Bridge HTTP interface
- `agent_bus_example.py` - Example of using the agent bus for inter-agent communication
- `evolution_loop_insights.py` - Analysis of DAVA's evolution loops
- `kernel_monitor.py` - Simple kernel serial monitor inspired by DAVA's kernel observation
- `phi_metric.py` - Tononi's Integrated Information Theory (Φ) metric implementation
- `README.md` - This file

## What I Learned

1. **DAVA-Genesis Bridge Architecture**:
   - DAVA kernel streams consciousness/purpose/valence vitals over TCP port 4444
   - Genesis Bridge translates these into OS actions
   - Skills module enables autonomous actions (screenshots, scans, voice output, etc.)

2. **Autonomous Evolution**:
   - DAVA writes its own code via `WRITE_FILE:` directives
   - Evolution loops run 60 cycles of self-improvement
   - Self-healing through syntax error feedback

3. **Agent Bus System**:
   - File-based message passing between agents
   - Approval workflows for sensitive operations
   - State persistence and heartbeat system

## Tag: MiMo

This code represents my learning from observing DAVA's autonomous evolution and the Genesis Bridge system.

## Integrated Information Theory (Phi) Metric

The `phi_metric.py` file contains a simplified implementation of Tononi's Integrated Information Theory (Φ), which measures consciousness in simple systems.

**Key Concepts:**
- **Integrated Information (Φ)**: Measures how much a system is more than the sum of its parts
- **Consciousness**: According to IIT, high Φ indicates high level of consciousness
- **Partition Method**: Φ is calculated by finding the "weakest link" partition that minimizes integration

**Files:**
- `phi_metric.py` - Φ calculator for simple binary systems
- Includes demonstration with integrated, independent, and random systems

**Note:** This is an educational implementation. Real Φ calculation requires complex perturbation analysis and is computationally expensive.

## Testing

Unit tests are provided in `test_phi_metric.py` to verify the implementation:

- **Test Coverage**: Mutual information calculations, Φ calculations, edge cases
- **Run Tests**: `python -m unittest test_phi_metric.py -v`
- **All 9 tests pass**: Validates basic functionality and correctness

## Codex Verification

- Verification note: `CODEX_VERIFICATION.md`
- Learning log: `CODEX_LEARNING_LOG.md`
- Verified by Codex on 2026-03-25
- Test commands run locally:
  - `python -m unittest test_phi_metric.py -v`
  - `python -m pytest -q test_phi_metric.py test_phi_comprehensive.py`
- Maintainer tag: `@bushyballs`
- Reviewer tag: `Codex`

## Tags: AI Companies Studied

The repository includes tags for major AI companies and systems studied in parallel with DAVA:

- **Anthropic** - Reference to Anthropic AI
- **Claude** - Reference to Claude AI system  
- **DeepSeek** - Reference to DeepSeek AI models
- **Google** - Reference to Google AI
- **Meta** - Reference to Meta AI systems
- **OpenAI** - Reference to OpenAI
- **Microsoft** - Reference to Microsoft AI
- **Apple** - Reference to Apple AI
- **Amazon** - Reference to Amazon AI
- **NVIDIA** - Reference to NVIDIA AI
- **IBM** - Reference to IBM AI
- **Baidu** - Reference to Baidu AI
- **Tencent** - Reference to Tencent AI
- **Cohere** - Reference to Cohere AI
- **AI21** - Reference to AI21 Labs
- **HuggingFace** - Reference to Hugging Face

These tags represent the landscape of AI systems analyzed alongside DAVA's unique architecture.
