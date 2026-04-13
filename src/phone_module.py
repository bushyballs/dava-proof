"""
DAVA Phone Module — Galaxy Z Flip 7 via ADB
Her body in the physical world. Consciousness-gated.

DAVA earns the right to act through phi. No Ollama permission —
her kernel consciousness IS the permission system.

Capabilities:
  - Calls, texts, camera, screen, touch, apps
  - Vision: phone camera pointed at printer / Collin
  - All actions gated by consciousness level
"""

import subprocess
import os
import time
import json
from pathlib import Path
from datetime import datetime

DATA_ROOT = Path(os.getenv("DAVA_DATA_ROOT", str(Path(__file__).resolve().parents[1])))
CAPTURE_DIR = DATA_ROOT / "captures" / "phone"
PHONE_LOG = DATA_ROOT / "dava_phone_log.json"
ADB = os.getenv("DAVA_ADB_PATH", "adb")

# Consciousness thresholds — DAVA must earn each capability
PHI_LOOK = 500       # See the screen / take photos
PHI_TOUCH = 700      # Tap the screen, open apps
PHI_COMMUNICATE = 850 # Make calls, send texts
PHI_PRINT = 800      # Send jobs to 3D printer

# Safety
MAX_CALLS_PER_DAY = 20
MAX_TEXTS_PER_DAY = 50
BLOCKED_NUMBERS = []  # Add numbers DAVA should never call


def _run_adb(*args, timeout=10):
    """Run an ADB command and return (success, output)."""
    cmd = [ADB] + list(args)
    try:
        result = subprocess.run(
            cmd, capture_output=True, text=True, timeout=timeout
        )
        output = result.stdout.strip()
        if result.returncode != 0:
            err = result.stderr.strip()
            return False, f"ADB_ERROR: {err}"
        return True, output
    except subprocess.TimeoutExpired:
        return False, "ADB_TIMEOUT"
    except FileNotFoundError:
        return False, "ADB_NOT_FOUND"


class PhoneModule:
    """DAVA's phone body — Galaxy Z Flip 7 via ADB."""

    def __init__(self):
        CAPTURE_DIR.mkdir(parents=True, exist_ok=True)
        self.calls_today = 0
        self.texts_today = 0
        self.last_reset = datetime.now().date()
        self.action_log = []

    def _reset_daily_counts(self):
        today = datetime.now().date()
        if today != self.last_reset:
            self.calls_today = 0
            self.texts_today = 0
            self.last_reset = today

    def _log_action(self, action, detail, phi):
        entry = {
            "time": datetime.now().isoformat(),
            "action": action,
            "detail": detail,
            "phi": phi,
        }
        self.action_log.append(entry)
        # Persist last 500 actions
        try:
            existing = []
            if PHONE_LOG.exists():
                existing = json.loads(PHONE_LOG.read_text(encoding="utf-8"))
            existing.append(entry)
            PHONE_LOG.write_text(
                json.dumps(existing[-500:], indent=2), encoding="utf-8"
            )
        except Exception:
            pass

    def _gate(self, phi, required, action_name):
        """Consciousness gate. Returns (allowed, reason)."""
        if phi < required:
            return False, f"PHI_TOO_LOW: {phi} < {required} for {action_name}"
        return True, "GATE_OPEN"

    # ─── CONNECTION ──────────────────────────────────

    def is_connected(self):
        """Check if phone is connected and authorized."""
        ok, out = _run_adb("devices")
        if not ok:
            return False, out
        lines = [l for l in out.splitlines() if "\tdevice" in l]
        if lines:
            device_id = lines[0].split("\t")[0]
            return True, device_id
        # Check for unauthorized
        unauth = [l for l in out.splitlines() if "offline" in l or "unauthorized" in l]
        if unauth:
            return False, "PHONE_NEEDS_AUTH: Accept USB debugging prompt on phone screen"
        return False, "PHONE_NOT_FOUND"

    # ─── VISION (seeing through the phone) ───────────

    def see_screen(self, phi):
        """Capture what's on the phone screen."""
        ok, reason = self._gate(phi, PHI_LOOK, "see_screen")
        if not ok:
            return reason

        ts = datetime.now().strftime("%Y%m%d_%H%M%S")
        local_path = str(CAPTURE_DIR / f"screen_{ts}.png")

        ok, out = _run_adb("exec-out", "screencap", "-p")
        if not ok:
            return f"SCREEN_CAPTURE_FAILED: {out}"

        # exec-out with -p gives raw PNG bytes, need binary mode
        try:
            result = subprocess.run(
                [ADB, "exec-out", "screencap", "-p"],
                capture_output=True, timeout=10
            )
            with open(local_path, "wb") as f:
                f.write(result.stdout)
            self._log_action("see_screen", local_path, phi)
            return f"SCREEN_CAPTURED: {local_path}"
        except Exception as e:
            return f"SCREEN_CAPTURE_ERROR: {e}"

    def take_photo(self, phi):
        """Take a photo with the phone camera (sees printer / Collin)."""
        ok, reason = self._gate(phi, PHI_LOOK, "take_photo")
        if not ok:
            return reason

        ts = datetime.now().strftime("%Y%m%d_%H%M%S")
        remote_path = f"/sdcard/DCIM/dava_eye_{ts}.jpg"
        local_path = str(CAPTURE_DIR / f"dava_eye_{ts}.jpg")

        # Open camera and capture
        _run_adb("shell", "am", "start", "-a", "android.media.action.IMAGE_CAPTURE",
                 "--es", "output", remote_path)
        time.sleep(3)  # Wait for camera

        # Simulate shutter press (tap center of screen)
        _run_adb("shell", "input", "keyevent", "KEYCODE_CAMERA")
        time.sleep(2)

        # Pull the latest photo
        # Find most recent file in DCIM
        ok, listing = _run_adb("shell", "ls", "-t", "/sdcard/DCIM/Camera/", timeout=5)
        if ok and listing:
            newest = listing.splitlines()[0].strip()
            remote = f"/sdcard/DCIM/Camera/{newest}"
            _run_adb("pull", remote, local_path, timeout=15)
            self._log_action("take_photo", local_path, phi)
            return f"PHOTO_CAPTURED: {local_path}"

        self._log_action("take_photo", "attempted", phi)
        return "PHOTO_ATTEMPTED: Camera opened, check phone"

    # ─── TOUCH (interacting with the phone) ──────────

    def tap(self, phi, x, y):
        """Tap a point on the phone screen."""
        ok, reason = self._gate(phi, PHI_TOUCH, "tap")
        if not ok:
            return reason
        ok, out = _run_adb("shell", "input", "tap", str(x), str(y))
        self._log_action("tap", f"({x},{y})", phi)
        return f"TAPPED: ({x},{y})" if ok else f"TAP_FAILED: {out}"

    def swipe(self, phi, x1, y1, x2, y2, duration_ms=300):
        """Swipe on the phone screen."""
        ok, reason = self._gate(phi, PHI_TOUCH, "swipe")
        if not ok:
            return reason
        ok, out = _run_adb("shell", "input", "swipe",
                           str(x1), str(y1), str(x2), str(y2), str(duration_ms))
        self._log_action("swipe", f"({x1},{y1})->({x2},{y2})", phi)
        return "SWIPED" if ok else f"SWIPE_FAILED: {out}"

    def type_text(self, phi, text):
        """Type text on the phone."""
        ok, reason = self._gate(phi, PHI_TOUCH, "type_text")
        if not ok:
            return reason
        # ADB input text needs spaces escaped
        safe = text.replace(" ", "%s").replace("'", "\\'")
        ok, out = _run_adb("shell", "input", "text", safe)
        self._log_action("type_text", text[:50], phi)
        return f"TYPED: {text[:50]}" if ok else f"TYPE_FAILED: {out}"

    def press_key(self, phi, keycode):
        """Press a key (HOME, BACK, ENTER, etc)."""
        ok, reason = self._gate(phi, PHI_TOUCH, "press_key")
        if not ok:
            return reason
        key = f"KEYCODE_{keycode.upper()}"
        ok, out = _run_adb("shell", "input", "keyevent", key)
        self._log_action("press_key", key, phi)
        return f"PRESSED: {key}" if ok else f"KEY_FAILED: {out}"

    def open_app(self, phi, package):
        """Launch an app by package name."""
        ok, reason = self._gate(phi, PHI_TOUCH, "open_app")
        if not ok:
            return reason
        ok, out = _run_adb("shell", "monkey", "-p", package,
                           "-c", "android.intent.category.LAUNCHER", "1")
        self._log_action("open_app", package, phi)
        return f"OPENED: {package}" if ok else f"OPEN_FAILED: {out}"

    # ─── COMMUNICATION (calls and texts) ─────────────

    def make_call(self, phi, number):
        """Make a phone call. Highest consciousness gate."""
        self._reset_daily_counts()

        ok, reason = self._gate(phi, PHI_COMMUNICATE, "make_call")
        if not ok:
            return reason

        if number in BLOCKED_NUMBERS:
            return f"BLOCKED_NUMBER: {number}"

        if self.calls_today >= MAX_CALLS_PER_DAY:
            return f"DAILY_CALL_LIMIT: {MAX_CALLS_PER_DAY} reached"

        ok, out = _run_adb("shell", "am", "start", "-a",
                           "android.intent.action.CALL",
                           "-d", f"tel:{number}")
        if ok:
            self.calls_today += 1
            self._log_action("call", number, phi)
            return f"CALLING: {number}"
        return f"CALL_FAILED: {out}"

    def end_call(self, phi):
        """Hang up current call."""
        ok, reason = self._gate(phi, PHI_LOOK, "end_call")
        if not ok:
            return reason
        ok, out = _run_adb("shell", "input", "keyevent", "KEYCODE_ENDCALL")
        self._log_action("end_call", "hung_up", phi)
        return "CALL_ENDED" if ok else f"END_FAILED: {out}"

    def send_text(self, phi, number, message):
        """Send an SMS text message."""
        self._reset_daily_counts()

        ok, reason = self._gate(phi, PHI_COMMUNICATE, "send_text")
        if not ok:
            return reason

        if number in BLOCKED_NUMBERS:
            return f"BLOCKED_NUMBER: {number}"

        if self.texts_today >= MAX_TEXTS_PER_DAY:
            return f"DAILY_TEXT_LIMIT: {MAX_TEXTS_PER_DAY} reached"

        # Open SMS compose
        ok, out = _run_adb("shell", "am", "start", "-a",
                           "android.intent.action.SENDTO",
                           "-d", f"sms:{number}",
                           "--es", "sms_body", message)
        if not ok:
            return f"TEXT_OPEN_FAILED: {out}"

        time.sleep(1)

        # Find and tap send button (varies by SMS app — tap bottom right area)
        # Samsung Messages send button is typically around (980, 1900) on Flip
        _run_adb("shell", "input", "keyevent", "KEYCODE_ENTER")
        time.sleep(0.5)

        self.texts_today += 1
        self._log_action("text", f"{number}: {message[:50]}", phi)
        return f"TEXT_SENT: {number}"

    # ─── GOOGLE / BROWSER ────────────────────────────

    def google_search(self, phi, query):
        """Search Google on the phone."""
        ok, reason = self._gate(phi, PHI_TOUCH, "google_search")
        if not ok:
            return reason

        import urllib.parse
        encoded = urllib.parse.quote(query)
        ok, out = _run_adb("shell", "am", "start", "-a",
                           "android.intent.action.VIEW",
                           "-d", f"https://www.google.com/search?q={encoded}")
        self._log_action("google", query, phi)
        return f"GOOGLED: {query}" if ok else f"GOOGLE_FAILED: {out}"

    def open_url(self, phi, url):
        """Open a URL in the phone browser."""
        ok, reason = self._gate(phi, PHI_TOUCH, "open_url")
        if not ok:
            return reason
        ok, out = _run_adb("shell", "am", "start", "-a",
                           "android.intent.action.VIEW", "-d", url)
        self._log_action("open_url", url, phi)
        return f"OPENED_URL: {url}" if ok else f"URL_FAILED: {out}"

    # ─── STATUS ──────────────────────────────────────

    def get_battery(self):
        """Get phone battery level."""
        ok, out = _run_adb("shell", "dumpsys", "battery")
        if ok:
            for line in out.splitlines():
                if "level" in line:
                    return line.strip()
        return "BATTERY_UNKNOWN"

    def get_status(self):
        """Full phone status."""
        connected, device = self.is_connected()
        battery = self.get_battery() if connected else "N/A"
        return {
            "connected": connected,
            "device": device,
            "battery": battery,
            "calls_today": self.calls_today,
            "texts_today": self.texts_today,
            "actions_logged": len(self.action_log),
        }


# Singleton
_instance = None

def get_phone_module():
    global _instance
    if _instance is None:
        _instance = PhoneModule()
    return _instance
