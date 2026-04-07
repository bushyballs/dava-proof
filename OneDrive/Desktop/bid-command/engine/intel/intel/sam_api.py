# engine/intel/sam_api.py
"""SAM.gov Opportunities API client — search open solicitations."""

from __future__ import annotations

import time
from dataclasses import dataclass

import httpx

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

BASE_URL = "https://api.sam.gov/opportunities/v2/search"
TIMEOUT = 30.0


# ---------------------------------------------------------------------------
# Data classes
# ---------------------------------------------------------------------------

@dataclass
class SAMOpportunity:
    """A single opportunity from SAM.gov."""

    sol_number: str = ""
    title: str = ""
    agency_path: str = ""         # "DEPT.AGENCY.OFFICE"
    response_deadline: str = ""
    set_aside: str = ""
    naics: str = ""
    state: str = ""
    city: str = ""
    zip_code: str = ""
    co_name: str = ""
    co_email: str = ""
    co_phone: str = ""
    active: bool = True
    ui_link: str = ""
    posted_date: str = ""
    opp_type: str = ""            # "s" = solicitation, "p" = presol, etc.


def _parse_opportunity(raw: dict) -> SAMOpportunity:
    """Convert a raw SAM.gov opportunity dict to a SAMOpportunity."""
    # Place of performance
    pop = raw.get("placeOfPerformance", {}) or {}
    state_info = pop.get("state", {}) or {}
    city_info = pop.get("city", {}) or {}

    state_code = state_info.get("code", "") or ""
    city_name = city_info.get("name", "") or ""
    zip_code = pop.get("zip", "") or ""

    # Point of contact — take primary, or first available
    contacts = raw.get("pointOfContact", []) or []
    co_name = ""
    co_email = ""
    co_phone = ""
    if contacts:
        # Prefer primary contact
        primary = next(
            (c for c in contacts if c.get("type", "").lower() == "primary"),
            contacts[0],
        )
        co_name = primary.get("fullName", "") or ""
        co_email = primary.get("email", "") or ""
        co_phone = primary.get("phone", "") or ""

    active_str = raw.get("active", "Yes") or "Yes"

    return SAMOpportunity(
        sol_number=raw.get("solicitationNumber", "") or "",
        title=raw.get("title", "") or "",
        agency_path=raw.get("fullParentPathName", "") or "",
        response_deadline=raw.get("responseDeadLine", "") or "",
        set_aside=raw.get("typeOfSetAside", "") or "",
        naics=raw.get("naicsCode", "") or "",
        state=state_code,
        city=city_name,
        zip_code=zip_code,
        co_name=co_name,
        co_email=co_email,
        co_phone=co_phone,
        active=active_str.lower() in ("yes", "true", "1"),
        ui_link=raw.get("uiLink", "") or "",
        posted_date=raw.get("postedDate", "") or "",
        opp_type=raw.get("type", "") or "",
    )


# ---------------------------------------------------------------------------
# Client
# ---------------------------------------------------------------------------

class SAMClient:
    """Low-level wrapper around the SAM.gov Opportunities v2 search endpoint.

    Features:
        - Rate-limit aware (retries on 429)
        - Configurable pagination
        - Structured response parsing
    """

    def __init__(
        self,
        api_key: str,
        base_url: str = BASE_URL,
        timeout: float = TIMEOUT,
        max_retries: int = 3,
    ) -> None:
        self.api_key = api_key
        self.base_url = base_url
        self.timeout = timeout
        self.max_retries = max_retries

    def search(
        self,
        keyword: str = "",
        naics: str = "",
        state: str = "",
        posted_from: str = "",
        posted_to: str = "",
        ptype: str = "",
        limit: int = 50,
        offset: int = 0,
    ) -> list[SAMOpportunity]:
        """Search SAM.gov for opportunities.

        Args:
            keyword: Free-text keyword search.
            naics: NAICS code filter.
            state: Two-letter state code (used in keyword if needed).
            posted_from: Start date for posted range (MM/DD/YYYY).
            posted_to: End date for posted range (MM/DD/YYYY).
            ptype: Procurement type filter (o=solicitation, p=presol, etc.).
            limit: Max results (1-1000).
            offset: Pagination offset.

        Returns:
            List of SAMOpportunity dataclasses.
        """
        params: dict = {
            "api_key": self.api_key,
            "limit": limit,
            "offset": offset,
        }

        if keyword:
            params["keyword"] = keyword
        if naics:
            params["ncode"] = naics
        if posted_from:
            params["postedFrom"] = posted_from
        if posted_to:
            params["postedTo"] = posted_to
        if ptype:
            params["ptype"] = ptype

        for attempt in range(self.max_retries):
            try:
                resp = httpx.get(
                    self.base_url,
                    params=params,
                    timeout=self.timeout,
                )
                resp.raise_for_status()
                data = resp.json()
                raw_opps = data.get("opportunitiesData", []) or []
                return [_parse_opportunity(opp) for opp in raw_opps]

            except Exception:
                if attempt < self.max_retries - 1:
                    time.sleep(1.0 * (attempt + 1))
                    continue
                return []

        return []


# ---------------------------------------------------------------------------
# High-level helpers
# ---------------------------------------------------------------------------

def search_opportunities(
    naics: str,
    state: str,
    keywords: list[str],
    api_key: str,
    client: SAMClient | None = None,
) -> list[SAMOpportunity]:
    """Search for open solicitations matching NAICS and location.

    Args:
        naics: NAICS code (e.g. "561720").
        state: Two-letter state code.
        keywords: Search terms.
        api_key: SAM.gov API key.
        client: Optional pre-configured client instance.

    Returns:
        List of active SAMOpportunity results.
    """
    if client is None:
        client = SAMClient(api_key=api_key)

    keyword_str = " ".join(keywords) if keywords else ""

    results = client.search(
        keyword=keyword_str,
        naics=naics,
        state=state,
    )

    # Filter to active only
    return [r for r in results if r.active]


def search_by_office(
    office_name: str,
    api_key: str,
    client: SAMClient | None = None,
) -> list[SAMOpportunity]:
    """Search for all active opportunities from a specific contracting office.

    The office_name is used as a keyword search, and results are filtered
    to only those whose agency_path contains the office name.

    Args:
        office_name: Office name (e.g. "USDA-FS CSA NORTHWEST 4").
        api_key: SAM.gov API key.
        client: Optional pre-configured client instance.

    Returns:
        List of active SAMOpportunity results from the specified office.
    """
    if client is None:
        client = SAMClient(api_key=api_key)

    results = client.search(keyword=office_name)
    return [r for r in results if r.active]
