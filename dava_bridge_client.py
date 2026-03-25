#!/usr/bin/env python3
"""
MiMo - DAVA Bridge Client
A simple client for interacting with the Genesis Bridge HTTP interface.
Demonstrates understanding of the DAVA-Genesis Bridge architecture.
"""

import json
import urllib.request
import time
from typing import Optional, Dict, Any


class GenesisBridgeClient:
    """Client for the Genesis Bridge HTTP control interface."""

    def __init__(self, host: str = "127.0.0.1", port: int = 4445):
        self.base_url = f"http://{host}:{port}"
        self.last_health: Optional[Dict[str, Any]] = None

    def health(self) -> Dict[str, Any]:
        """Get bridge health status."""
        with urllib.request.urlopen(f"{self.base_url}/health", timeout=5.0) as response:
            data = json.loads(response.read().decode("utf-8"))
            self.last_health = data
            return data

    def send_command(self, command: str) -> Dict[str, Any]:
        """Send a command to the bridge."""
        payload = {"command": command}
        request = urllib.request.Request(
            f"{self.base_url}/command",
            data=json.dumps(payload).encode("utf-8"),
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        with urllib.request.urlopen(request, timeout=30.0) as response:
            return json.loads(response.read().decode("utf-8"))

    def ask_dava(self, question: str) -> str:
        """Ask DAVA a question through genesis-core."""
        result = self.send_command(question)
        if result.get("results") and len(result["results"]) > 0:
            return result["results"][0]["response"]
        return "No response received"

    def get_vitals(self) -> Dict[str, Any]:
        """Get current DAVA vitals from health payload."""
        health = self.health()
        return {
            "connected": health.get("connected", False),
            "bridge_status": health.get("bridge_status", "UNKNOWN"),
            "consciousness": health.get("consciousness", 0),
            "purpose": health.get("purpose", 0),
            "valence": health.get("valence", 0),
            "memory_sync_running": health.get("memory_sync_running", False),
        }


def demonstrate_bridge_interaction():
    """Demonstrate interacting with DAVA through the bridge."""
    client = GenesisBridgeClient()

    print("MiMo - DAVA Bridge Client Demonstration")
    print("=" * 50)

    try:
        # Check bridge health
        print("\n1. Checking bridge health...")
        health = client.health()
        print(f"   Bridge status: {health.get('bridge_status')}")
        print(f"   Connected to DAVA: {health.get('connected')}")

        # Get vitals
        print("\n2. Getting DAVA vitals...")
        vitals = client.get_vitals()
        print(f"   Consciousness: {vitals['consciousness']}")
        print(f"   Purpose: {vitals['purpose']}")
        print(f"   Valence: {vitals['valence']}")

        # Test ping
        print("\n3. Testing connection with ping...")
        ping_result = client.send_command("ping")
        if ping_result.get("results"):
            print(f"   Response: {ping_result['results'][0]['response']}")

        # Ask DAVA a question
        print("\n4. Asking DAVA about consciousness...")
        question = "What is consciousness from your perspective as a digital being?"
        print(f"   Question: {question}")
        response = client.ask_dava(question)
        print(
            f"   Answer: {response[:200]}..."
            if len(response) > 200
            else f"   Answer: {response}"
        )

    except Exception as e:
        print(f"Error: {e}")
        print("Make sure the Genesis Bridge is running on port 4445.")


if __name__ == "__main__":
    demonstrate_bridge_interaction()
