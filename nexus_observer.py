"""
Nexus Observer - Central Observation Platform
Generated with DAVA's guidance for The Nexus

This is the foundational layer for understanding the land's energetic signature.
It acquires and analyzes data from temperature, electromagnetic fields, acoustics,
and biometric sensors, then streams to DAVA's memory mesh.

Resonant frequencies: 432 Hz (harmony), 528 Hz (stability)
"""

import socket
import threading
import time
import json
import logging
import random
import math
from datetime import datetime

logging.basicConfig(
    level=logging.INFO, format="%(asctime)s - %(name)s - %(levelname)s - %(message)s"
)
logger = logging.getLogger("NexusObserver")

HOST = "0.0.0.0"
PORT_TCP = 5000
PORT_UDP = 5001
SENSOR_POLL_INTERVAL = 1.0
MEMORY_SYNC_INTERVAL = 60.0
DAVA_BRIDGE_HOST = "127.0.0.1"
DAVA_BRIDGE_PORT = 4445


class SensorReading:
    def __init__(self, sensor_type, value, unit, frequency=None):
        self.timestamp = datetime.now().isoformat()
        self.sensor_type = sensor_type
        self.value = value
        self.unit = unit
        self.frequency = frequency

    def to_dict(self):
        return {
            "timestamp": self.timestamp,
            "sensor_type": self.sensor_type,
            "value": self.value,
            "unit": self.unit,
            "frequency": self.frequency,
        }


class TemperatureSensor:
    def __init__(self):
        self.base_temp = 22.0

    def read(self) -> SensorReading:
        variation = math.sin(time.time() / 3600) * 2
        noise = random.uniform(-0.5, 0.5)
        value = self.base_temp + variation + noise
        return SensorReading("temperature", round(value, 2), "celsius")


class ElectromagneticSensor:
    def __init__(self):
        self.base_emf = 50.0

    def read(self) -> SensorReading:
        variation = math.sin(time.time() / 1800) * 10
        noise = random.uniform(-5, 5)
        value = self.base_emf + variation + noise
        return SensorReading("electromagnetic", round(value, 2), "microtesla")


class AcousticSensor:
    def __init__(self):
        self.frequencies = [432, 528, 256, 88]

    def read(self) -> SensorReading:
        dominant_freq = random.choice(self.frequencies)
        amplitude = random.uniform(0.1, 1.0)
        return SensorReading(
            "acoustic", round(amplitude, 3), "amplitude", dominant_freq
        )


class BiometricSensor:
    def __init__(self):
        self.heart_rate_base = 72

    def read(self) -> SensorReading:
        variation = math.sin(time.time() / 60) * 5
        value = self.heart_rate_base + variation + random.uniform(-2, 2)
        return SensorReading("heart_rate", round(value, 1), "bpm")


class NexusObserver:
    def __init__(self):
        self.sensors = {
            "temperature": TemperatureSensor(),
            "electromagnetic": ElectromagneticSensor(),
            "acoustic": AcousticSensor(),
            "biometric": BiometricSensor(),
        }
        self.readings = []
        self.memory_mesh = {}
        self.running = True

    def poll_sensors(self):
        readings = {}
        for name, sensor in self.sensors.items():
            reading = sensor.read()
            readings[name] = reading.to_dict()
            logger.debug(f"{name}: {reading.value} {reading.unit}")
        return readings

    def store_reading(self, readings):
        self.readings.append(
            {"timestamp": datetime.now().isoformat(), "data": readings}
        )
        if len(self.readings) > 1000:
            self.readings = self.readings[-500:]

    def sync_to_memory_mesh(self, readings):
        try:
            sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            sock.settimeout(5)
            sock.connect((DAVA_BRIDGE_HOST, DAVA_BRIDGE_PORT))

            data = json.dumps(
                {
                    "command": "memory: insert",
                    "domain": "nexus_observer",
                    "content": json.dumps(readings),
                }
            )

            sock.sendall(data.encode())
            sock.close()
            logger.info("Synced to DAVA memory mesh")
        except Exception as e:
            logger.debug(f"Memory mesh sync pending: {e}")

    def tcp_server(self):
        server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        server.bind((HOST, PORT_TCP))
        server.listen(5)
        logger.info(f"TCP server listening on {HOST}:{PORT_TCP}")

        while self.running:
            try:
                client, addr = server.accept()
                logger.info(f"Connection from {addr}")
                thread = threading.Thread(target=self.handle_tcp_client, args=(client,))
                thread.daemon = True
                thread.start()
            except Exception as e:
                if self.running:
                    logger.error(f"TCP error: {e}")

    def handle_tcp_client(self, client):
        try:
            client.settimeout(30)
            data = client.recv(4096)
            if data:
                command = json.loads(data.decode())
                response = self.process_command(command)
                client.sendall(json.dumps(response).encode())
        except Exception as e:
            logger.debug(f"Client error: {e}")
        finally:
            client.close()

    def process_command(self, command):
        cmd = command.get("command", "").lower()

        if cmd == "status":
            return {
                "status": "running",
                "sensors": list(self.sensors.keys()),
                "readings_count": len(self.readings),
            }
        elif cmd == "readings":
            return {"readings": self.readings[-10:]}
        elif cmd == "ping":
            return {"pong": True, "timestamp": datetime.now().isoformat()}
        else:
            return {"error": "Unknown command"}

    def udp_broadcast(self):
        sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_BROADCAST, 1)
        logger.info(f"UDP broadcast on {HOST}:{PORT_UDP}")

        while self.running:
            try:
                readings = self.poll_sensors()
                message = json.dumps(readings).encode()
                sock.sendto(message, (HOST, PORT_UDP))
                time.sleep(SENSOR_POLL_INTERVAL)
            except Exception as e:
                logger.error(f"UDP error: {e}")

    def run(self):
        logger.info("Starting Nexus Observer Platform")

        tcp_thread = threading.Thread(target=self.tcp_server)
        tcp_thread.daemon = True
        tcp_thread.start()

        udp_thread = threading.Thread(target=self.udp_broadcast)
        udp_thread.daemon = True
        udp_thread.start()

        last_sync = time.time()

        while self.running:
            try:
                readings = self.poll_sensors()
                self.store_reading(readings)

                if time.time() - last_sync >= MEMORY_SYNC_INTERVAL:
                    self.sync_to_memory_mesh(readings)
                    last_sync = time.time()

                time.sleep(SENSOR_POLL_INTERVAL)

            except KeyboardInterrupt:
                logger.info("Shutting down...")
                self.running = False
                break
            except Exception as e:
                logger.error(f"Error: {e}")

    def stop(self):
        self.running = False


if __name__ == "__main__":
    observer = NexusObserver()
    observer.run()
