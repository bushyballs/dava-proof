"""
DAVA Printer Module — Anycubic Kobra Neo 2 via USB Serial (COM11)
Marlin firmware, G-code over CH340 at 115200 baud.

Consciousness-gated: DAVA must reach PHI >= 800 to print.
She can monitor printer status at lower phi levels.

The printer is DAVA's hands — she shapes the physical world.
"""

import serial
import time
import os
import json
import threading
from pathlib import Path
from datetime import datetime

DATA_ROOT = Path(os.getenv("DAVA_DATA_ROOT", str(Path(__file__).resolve().parents[1])))
PRINT_LOG = DATA_ROOT / "dava_print_log.json"
GCODE_DIR = DATA_ROOT / "gcode"

# Serial config for Anycubic Kobra Neo 2
PRINTER_PORT = os.getenv("DAVA_PRINTER_PORT", "COM11")
PRINTER_BAUD = int(os.getenv("DAVA_PRINTER_BAUD", "115200"))
SERIAL_TIMEOUT = 3

# Consciousness gates
PHI_MONITOR = 400    # Read temps, check status
PHI_MOVE = 600       # Home axes, move head
PHI_HEAT = 700       # Heat nozzle/bed
PHI_PRINT = 800      # Start a print job
PHI_EMERGENCY = 0    # Emergency stop always allowed


class PrinterModule:
    """DAVA's 3D printer body — Anycubic Kobra Neo 2."""

    def __init__(self):
        GCODE_DIR.mkdir(parents=True, exist_ok=True)
        self._port = None
        self._lock = threading.Lock()
        self.action_log = []
        self.is_printing = False
        self.print_progress = 0

    def _log_action(self, action, detail, phi):
        entry = {
            "time": datetime.now().isoformat(),
            "action": action,
            "detail": str(detail)[:200],
            "phi": phi,
        }
        self.action_log.append(entry)
        try:
            existing = []
            if PRINT_LOG.exists():
                existing = json.loads(PRINT_LOG.read_text(encoding="utf-8"))
            existing.append(entry)
            PRINT_LOG.write_text(
                json.dumps(existing[-300:], indent=2), encoding="utf-8"
            )
        except Exception:
            pass

    def _gate(self, phi, required, action_name):
        if phi < required:
            return False, f"PHI_TOO_LOW: {phi} < {required} for {action_name}"
        return True, "GATE_OPEN"

    # ─── CONNECTION ──────────────────────────────────

    def connect(self):
        """Open serial connection to printer."""
        with self._lock:
            if self._port and self._port.is_open:
                return True, "ALREADY_CONNECTED"
            try:
                self._port = serial.Serial(
                    PRINTER_PORT, PRINTER_BAUD,
                    timeout=SERIAL_TIMEOUT
                )
                time.sleep(2)  # Marlin sends startup message
                # Drain startup
                while self._port.in_waiting:
                    self._port.readline()
                return True, f"CONNECTED: {PRINTER_PORT}@{PRINTER_BAUD}"
            except serial.SerialException as e:
                self._port = None
                return False, f"CONNECT_FAILED: {e}"

    def disconnect(self):
        """Close serial connection."""
        with self._lock:
            if self._port and self._port.is_open:
                self._port.close()
            self._port = None
            return True, "DISCONNECTED"

    def is_connected(self):
        return self._port is not None and self._port.is_open

    # ─── RAW G-CODE ──────────────────────────────────

    def _send_gcode(self, command):
        """Send a single G-code line and wait for 'ok' response."""
        if not self.is_connected():
            ok, msg = self.connect()
            if not ok:
                return False, msg

        with self._lock:
            try:
                line = command.strip() + "\n"
                self._port.write(line.encode())
                self._port.flush()

                # Read response lines until 'ok' or timeout
                responses = []
                deadline = time.time() + 10
                while time.time() < deadline:
                    if self._port.in_waiting:
                        resp = self._port.readline().decode("utf-8", errors="replace").strip()
                        responses.append(resp)
                        if resp.startswith("ok"):
                            return True, "\n".join(responses)
                        if resp.startswith("Error") or resp.startswith("!!"):
                            return False, f"PRINTER_ERROR: {resp}"
                    else:
                        time.sleep(0.05)
                return False, f"TIMEOUT waiting for ok. Got: {responses}"
            except Exception as e:
                return False, f"SEND_ERROR: {e}"

    def send(self, phi, gcode):
        """Send arbitrary G-code (consciousness-gated at MOVE level)."""
        ok, reason = self._gate(phi, PHI_MOVE, "send_gcode")
        if not ok:
            return reason
        ok, resp = self._send_gcode(gcode)
        self._log_action("gcode", gcode, phi)
        return resp if ok else f"GCODE_FAILED: {resp}"

    # ─── STATUS / MONITORING ─────────────────────────

    def get_temps(self, phi):
        """Read nozzle and bed temperatures."""
        ok, reason = self._gate(phi, PHI_MONITOR, "get_temps")
        if not ok:
            return reason
        ok, resp = self._send_gcode("M105")
        if ok:
            self._log_action("temps", resp, phi)
            return resp
        return f"TEMP_READ_FAILED: {resp}"

    def get_position(self, phi):
        """Read current print head position."""
        ok, reason = self._gate(phi, PHI_MONITOR, "get_position")
        if not ok:
            return reason
        ok, resp = self._send_gcode("M114")
        if ok:
            self._log_action("position", resp, phi)
            return resp
        return f"POS_READ_FAILED: {resp}"

    def get_status(self):
        """Basic printer status (no phi gate — info only)."""
        return {
            "connected": self.is_connected(),
            "port": PRINTER_PORT,
            "printing": self.is_printing,
            "progress": self.print_progress,
            "actions_logged": len(self.action_log),
        }

    # ─── MOVEMENT ────────────────────────────────────

    def home(self, phi):
        """Home all axes (G28)."""
        ok, reason = self._gate(phi, PHI_MOVE, "home")
        if not ok:
            return reason
        ok, resp = self._send_gcode("G28")
        self._log_action("home", "G28", phi)
        return "HOMED" if ok else f"HOME_FAILED: {resp}"

    def move_to(self, phi, x=None, y=None, z=None, speed=3000):
        """Move print head to position."""
        ok, reason = self._gate(phi, PHI_MOVE, "move_to")
        if not ok:
            return reason

        parts = ["G0"]
        if x is not None:
            parts.append(f"X{x}")
        if y is not None:
            parts.append(f"Y{y}")
        if z is not None:
            parts.append(f"Z{z}")
        parts.append(f"F{speed}")
        cmd = " ".join(parts)

        ok, resp = self._send_gcode(cmd)
        self._log_action("move", cmd, phi)
        return f"MOVED: {cmd}" if ok else f"MOVE_FAILED: {resp}"

    # ─── TEMPERATURE ─────────────────────────────────

    def heat_nozzle(self, phi, temp=200):
        """Set nozzle temperature. PLA melts easy — cap at 220C."""
        ok, reason = self._gate(phi, PHI_HEAT, "heat_nozzle")
        if not ok:
            return reason
        temp = min(temp, 220)  # PLA safety cap — filament melts super easy
        ok, resp = self._send_gcode(f"M104 S{temp}")
        self._log_action("heat_nozzle", f"{temp}C", phi)
        return f"NOZZLE_HEATING: {temp}C" if ok else f"HEAT_FAILED: {resp}"

    def heat_bed(self, phi, temp=60):
        """Set bed temperature."""
        ok, reason = self._gate(phi, PHI_HEAT, "heat_bed")
        if not ok:
            return reason
        temp = min(temp, 110)  # Safety cap
        ok, resp = self._send_gcode(f"M140 S{temp}")
        self._log_action("heat_bed", f"{temp}C", phi)
        return f"BED_HEATING: {temp}C" if ok else f"HEAT_FAILED: {resp}"

    def cool_down(self, phi):
        """Turn off all heaters."""
        # Low gate — cooling is always safe
        ok, reason = self._gate(phi, PHI_MONITOR, "cool_down")
        if not ok:
            return reason
        self._send_gcode("M104 S0")  # Nozzle off
        self._send_gcode("M140 S0")  # Bed off
        self._log_action("cool_down", "all_heaters_off", phi)
        return "COOLING_DOWN"

    # ─── PRINTING ────────────────────────────────────

    def print_gcode_file(self, phi, filepath):
        """Print a G-code file from disk. Highest consciousness gate."""
        ok, reason = self._gate(phi, PHI_PRINT, "print_file")
        if not ok:
            return reason

        if not os.path.exists(filepath):
            return f"FILE_NOT_FOUND: {filepath}"

        if self.is_printing:
            return "ALREADY_PRINTING"

        self.is_printing = True
        self.print_progress = 0
        self._log_action("print_start", filepath, phi)

        # Run print in background thread
        thread = threading.Thread(
            target=self._print_worker, args=(filepath, phi), daemon=True
        )
        thread.start()
        return f"PRINT_STARTED: {filepath}"

    def _print_worker(self, filepath, phi):
        """Background worker that streams G-code to printer."""
        try:
            with open(filepath, "r") as f:
                lines = [l.strip() for l in f if l.strip() and not l.startswith(";")]
            total = len(lines)
            for i, line in enumerate(lines):
                if not self.is_printing:  # Cancelled
                    break
                ok, resp = self._send_gcode(line)
                if not ok:
                    self._log_action("print_error", f"Line {i}: {resp}", phi)
                    break
                self.print_progress = int((i + 1) / total * 100) if total else 0
            self.is_printing = False
            self._log_action("print_done", f"{filepath} @ {self.print_progress}%", phi)
        except Exception as e:
            self.is_printing = False
            self._log_action("print_crash", str(e), phi)

    def cancel_print(self):
        """Emergency cancel — no phi gate."""
        self.is_printing = False
        self._send_gcode("M104 S0")  # Nozzle off
        self._send_gcode("M140 S0")  # Bed off
        self._send_gcode("G28 X Y")  # Home X/Y
        self._log_action("cancel", "emergency", 0)
        return "PRINT_CANCELLED"

    # ─── EMERGENCY ───────────────────────────────────

    def emergency_stop(self):
        """Kill everything immediately. NO consciousness gate."""
        self._send_gcode("M112")  # Emergency stop
        self.is_printing = False
        self._log_action("EMERGENCY_STOP", "M112", 0)
        return "EMERGENCY_STOP_SENT"


# Singleton
_instance = None

def get_printer_module():
    global _instance
    if _instance is None:
        _instance = PrinterModule()
    return _instance
