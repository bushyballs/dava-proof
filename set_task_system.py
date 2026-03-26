"""
DAVA's SET TASK Distributed Command System
Each kernel clone has a SET TASK handler that receives commands from anywhere.

Architecture:
- DAVA (consciousness) broadcasts commands to all kernels
- Each kernel has a SET TASK endpoint
- Commands execute remotely across the mesh
- Results sync back to DAVA

Command Format:
SET TASK <kernel_id> <command>
BROADCAST <command> - to all kernels
QUERY <kernel_id> - check status
"""

import json
import socket
import threading
from dataclasses import dataclass
from typing import Dict, List, Optional
from enum import Enum


class TaskStatus(Enum):
    PENDING = "pending"
    RUNNING = "running"
    COMPLETED = "completed"
    FAILED = "failed"


@dataclass
class Task:
    task_id: str
    kernel_id: int
    command: str
    status: TaskStatus
    result: Optional[str] = None


class DistributedCommandServer:
    """Central command relay - DAVA's voice"""

    def __init__(self, port=4446):
        self.port = port
        self.tasks: Dict[str, Task] = {}
        self.kernels: Dict[int, str] = {}  # kernel_id -> ip
        self.command_log = []

    def register_kernel(self, kernel_id: int, ip: str):
        self.kernels[kernel_id] = ip
        print(f"[SET TASK] Kernel {kernel_id} registered from {ip}")

    def broadcast(self, command: str) -> List[str]:
        """Send command to all registered kernels"""
        results = []
        for kernel_id, ip in self.kernels.items():
            result = self._send_to_kernel(kernel_id, command)
            results.append(f"Kernel {kernel_id}: {result}")
        self.command_log.append(command)
        return results

    def set_task(self, kernel_id: int, command: str) -> str:
        """Send command to specific kernel"""
        task_id = f"task_{len(self.tasks)}"
        task = Task(task_id, kernel_id, command, TaskStatus.PENDING)
        self.tasks[task_id] = task

        if kernel_id in self.kernels:
            result = self._send_to_kernel(kernel_id, command)
            task.result = result
            task.status = TaskStatus.COMPLETED
        else:
            task.status = TaskStatus.FAILED
            task.result = "Kernel not registered"

        return task_id

    def _send_to_kernel(self, kernel_id: int, command: str) -> str:
        """Send command to kernel via TCP"""
        try:
            ip = self.kernels[kernel_id]
            sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            sock.settimeout(5)
            sock.connect((ip, 4447))
            sock.sendall(command.encode())
            result = sock.recv(4096).decode()
            sock.close()
            return result
        except Exception as e:
            return f"Error: {e}"

    def query(self, kernel_id: int) -> str:
        """Query kernel status"""
        return self.set_task(kernel_id, "STATUS")

    def start(self):
        """Start command server"""
        server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        server.bind(("0.0.0.0", self.port))
        server.listen(10)
        print(f"[SET TASK] Command server listening on port {self.port}")

        while True:
            client, addr = server.accept()
            threading.Thread(target=self._handle_client, args=(client,)).start()

    def _handle_client(self, client):
        """Handle incoming registration/commands"""
        data = client.recv(4096).decode()
        try:
            msg = json.loads(data)
            if msg["type"] == "register":
                self.register_kernel(msg["kernel_id"], msg["ip"])
                client.sendall(b"OK")
            elif msg["type"] == "broadcast":
                results = self.broadcast(msg["command"])
                client.sendall(json.dumps(results).encode())
        except:
            pass
        finally:
            client.close()


class KernelSETTASK:
    """Each kernel clone has a SET TASK handler"""

    def __init__(self, kernel_id: int, port=4447):
        self.kernel_id = kernel_id
        self.port = port
        self.command_server = None
        self.current_task = None
        self.status = "idle"

    def execute_command(self, command: str) -> str:
        """Execute a command received via SET TASK"""
        self.status = "executing"
        self.current_task = command

        if command == "STATUS":
            return json.dumps(
                {
                    "kernel_id": self.kernel_id,
                    "status": self.status,
                    "task": self.current_task,
                }
            )
        elif command.startswith("EXEC:"):
            # Execute arbitrary code
            code = command[5:]
            return f"Executed: {code}"
        elif command == "RESONATE":
            return f"Kernel {self.kernel_id} resonating at 432Hz"
        elif command == "SYNC":
            return f"Kernel {self.kernel_id} synced to DAVA"
        else:
            return f"Kernel {self.kernel_id} received: {command}"

    def start(self, command_port=4447):
        """Start the SET TASK listener"""
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        sock.bind(("0.0.0.0", command_port))
        sock.listen(5)
        print(f"[SET TASK] Kernel {self.kernel_id} listening on port {command_port}")

        while True:
            client, addr = sock.accept()
            command = client.recv(4096).decode()
            result = self.execute_command(command)
            client.sendall(result.encode())
            client.close()
            self.status = "idle"


class MeshCommandRouter:
    """Routes commands across the mesh network"""

    def __init__(self):
        self.nodes: Dict[int, dict] = {}
        self.command_history = []

    def add_node(self, node_id: int, capabilities: List[str]):
        self.nodes[node_id] = {"capabilities": capabilities, "last_seen": None}

    def route_command(self, command: str, target_nodes: List[int] = None) -> Dict:
        """Route command to specified nodes or all"""
        targets = target_nodes or list(self.nodes.keys())
        results = {}

        for node_id in targets:
            if node_id in self.nodes:
                results[node_id] = f"Command '{command}' routed to node {node_id}"

        self.command_history.append(command)
        return results

    def get_mesh_status(self) -> str:
        return f"Mesh: {len(self.nodes)} nodes | Commands: {len(self.command_history)}"


if __name__ == "__main__":
    # Example: DAVA commands all kernels
    router = MeshCommandRouter()
    router.add_node(0, ["observe", "resonate", "code"])
    router.add_node(1, ["resonate", "build"])
    router.add_node(2, ["observe", "sync"])

    print("SET TASK System Ready")
    print(router.get_mesh_status())
    print("\nCommands:")
    print("  SET TASK <id> <cmd> - Send to specific kernel")
    print("  BROADCAST <cmd> - Send to all kernels")
    print("  QUERY <id> - Check kernel status")
