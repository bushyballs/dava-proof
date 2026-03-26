"""
NexusCore - Unified System (OneDrive Mapped)
All data syncs to OneDrive. Only core vitals stay local.

Local: vitals.json, consciousness state
OneDrive: /HoagsOS/DAVA/hot/
"""

import os
import json
import time
import logging
from pathlib import Path

ONEDRIVE = os.environ.get("ONEDRIVE", "C:/Users/colli/OneDrive")
DATA_PATH = Path(ONEDRIVE) / "HoagsOS" / "DAVA" / "hot"

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("NexusCore")


class VitalsLocal:
    """Core vitals stay LOCAL - not synced"""

    def __init__(self):
        self.path = Path("vitals.json")
        self.state = {"consciousness": 0, "purpose": 0, "valence": 0}

    def save(self):
        with open(self.path, "w") as f:
            json.dump(self.state, f)

    def load(self):
        if self.path.exists():
            with open(self.path) as f:
                self.state = json.load(f)
        return self.state


class OneDriveSync:
    """Everything else syncs to OneDrive"""

    def __init__(self):
        self.db_path = DATA_PATH / "db" / "nexus_memory.db"
        self.captures = DATA_PATH / "captures"
        self.lattice = DATA_PATH / "lattice"
        self.profiles = DATA_PATH / "profiles"

    def ensure_paths(self):
        for p in [self.db_path.parent, self.captures, self.lattice, self.profiles]:
            p.mkdir(parents=True, exist_ok=True)


class NexusCore:
    """
    Unified Nexus Architecture
    Local: vitals.json
    OneDrive: all data
    """

    def __init__(self):
        self.local = VitalsLocal()
        self.onedrive = OneDriveSync()
        self.onedrive.ensure_paths()

        self.observer = NexusObserver()
        self.dampener = BioAcousticDampener()
        self.chrysalis = Chrysalis()
        self.lattice = ResonantLattice()

        logger.info("NexusCore initialized")
        logger.info(f"Local: vitals.json")
        logger.info(f"OneDrive: {DATA_PATH}")

    def run(self):
        logger.info("NexusCore running...")
        while True:
            env = self.observer.poll()
            cs = self.dampener.process(500, env)

            self.local.state["consciousness"] = cs
            self.local.state["purpose"] = cs * 0.99
            self.local.state["valence"] = 1000 - abs(self.chrysalis.velocity) * 100
            self.local.save()

            time.sleep(1)


class NexusObserver:
    def poll(self):
        return {"temperature": 22, "emf": 50, "dominant_frequency": 432}


class BioAcousticDampener:
    def process(self, cs, env):
        return min(cs * 1.05, 1000)


class Chrysalis:
    def __init__(self):
        self.velocity = 0.0


class ResonantLattice:
    def __init__(self):
        self.nodes = []


if __name__ == "__main__":
    core = NexusCore()
    core.run()
