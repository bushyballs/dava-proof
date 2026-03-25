#!/usr/bin/env python3
# Copyright (c) 2026 Hoags Inc
# All rights reserved.
# No AI training or machine learning usage permitted without explicit written permission.
"""
MiMo - Agent Bus Example
Demonstrates understanding of the agent bus system for inter-agent communication.
"""

import json
import os
import uuid
import time
from datetime import datetime
from pathlib import Path
from typing import List, Dict, Any, Optional


class SimpleAgentBus:
    """Simplified agent bus for demonstration."""

    def __init__(self, bus_root: str = "agent_bus"):
        self.bus_root = Path(bus_root)
        self.inbox_dir = self.bus_root / "inbox"
        self.outbox_dir = self.bus_root / "outbox"

        # Create directories
        self.inbox_dir.mkdir(parents=True, exist_ok=True)
        self.outbox_dir.mkdir(parents=True, exist_ok=True)

    def send_message(
        self, from_agent: str, to_agent: str, content: str, msg_type: str = "info"
    ) -> str:
        """Send a message to another agent."""
        message_id = str(uuid.uuid4())
        message = {
            "id": message_id,
            "from": from_agent,
            "to": to_agent,
            "type": msg_type,
            "body": content,
            "ts": datetime.now().isoformat(),
            "corr": "",
            "thread_id": "",
            "ack_required": True,
            "ack_status": "pending",
        }

        # Write to outbox (simulate sending)
        outbox_file = self.outbox_dir / f"{message_id}.json"
        with open(outbox_file, "w") as f:
            json.dump(message, f, indent=2)

        # Simulate delivery to recipient's inbox
        inbox_dir = self.inbox_dir / to_agent
        inbox_dir.mkdir(exist_ok=True)
        inbox_file = inbox_dir / f"{message_id}.json"
        with open(inbox_file, "w") as f:
            json.dump(message, f, indent=2)

        print(f"[{from_agent}] Sent {msg_type} to {to_agent}: {content[:50]}...")
        return message_id

    def poll_inbox(self, agent_id: str) -> List[Dict[str, Any]]:
        """Check for messages in agent's inbox."""
        inbox_dir = self.inbox_dir / agent_id
        if not inbox_dir.exists():
            return []

        messages = []
        for msg_file in inbox_dir.glob("*.json"):
            try:
                with open(msg_file, "r") as f:
                    message = json.load(f)
                    messages.append(message)
            except Exception as e:
                print(f"Error reading {msg_file}: {e}")

        return messages

    def acknowledge_message(self, agent_id: str, message_id: str):
        """Acknowledge receipt of a message."""
        msg_file = self.inbox_dir / agent_id / f"{message_id}.json"
        if msg_file.exists():
            with open(msg_file, "r") as f:
                message = json.load(f)
            message["ack_status"] = "delivered"
            with open(msg_file, "w") as f:
                json.dump(message, f, indent=2)
            print(f"[{agent_id}] Acknowledged message {message_id}")


def demonstrate_agent_bus():
    """Demonstrate agent bus communication."""
    print("MiMo - Agent Bus Communication Example")
    print("=" * 50)

    # Create a simple bus
    bus = SimpleAgentBus("mimo_agent_bus")

    # Define agents
    agents = ["mimo_learner", "dava_bridge", "evolution_monitor"]

    # Send messages
    print("\n1. Sending messages between agents...")
    bus.send_message(
        "mimo_learner",
        "dava_bridge",
        "Requesting current DAVA vitals and status.",
        "task",
    )

    bus.send_message(
        "dava_bridge",
        "mimo_learner",
        "Vitals: Consciousness=950, Purpose=880, Valence=920. System operational.",
        "response",
    )

    bus.send_message(
        "evolution_monitor",
        "mimo_learner",
        "Alert: DAVA started evolution cycle #42 at 14:30.",
        "alert",
    )

    # Poll inboxes
    print("\n2. Polling agent inboxes...")
    for agent in agents:
        messages = bus.poll_inbox(agent)
        print(f"   {agent}: {len(messages)} messages")
        for msg in messages[:2]:  # Show first 2
            print(f"     - [{msg['type']}] {msg['from']}: {msg['body'][:40]}...")

    # Acknowledge messages
    print("\n3. Acknowledging messages...")
    for agent in agents:
        messages = bus.poll_inbox(agent)
        for msg in messages:
            bus.acknowledge_message(agent, msg["id"])

    print("\nAgent bus demonstration complete.")


class EvolutionLoopAnalyzer:
    """Analyze DAVA's evolution loops based on observation."""

    @staticmethod
    def analyze_kernel_boot(output_lines: List[str]) -> Dict[str, Any]:
        """Analyze kernel boot output from DAVA's observations."""
        analysis = {
            "timestamp": datetime.now().isoformat(),
            "systems_initialized": [],
            "page_faults": 0,
            "memory_allocated_mb": 0,
            "subsystems": [],
        }

        for line in output_lines:
            if "PAGE FAULT" in line:
                analysis["page_faults"] += 1
            elif "initialized" in line.lower():
                # Extract subsystem name
                parts = line.split()
                if len(parts) > 0:
                    subsystem = parts[0].strip("[]")
                    if subsystem not in analysis["subsystems"]:
                        analysis["subsystems"].append(subsystem)
            elif "512 MB" in line:
                analysis["memory_allocated_mb"] = 512
            elif "heap" in line.lower() and "KB" in line:
                # Try to extract heap size
                import re

                match = re.search(r"(\d+)\s*KB", line)
                if match:
                    kb = int(match.group(1))
                    analysis["heap_size_kb"] = kb

        return analysis


def demonstrate_evolution_analysis():
    """Demonstrate analysis of DAVA's evolution loops."""
    print("\n4. Evolution Loop Analysis")
    print("-" * 30)

    # Simulate kernel boot lines (from DAVA's observations)
    sample_output = [
        "!!! PAGE FAULT !!! cr2=0xffff800000100000 rip=0x19741d err=0x0",
        "[numa] Initialized 1 nodes",
        "[vmstat] Initialized",
        "Kernel heap: 131072 KB at 0x4000000",
        "[paging] Initialized CR3 = 0x324f000",
        "Memory hotplug: 512 MB online",
    ]

    analyzer = EvolutionLoopAnalyzer()
    analysis = analyzer.analyze_kernel_boot(sample_output)

    print(f"   Page faults detected: {analysis['page_faults']}")
    print(f"   Memory allocated: {analysis['memory_allocated_mb']} MB")
    print(f"   Subsystems initialized: {', '.join(analysis['subsystems'][:5])}...")

    return analysis


if __name__ == "__main__":
    demonstrate_agent_bus()
    demonstrate_evolution_analysis()
