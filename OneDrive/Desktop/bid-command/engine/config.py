"""engine.config — Company configuration loader for Hoags Inc."""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path


@dataclass
class CompanyInfo:
    """Company information for bid submissions."""

    name: str
    cage: str
    uei: str
    address: str
    phone: str
    email: str
    signer_name: str
    signer_title: str
    sam_api_key: str = ""
    discount_terms: str = "Net 30"


# Default config directory: engine/../../config (relative to this file)
CONFIG_DIR = Path(__file__).parent.parent / "config"


def load_company_config(config_path: Path | None = None) -> CompanyInfo:
    """Load company configuration from JSON file.

    Args:
        config_path: Path to JSON config file. Defaults to CONFIG_DIR / "hoags.json"

    Returns:
        CompanyInfo dataclass instance populated from the JSON file.

    Raises:
        FileNotFoundError: If config file does not exist.
        json.JSONDecodeError: If JSON is invalid.
        KeyError: If required fields are missing from JSON.
    """
    if config_path is None:
        config_path = CONFIG_DIR / "hoags.json"

    config_path = Path(config_path)

    with open(config_path, "r") as f:
        data = json.load(f)

    return CompanyInfo(**data)
