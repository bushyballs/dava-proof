# tests/test_sub_finder.py
"""Tests for local subcontractor finder — all external calls mocked."""

import sqlite3
import json
import pytest
from pathlib import Path
from unittest.mock import patch, MagicMock
from engine.intel.sub_finder import (
    SubFinder,
    Subcontractor,
    SubDB,
    draft_outreach_email,
)


# --- Realistic mock response from Google Places API ---

MOCK_PLACES_RESPONSE = {
    "results": [
        {
            "name": "Southwest Janitorial Services",
            "formatted_address": "123 Main St, Prospect, OR 97536",
            "formatted_phone_number": "(541) 555-0101",
            "geometry": {"location": {"lat": 42.75, "lng": -122.49}},
            "rating": 4.5,
            "types": ["point_of_interest", "establishment"],
            "place_id": "ChIJ_test_place_1",
            "business_status": "OPERATIONAL",
        },
        {
            "name": "Rogue Valley Cleaning Co",
            "formatted_address": "456 Oak Ave, Medford, OR 97501",
            "formatted_phone_number": "(541) 555-0202",
            "geometry": {"location": {"lat": 42.33, "lng": -122.87}},
            "rating": 4.2,
            "types": ["point_of_interest", "establishment"],
            "place_id": "ChIJ_test_place_2",
            "business_status": "OPERATIONAL",
        },
    ],
    "status": "OK",
}

MOCK_PLACES_EMPTY = {
    "results": [],
    "status": "ZERO_RESULTS",
}

MOCK_PLACE_DETAILS = {
    "result": {
        "name": "Southwest Janitorial Services",
        "formatted_address": "123 Main St, Prospect, OR 97536",
        "formatted_phone_number": "(541) 555-0101",
        "website": "https://swjanitorial.example.com",
        "opening_hours": {"open_now": True},
    },
    "status": "OK",
}


class TestSubFinder:
    """Test the SubFinder search client."""

    @patch("engine.intel.sub_finder.httpx.get")
    def test_find_local_subs_returns_results(self, mock_get):
        mock_resp = MagicMock()
        mock_resp.status_code = 200
        mock_resp.json.return_value = MOCK_PLACES_RESPONSE
        mock_resp.raise_for_status = MagicMock()
        mock_get.return_value = mock_resp

        finder = SubFinder(google_api_key="TEST-KEY")
        results = finder.find_local_subs(
            city="Prospect",
            state="OR",
            service_type="janitorial",
        )

        assert len(results) == 2
        assert all(isinstance(r, Subcontractor) for r in results)
        assert results[0].name == "Southwest Janitorial Services"
        assert results[0].address == "123 Main St, Prospect, OR 97536"
        assert results[0].phone == "(541) 555-0101"

    @patch("engine.intel.sub_finder.httpx.get")
    def test_find_local_subs_empty(self, mock_get):
        mock_resp = MagicMock()
        mock_resp.status_code = 200
        mock_resp.json.return_value = MOCK_PLACES_EMPTY
        mock_resp.raise_for_status = MagicMock()
        mock_get.return_value = mock_resp

        finder = SubFinder(google_api_key="TEST-KEY")
        results = finder.find_local_subs(
            city="Nowhere",
            state="XX",
            service_type="janitorial",
        )
        assert results == []

    @patch("engine.intel.sub_finder.httpx.get")
    def test_find_local_subs_builds_correct_query(self, mock_get):
        mock_resp = MagicMock()
        mock_resp.status_code = 200
        mock_resp.json.return_value = MOCK_PLACES_EMPTY
        mock_resp.raise_for_status = MagicMock()
        mock_get.return_value = mock_resp

        finder = SubFinder(google_api_key="MY-KEY")
        finder.find_local_subs(
            city="Medford",
            state="OR",
            service_type="landscaping",
        )

        call_args = mock_get.call_args
        params = call_args[1].get("params", {})
        assert "MY-KEY" in params.get("key", "")
        assert "landscaping" in params.get("query", "").lower()
        assert "Medford" in params.get("query", "")

    @patch("engine.intel.sub_finder.httpx.get")
    def test_get_details_enriches_sub(self, mock_get):
        """get_details should add website and hours."""
        # First call = search, second call = details
        search_resp = MagicMock()
        search_resp.status_code = 200
        search_resp.json.return_value = MOCK_PLACES_RESPONSE
        search_resp.raise_for_status = MagicMock()

        details_resp = MagicMock()
        details_resp.status_code = 200
        details_resp.json.return_value = MOCK_PLACE_DETAILS
        details_resp.raise_for_status = MagicMock()

        mock_get.side_effect = [search_resp, details_resp]

        finder = SubFinder(google_api_key="TEST-KEY")
        subs = finder.find_local_subs(city="Prospect", state="OR", service_type="janitorial")

        enriched = finder.get_details(subs[0])
        assert enriched.website == "https://swjanitorial.example.com"


class TestSubDB:
    """Test the SQLite subcontractor database."""

    def test_save_and_load(self, tmp_path):
        db = SubDB(tmp_path / "subs.db")
        sub = Subcontractor(
            name="Test Cleaning",
            address="100 Test Ln, Test, OR 97000",
            phone="(555) 555-5555",
            service_type="janitorial",
            state="OR",
            city="Test",
        )
        db.save_sub(sub)
        results = db.search_subs(state="OR", service_type="janitorial")
        assert len(results) == 1
        assert results[0].name == "Test Cleaning"

    def test_no_duplicates(self, tmp_path):
        db = SubDB(tmp_path / "subs.db")
        sub = Subcontractor(
            name="Test Cleaning",
            address="100 Test Ln, Test, OR 97000",
            phone="(555) 555-5555",
            service_type="janitorial",
            state="OR",
            city="Test",
        )
        db.save_sub(sub)
        db.save_sub(sub)  # same sub again
        results = db.search_subs(state="OR")
        assert len(results) == 1

    def test_search_by_city(self, tmp_path):
        db = SubDB(tmp_path / "subs.db")
        db.save_sub(Subcontractor(name="A", address="", phone="", service_type="janitorial", state="OR", city="Medford"))
        db.save_sub(Subcontractor(name="B", address="", phone="", service_type="janitorial", state="OR", city="Prospect"))
        results = db.search_subs(city="Medford")
        assert len(results) == 1
        assert results[0].name == "A"

    def test_list_all_subs(self, tmp_path):
        db = SubDB(tmp_path / "subs.db")
        db.save_sub(Subcontractor(name="A", address="", phone="", service_type="janitorial", state="OR", city="Medford"))
        db.save_sub(Subcontractor(name="B", address="", phone="", service_type="mowing", state="NM", city="Taos"))
        results = db.list_all()
        assert len(results) == 2


class TestOutreachEmail:
    """Test outreach email template generation."""

    def test_draft_outreach_email(self):
        sub = Subcontractor(
            name="Southwest Janitorial Services",
            address="123 Main St, Prospect, OR 97536",
            phone="(541) 555-0101",
            service_type="janitorial",
            state="OR",
            city="Prospect",
        )
        email = draft_outreach_email(
            sub=sub,
            solicitation_title="Janitorial Services for High Cascades RD",
            solicitation_number="1240BF26Q0027",
            company_name="Hoags Inc.",
        )
        assert "Southwest Janitorial Services" in email
        assert "1240BF26Q0027" in email
        assert "Hoags Inc." in email
        assert "janitorial" in email.lower()

    def test_draft_outreach_has_required_sections(self):
        sub = Subcontractor(
            name="Test Co",
            address="123 Test",
            phone="555-5555",
            service_type="mowing",
            state="NM",
            city="Taos",
        )
        email = draft_outreach_email(
            sub=sub,
            solicitation_title="Mowing Services",
            solicitation_number="TEST001",
            company_name="Hoags Inc.",
        )
        # Must have subject line, greeting, scope mention, and sign-off
        assert "Subject:" in email
        assert "Hoags Inc." in email
        assert len(email) > 100
