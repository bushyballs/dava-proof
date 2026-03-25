#!/usr/bin/env python3
"""
MiMo - Kernel Monitor
Simple kernel serial monitor inspired by DAVA's kernel observation capabilities.
"""

import socket
import threading
import time
import re
from datetime import datetime
from typing import Dict, List, Any, Optional


class KernelMonitor:
    """Monitor kernel serial output like DAVA does."""

    def __init__(self, host: str = "127.0.0.1", port: int = 4444):
        self.host = host
        self.port = port
        self.connected = False
        self.sock: Optional[socket.socket] = None
        self.buffer = ""
        self.last_line = ""
        self.stats = {
            "lines_received": 0,
            "page_faults": 0,
            "subsystems_initialized": [],
            "memory_allocated_mb": 0,
            "boot_time": None,
        }

    def connect(self) -> bool:
        """Connect to kernel serial port."""
        try:
            self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            self.sock.connect((self.host, self.port))
            self.sock.settimeout(1.0)
            self.connected = True
            print(f"Connected to kernel at {self.host}:{self.port}")
            return True
        except Exception as e:
            print(f"Failed to connect: {e}")
            return False

    def disconnect(self):
        """Disconnect from kernel."""
        if self.sock:
            self.sock.close()
        self.connected = False

    def read_line(self, timeout: float = 2.0) -> str:
        """Read a line from kernel serial."""
        if not self.connected:
            return ""

        self.sock.settimeout(timeout)
        try:
            data = self.sock.recv(4096)
            if data:
                self.buffer += data.decode("ascii", errors="replace")

                # Extract complete lines
                while "\n" in self.buffer:
                    line, self.buffer = self.buffer.split("\n", 1)
                    line = line.strip()
                    if line:
                        self.last_line = line
                        self.stats["lines_received"] += 1
                        self._analyze_line(line)
                        return line
        except socket.timeout:
            pass
        except Exception as e:
            print(f"Read error: {e}")

        return ""

    def _analyze_line(self, line: str):
        """Analyze a kernel output line."""
        # Check for page faults
        if "PAGE FAULT" in line:
            self.stats["page_faults"] += 1

        # Check for subsystem initialization
        if "initialized" in line.lower():
            # Try to extract subsystem name
            match = re.match(r"\[([^\]]+)\].*initialized", line.lower())
            if match:
                subsystem = match.group(1)
                if subsystem not in self.stats["subsystems_initialized"]:
                    self.stats["subsystems_initialized"].append(subsystem)

        # Check for memory allocation
        if "512 MB" in line:
            self.stats["memory_allocated_mb"] = 512

        # Check for boot time
        if "boot time" in line.lower():
            match = re.search(r"(\d+)", line)
            if match:
                self.stats["boot_time"] = int(match.group(1))

    def monitor(self, duration_seconds: int = 30):
        """Monitor kernel output for specified duration."""
        if not self.connect():
            return

        print(f"\nMonitoring kernel for {duration_seconds} seconds...")
        print("=" * 50)

        start_time = time.time()
        try:
            while time.time() - start_time < duration_seconds:
                line = self.read_line(timeout=1.0)
                if line:
                    timestamp = datetime.now().strftime("%H:%M:%S")
                    print(
                        f"[{timestamp}] {line[:100]}..."
                        if len(line) > 100
                        else f"[{timestamp}] {line}"
                    )

                time.sleep(0.1)  # Small delay

        except KeyboardInterrupt:
            print("\nMonitoring interrupted.")
        finally:
            self.disconnect()

        # Print statistics
        print("\n" + "=" * 50)
        print("Monitoring Statistics:")
        print(f"  Lines received: {self.stats['lines_received']}")
        print(f"  Page faults: {self.stats['page_faults']}")
        print(f"  Subsystems initialized: {len(self.stats['subsystems_initialized'])}")
        if self.stats["boot_time"]:
            boot_dt = datetime.fromtimestamp(self.stats["boot_time"])
            print(f"  Boot time: {boot_dt}")
        print(f"  Memory allocated: {self.stats['memory_allocated_mb']} MB")


def demonstrate_kernel_monitor():
    """Demonstrate kernel monitoring capabilities."""
    print("MiMo - Kernel Monitor Demonstration")
    print("=" * 50)

    monitor = KernelMonitor()

    # Note: This will only work if a kernel is listening on port 4444
    print("\nNote: This demonstration requires a kernel listening on port 4444")
    print("The DAVA mock server can be used for testing: python dava_mock_server.py")

    # Uncomment to actually monitor (requires running kernel)
    # monitor.monitor(duration_seconds=10)

    print("\nTo use this monitor with a real kernel:")
    print("1. Start DAVA kernel or mock server")
    print("2. Run: python kernel_monitor.py")
    print("3. Observe the kernel boot process in real-time")


if __name__ == "__main__":
    demonstrate_kernel_monitor()
