# engine/intel/sub_finder.py
"""Local subcontractor finder — search for businesses near a job site.

Uses Google Places API (Text Search) to find local contractors matching
a service type (janitorial, landscaping, mowing, etc.) and stores results
in a local SQLite database for reuse across bids.
"""

from __future__ import annotations

import sqlite3
import time
from dataclasses import dataclass, field
from pathlib import Path

import httpx


# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

PLACES_SEARCH_URL = "https://maps.googleapis.com/maps/api/place/textsearch/json"
PLACES_DETAILS_URL = "https://maps.googleapis.com/maps/api/place/details/json"
TIMEOUT = 15.0

# Service type aliases for better search queries
_SERVICE_QUERIES: dict[str, str] = {
    "janitorial": "janitorial cleaning services commercial",
    "landscaping": "landscaping lawn care services commercial",
    "mowing": "mowing lawn maintenance services",
    "custodial": "custodial cleaning services",
    "carpet": "carpet cleaning services commercial",
    "pest": "pest control services commercial",
    "snow": "snow removal plowing services",
    "trash": "trash waste removal services",
}


# ---------------------------------------------------------------------------
# Data classes
# ---------------------------------------------------------------------------

@dataclass
class Subcontractor:
    """A potential local subcontractor."""

    name: str = ""
    address: str = ""
    phone: str = ""
    service_type: str = ""
    state: str = ""
    city: str = ""
    website: str = ""
    rating: float = 0.0
    place_id: str = ""
    email: str = ""     # rarely available from Places API


# ---------------------------------------------------------------------------
# SubFinder — Google Places client
# ---------------------------------------------------------------------------

class SubFinder:
    """Find local subcontractors using Google Places API.

    Features:
        - Text search for businesses by service type and location
        - Detail enrichment (website, hours)
        - Service type aliases for better results
    """

    def __init__(
        self,
        google_api_key: str = "",
        timeout: float = TIMEOUT,
        max_retries: int = 2,
    ) -> None:
        self.api_key = google_api_key
        self.timeout = timeout
        self.max_retries = max_retries

    def find_local_subs(
        self,
        city: str,
        state: str,
        service_type: str,
        radius_miles: int = 50,
    ) -> list[Subcontractor]:
        """Search for local subcontractors matching the service type.

        Args:
            city: City name (e.g. "Prospect").
            state: Two-letter state code (e.g. "OR").
            service_type: Service category (e.g. "janitorial", "mowing").
            radius_miles: Search radius in miles (converted to meters).

        Returns:
            List of Subcontractor results.
        """
        # Build search query
        query_base = _SERVICE_QUERIES.get(
            service_type.lower(),
            f"{service_type} services commercial",
        )
        query = f"{query_base} in {city}, {state}"

        params = {
            "query": query,
            "key": self.api_key,
            "radius": radius_miles * 1609,  # miles to meters
        }

        for attempt in range(self.max_retries):
            try:
                resp = httpx.get(
                    PLACES_SEARCH_URL,
                    params=params,
                    timeout=self.timeout,
                )
                resp.raise_for_status()
                data = resp.json()

                if data.get("status") not in ("OK", "ZERO_RESULTS"):
                    if attempt < self.max_retries - 1:
                        time.sleep(1.0)
                        continue
                    return []

                raw_results = data.get("results", [])
                return [
                    self._parse_place(r, service_type, state, city)
                    for r in raw_results
                    if r.get("business_status", "OPERATIONAL") == "OPERATIONAL"
                ]

            except Exception:
                if attempt < self.max_retries - 1:
                    time.sleep(1.0)
                    continue
                return []

        return []

    def _parse_place(
        self,
        raw: dict,
        service_type: str,
        state: str,
        city: str,
    ) -> Subcontractor:
        """Convert a raw Places API result to a Subcontractor."""
        return Subcontractor(
            name=raw.get("name", "") or "",
            address=raw.get("formatted_address", "") or "",
            phone=raw.get("formatted_phone_number", "") or "",
            service_type=service_type,
            state=state,
            city=city,
            rating=float(raw.get("rating", 0) or 0),
            place_id=raw.get("place_id", "") or "",
        )

    def get_details(self, sub: Subcontractor) -> Subcontractor:
        """Enrich a Subcontractor with details from the Places Details API.

        Adds website and other data not available from text search.

        Args:
            sub: Subcontractor with place_id set.

        Returns:
            Enriched Subcontractor (same object, mutated).
        """
        if not sub.place_id:
            return sub

        params = {
            "place_id": sub.place_id,
            "fields": "name,formatted_address,formatted_phone_number,website,opening_hours",
            "key": self.api_key,
        }

        try:
            resp = httpx.get(
                PLACES_DETAILS_URL,
                params=params,
                timeout=self.timeout,
            )
            resp.raise_for_status()
            data = resp.json()
            result = data.get("result", {})

            sub.website = result.get("website", "") or ""
            if result.get("formatted_phone_number"):
                sub.phone = result["formatted_phone_number"]

            return sub

        except Exception:
            return sub


# ---------------------------------------------------------------------------
# SubDB — SQLite persistence
# ---------------------------------------------------------------------------

class SubDB:
    """SQLite database for storing found subcontractors.

    Builds a local database over time, so we don't re-search for
    subs we've already found. Deduplicates by name + address.
    """

    def __init__(self, db_path: Path) -> None:
        self.db_path = db_path
        db_path.parent.mkdir(parents=True, exist_ok=True)
        self._init_schema()

    def _connect(self) -> sqlite3.Connection:
        conn = sqlite3.connect(str(self.db_path))
        conn.row_factory = sqlite3.Row
        conn.execute("PRAGMA journal_mode=WAL")
        return conn

    def _init_schema(self) -> None:
        conn = self._connect()
        conn.execute("""
            CREATE TABLE IF NOT EXISTS subs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                address TEXT DEFAULT '',
                phone TEXT DEFAULT '',
                service_type TEXT DEFAULT '',
                state TEXT DEFAULT '',
                city TEXT DEFAULT '',
                website TEXT DEFAULT '',
                rating REAL DEFAULT 0.0,
                place_id TEXT DEFAULT '',
                email TEXT DEFAULT '',
                created_at TEXT DEFAULT (datetime('now')),
                UNIQUE(name, address)
            )
        """)
        conn.commit()
        conn.close()

    def save_sub(self, sub: Subcontractor) -> None:
        """Save a subcontractor to the database. Skips duplicates."""
        conn = self._connect()
        try:
            conn.execute(
                """INSERT OR IGNORE INTO subs
                   (name, address, phone, service_type, state, city, website, rating, place_id, email)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)""",
                (
                    sub.name, sub.address, sub.phone, sub.service_type,
                    sub.state, sub.city, sub.website, sub.rating,
                    sub.place_id, sub.email,
                ),
            )
            conn.commit()
        finally:
            conn.close()

    def save_many(self, subs: list[Subcontractor]) -> None:
        """Save multiple subcontractors."""
        for sub in subs:
            self.save_sub(sub)

    def search_subs(
        self,
        state: str = "",
        city: str = "",
        service_type: str = "",
    ) -> list[Subcontractor]:
        """Search stored subcontractors by location and service type."""
        conn = self._connect()
        query = "SELECT * FROM subs WHERE 1=1"
        params: list = []

        if state:
            query += " AND state = ?"
            params.append(state)
        if city:
            query += " AND city = ?"
            params.append(city)
        if service_type:
            query += " AND service_type = ?"
            params.append(service_type)

        query += " ORDER BY rating DESC, name ASC"
        rows = conn.execute(query, params).fetchall()
        conn.close()
        return [self._row_to_sub(r) for r in rows]

    def list_all(self) -> list[Subcontractor]:
        """List all stored subcontractors."""
        conn = self._connect()
        rows = conn.execute("SELECT * FROM subs ORDER BY state, city, name").fetchall()
        conn.close()
        return [self._row_to_sub(r) for r in rows]

    def _row_to_sub(self, row: sqlite3.Row) -> Subcontractor:
        return Subcontractor(
            name=row["name"],
            address=row["address"],
            phone=row["phone"],
            service_type=row["service_type"],
            state=row["state"],
            city=row["city"],
            website=row["website"],
            rating=row["rating"],
            place_id=row["place_id"],
            email=row["email"],
        )


# ---------------------------------------------------------------------------
# Outreach Email Template
# ---------------------------------------------------------------------------

def draft_outreach_email(
    sub: Subcontractor,
    solicitation_title: str,
    solicitation_number: str,
    company_name: str = "Hoags Inc.",
) -> str:
    """Generate a subcontractor outreach email template.

    Args:
        sub: The subcontractor to reach out to.
        solicitation_title: Title of the solicitation.
        solicitation_number: Sol number (e.g. "1240BF26Q0027").
        company_name: Prime contractor company name.

    Returns:
        Formatted email string with Subject, body, and sign-off.
    """
    service = sub.service_type or "services"

    return f"""Subject: Subcontracting Opportunity - {solicitation_number} {service.title()} Services

Dear {sub.name},

My name is Collin Hoag with {company_name}. We are preparing a proposal for the following federal contract opportunity and are looking for qualified local {service} subcontractors in the {sub.city}, {sub.state} area:

    Solicitation: {solicitation_number}
    Title: {solicitation_title}
    Service: {service.title()}
    Location: {sub.city}, {sub.state}

We found your business and believe you may be a good fit for this project. If you are interested in discussing a potential subcontracting arrangement, please reply to this email or call at your earliest convenience.

We would need:
    - Your company name, address, and point of contact
    - Relevant experience and references
    - Approximate pricing for the scope described above
    - Any applicable certifications (small business, SDVOSB, HUBZone, etc.)

Thank you for your time. We look forward to hearing from you.

Best regards,
Collin Hoag
President, {company_name}
collinhoag@hoagsandfamily.com
(458) 239-3215
"""
