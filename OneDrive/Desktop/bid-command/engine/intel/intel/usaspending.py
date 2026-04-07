# engine/intel/usaspending.py
"""USASpending.gov API client — find incumbents and comparable awards."""

from __future__ import annotations

import time
from dataclasses import dataclass, field
from datetime import datetime

import httpx

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

BASE_URL = "https://api.usaspending.gov/api/v2/search/spending_by_award/"

# Fields we request from the API
_FIELDS = [
    "Recipient Name",
    "Award Amount",
    "Start Date",
    "End Date",
    "Description",
    "Award ID",
    "Awarding Agency",
    "Awarding Sub Agency",
    "Place of Performance State Code",
    "Place of Performance City",
]

# Contract award type codes: A=BPA Call, B=Purchase Order, C=Delivery Order, D=Definitive Contract
_AWARD_TYPE_CODES = ["A", "B", "C", "D"]

TIMEOUT = 30.0  # seconds


# ---------------------------------------------------------------------------
# Data classes
# ---------------------------------------------------------------------------

@dataclass
class AwardResult:
    """A single federal contract award from USASpending."""

    recipient_name: str = ""
    award_amount: float = 0.0
    start_date: str = ""
    end_date: str = ""
    description: str = ""
    award_id: str = ""
    agency: str = ""
    sub_agency: str = ""
    state: str = ""
    city: str = ""
    annual_rate: float = 0.0  # computed: award_amount / years

    def __post_init__(self) -> None:
        if self.award_amount > 0 and self.start_date and self.end_date:
            self.annual_rate = _compute_annual_rate(
                self.award_amount, self.start_date, self.end_date,
            )


def _compute_annual_rate(amount: float, start: str, end: str) -> float:
    """Compute annual rate from total award amount and date range.

    Handles dates in YYYY-MM-DD format. Returns the full amount if
    the period is less than 1 year.
    """
    try:
        s = datetime.strptime(start, "%Y-%m-%d")
        e = datetime.strptime(end, "%Y-%m-%d")
        days = (e - s).days
        if days <= 0:
            return amount
        years = days / 365.25
        if years < 1.0:
            return amount
        return round(amount / years, 2)
    except (ValueError, TypeError):
        return amount


def _parse_result(raw: dict) -> AwardResult:
    """Convert a raw USASpending result dict to an AwardResult."""
    return AwardResult(
        recipient_name=raw.get("Recipient Name", "") or "",
        award_amount=float(raw.get("Award Amount", 0) or 0),
        start_date=raw.get("Start Date", "") or "",
        end_date=raw.get("End Date", "") or "",
        description=raw.get("Description", "") or "",
        award_id=raw.get("Award ID", "") or "",
        agency=raw.get("Awarding Agency", "") or "",
        sub_agency=raw.get("Awarding Sub Agency", "") or "",
        state=raw.get("Place of Performance State Code", "") or "",
        city=raw.get("Place of Performance City", "") or "",
    )


# ---------------------------------------------------------------------------
# Client
# ---------------------------------------------------------------------------

class USASpendingClient:
    """Low-level wrapper around the USASpending spending_by_award endpoint.

    Features:
        - Structured request building with filters
        - Retry on transient HTTP errors (500, 502, 503, 504)
        - Configurable timeout and retry count
    """

    def __init__(
        self,
        base_url: str = BASE_URL,
        timeout: float = TIMEOUT,
        max_retries: int = 3,
    ) -> None:
        self.base_url = base_url
        self.timeout = timeout
        self.max_retries = max_retries

    def _build_payload(
        self,
        keywords: list[str],
        state: str = "",
        naics: str = "",
        date_range: tuple[str, str] | None = None,
        limit: int = 50,
        page: int = 1,
    ) -> dict:
        """Build the POST JSON body for spending_by_award search."""
        filters: dict = {
            "keywords": keywords,
            "award_type_codes": _AWARD_TYPE_CODES,
        }

        if naics:
            filters["naics_codes"] = {"require": [naics]}

        if state:
            filters["place_of_performance_locations"] = [
                {"country": "USA", "state": state}
            ]

        if date_range:
            filters["time_period"] = [
                {"start_date": date_range[0], "end_date": date_range[1]}
            ]

        return {
            "filters": filters,
            "fields": _FIELDS,
            "limit": limit,
            "page": page,
            "sort": "Start Date",
            "order": "desc",
            "subawards": False,
        }

    def search_awards(
        self,
        keywords: list[str],
        state: str = "",
        naics: str = "",
        date_range: tuple[str, str] | None = None,
        limit: int = 50,
        page: int = 1,
    ) -> list[AwardResult]:
        """Search USASpending for contract awards matching the given filters.

        Args:
            keywords: Search terms (e.g. ["janitorial", "High Cascades"]).
            state: Two-letter state code (e.g. "OR").
            naics: NAICS code to add to keywords if provided.
            date_range: Optional (start_date, end_date) in YYYY-MM-DD format.
            limit: Max results per page (1-100).
            page: Page number for pagination.

        Returns:
            List of AwardResult dataclasses, sorted by start_date descending.
        """
        payload = self._build_payload(
            keywords=keywords,
            state=state,
            naics=naics,
            date_range=date_range,
            limit=limit,
            page=page,
        )

        for attempt in range(self.max_retries):
            try:
                resp = httpx.post(
                    self.base_url,
                    json=payload,
                    timeout=self.timeout,
                )
                resp.raise_for_status()
                data = resp.json()
                raw_results = data.get("results", [])
                return [_parse_result(r) for r in raw_results]

            except Exception:
                if attempt < self.max_retries - 1:
                    time.sleep(1.0 * (attempt + 1))
                    continue
                return []

        return []


# ---------------------------------------------------------------------------
# High-level helpers
# ---------------------------------------------------------------------------

def find_incumbent(
    agency: str,
    location: str,
    keywords: list[str],
    client: USASpendingClient | None = None,
) -> AwardResult | None:
    """Find the most recent incumbent contractor for a given contract.

    Searches USASpending for the most recent award matching the agency,
    location, and keywords. Returns the top result (most recent start date)
    or None if no matches found.

    Args:
        agency: Awarding agency name (e.g. "Forest Service").
        location: "City, ST" format (e.g. "Prospect, OR").
        keywords: Search terms from the solicitation.
        client: Optional pre-configured client instance.

    Returns:
        The most recent AwardResult, or None.
    """
    if client is None:
        client = USASpendingClient()

    # Extract state from "City, ST" format
    state = ""
    if "," in location:
        parts = location.split(",")
        state = parts[-1].strip()[:2].upper()

    # Combine agency + keywords for broader search
    search_terms = [agency] + keywords

    results = client.search_awards(
        keywords=search_terms,
        state=state,
        limit=10,
    )

    if not results:
        return None

    # Sort by start_date descending, return most recent
    results.sort(key=lambda r: r.start_date, reverse=True)
    return results[0]


def find_comps(
    naics: str,
    state: str,
    keywords: list[str],
    limit: int = 20,
    client: USASpendingClient | None = None,
) -> list[AwardResult]:
    """Find comparable contract awards in the same area and NAICS code.

    Args:
        naics: NAICS code (e.g. "561720").
        state: Two-letter state code.
        keywords: Additional search terms.
        limit: Max results to return.
        client: Optional pre-configured client instance.

    Returns:
        List of AwardResult sorted by start_date descending, each with
        annual_rate computed.
    """
    if client is None:
        client = USASpendingClient()

    results = client.search_awards(
        keywords=keywords,
        state=state,
        naics=naics,
        limit=limit,
    )

    # Sort by start_date descending (most recent first)
    results.sort(key=lambda r: r.start_date, reverse=True)
    return results
