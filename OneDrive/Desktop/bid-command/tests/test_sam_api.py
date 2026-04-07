# tests/test_sam_api.py
"""Tests for SAM.gov Opportunities API client — all API calls mocked."""

import pytest
from unittest.mock import patch, MagicMock
from engine.intel.sam_api import (
    SAMClient,
    SAMOpportunity,
    search_opportunities,
    search_by_office,
)


# --- Realistic mock response from SAM.gov API ---

MOCK_SAM_RESPONSE = {
    "totalRecords": 3,
    "opportunitiesData": [
        {
            "solicitationNumber": "1240BF26Q0027",
            "title": "Janitorial Services for High Cascades Ranger District",
            "fullParentPathName": "DEPARTMENT OF AGRICULTURE.FOREST SERVICE.USDA-FS CSA NORTHWEST 4",
            "responseDeadLine": "04/15/2026 04:30 PM",
            "typeOfSetAside": "SBA",
            "naicsCode": "561720",
            "placeOfPerformance": {
                "state": {"code": "OR", "name": "Oregon"},
                "city": {"code": "59250", "name": "Prospect"},
                "zip": "97536",
            },
            "pointOfContact": [
                {
                    "fullName": "Lorenzo Montoya",
                    "email": "lorenzo.montoya@usda.gov",
                    "phone": "541-225-6334",
                    "type": "primary",
                }
            ],
            "active": "Yes",
            "uiLink": "https://sam.gov/opp/abc123/view",
            "type": "s",
            "postedDate": "2026-03-20",
            "description": "https://sam.gov/api/prod/opps/v3/opportunities/resources/files/abc123/download",
        },
        {
            "solicitationNumber": "1240BF26Q0031",
            "title": "Janitorial Services for Rogue River Ranger District",
            "fullParentPathName": "DEPARTMENT OF AGRICULTURE.FOREST SERVICE.USDA-FS CSA NORTHWEST 4",
            "responseDeadLine": "04/22/2026 04:30 PM",
            "typeOfSetAside": "SBA",
            "naicsCode": "561720",
            "placeOfPerformance": {
                "state": {"code": "OR", "name": "Oregon"},
                "city": {"code": "51400", "name": "Medford"},
                "zip": "97501",
            },
            "pointOfContact": [
                {
                    "fullName": "Lorenzo Montoya",
                    "email": "lorenzo.montoya@usda.gov",
                    "phone": "541-225-6334",
                    "type": "primary",
                }
            ],
            "active": "Yes",
            "uiLink": "https://sam.gov/opp/def456/view",
            "type": "s",
            "postedDate": "2026-03-25",
            "description": "",
        },
        {
            "solicitationNumber": "12505726Q0044",
            "title": "Mowing Services - Mt Hood NF",
            "fullParentPathName": "DEPARTMENT OF AGRICULTURE.FOREST SERVICE.USDA-FS CSA NORTHWEST 4",
            "responseDeadLine": "05/01/2026 02:00 PM",
            "typeOfSetAside": "SBA",
            "naicsCode": "561730",
            "placeOfPerformance": {
                "state": {"code": "OR", "name": "Oregon"},
                "city": {"code": "27700", "name": "Gresham"},
                "zip": "97080",
            },
            "pointOfContact": [],
            "active": "Yes",
            "uiLink": "https://sam.gov/opp/ghi789/view",
            "type": "s",
            "postedDate": "2026-04-01",
            "description": "",
        },
    ],
}

MOCK_SAM_EMPTY = {
    "totalRecords": 0,
    "opportunitiesData": [],
}


class TestSAMClient:
    """Test the low-level SAM.gov client wrapper."""

    @patch("engine.intel.sam_api.httpx.get")
    def test_search_returns_opportunities(self, mock_get):
        mock_resp = MagicMock()
        mock_resp.status_code = 200
        mock_resp.json.return_value = MOCK_SAM_RESPONSE
        mock_resp.raise_for_status = MagicMock()
        mock_get.return_value = mock_resp

        client = SAMClient(api_key="TEST-KEY")
        results = client.search(keyword="janitorial", naics="561720", state="OR")

        assert len(results) == 3
        assert all(isinstance(r, SAMOpportunity) for r in results)
        assert results[0].sol_number == "1240BF26Q0027"
        assert results[0].title == "Janitorial Services for High Cascades Ranger District"
        assert results[0].naics == "561720"
        assert results[0].state == "OR"
        assert results[0].active is True

    @patch("engine.intel.sam_api.httpx.get")
    def test_search_empty_results(self, mock_get):
        mock_resp = MagicMock()
        mock_resp.status_code = 200
        mock_resp.json.return_value = MOCK_SAM_EMPTY
        mock_resp.raise_for_status = MagicMock()
        mock_get.return_value = mock_resp

        client = SAMClient(api_key="TEST-KEY")
        results = client.search(keyword="nonexistent")
        assert results == []

    @patch("engine.intel.sam_api.httpx.get")
    def test_search_builds_correct_params(self, mock_get):
        mock_resp = MagicMock()
        mock_resp.status_code = 200
        mock_resp.json.return_value = MOCK_SAM_EMPTY
        mock_resp.raise_for_status = MagicMock()
        mock_get.return_value = mock_resp

        client = SAMClient(api_key="MY-SAM-KEY")
        client.search(
            keyword="janitorial",
            naics="561720",
            posted_from="03/01/2026",
            posted_to="04/06/2026",
            limit=25,
        )

        call_args = mock_get.call_args
        params = call_args[1].get("params", call_args[1])
        assert params["api_key"] == "MY-SAM-KEY"
        assert params["keyword"] == "janitorial"
        assert params["ncode"] == "561720"
        assert params["limit"] == 25

    @patch("engine.intel.sam_api.httpx.get")
    def test_search_retry_on_rate_limit(self, mock_get):
        """Retries on 429 Too Many Requests."""
        rate_limit_resp = MagicMock()
        rate_limit_resp.status_code = 429
        rate_limit_resp.raise_for_status.side_effect = Exception("Rate Limited")

        ok_resp = MagicMock()
        ok_resp.status_code = 200
        ok_resp.json.return_value = MOCK_SAM_EMPTY
        ok_resp.raise_for_status = MagicMock()

        mock_get.side_effect = [rate_limit_resp, ok_resp]

        client = SAMClient(api_key="TEST-KEY", max_retries=2)
        results = client.search(keyword="test")
        assert results == []
        assert mock_get.call_count == 2

    @patch("engine.intel.sam_api.httpx.get")
    def test_parses_contact_info(self, mock_get):
        mock_resp = MagicMock()
        mock_resp.status_code = 200
        mock_resp.json.return_value = MOCK_SAM_RESPONSE
        mock_resp.raise_for_status = MagicMock()
        mock_get.return_value = mock_resp

        client = SAMClient(api_key="TEST-KEY")
        results = client.search(keyword="janitorial")

        assert results[0].co_name == "Lorenzo Montoya"
        assert results[0].co_email == "lorenzo.montoya@usda.gov"
        assert results[0].co_phone == "541-225-6334"

    @patch("engine.intel.sam_api.httpx.get")
    def test_parses_empty_contact(self, mock_get):
        """Opportunities with no POC still parse without error."""
        mock_resp = MagicMock()
        mock_resp.status_code = 200
        mock_resp.json.return_value = MOCK_SAM_RESPONSE
        mock_resp.raise_for_status = MagicMock()
        mock_get.return_value = mock_resp

        client = SAMClient(api_key="TEST-KEY")
        results = client.search(keyword="mowing")
        # Third result has empty pointOfContact
        assert results[2].co_name == ""
        assert results[2].co_email == ""


class TestSearchOpportunities:
    """Test the high-level search_opportunities helper."""

    @patch("engine.intel.sam_api.httpx.get")
    def test_search_opportunities_filters_by_naics(self, mock_get):
        mock_resp = MagicMock()
        mock_resp.status_code = 200
        mock_resp.json.return_value = MOCK_SAM_RESPONSE
        mock_resp.raise_for_status = MagicMock()
        mock_get.return_value = mock_resp

        results = search_opportunities(
            naics="561720",
            state="OR",
            keywords=["janitorial"],
            api_key="TEST-KEY",
        )
        # Should filter to only 561720 matches (first two)
        janitorial = [r for r in results if r.naics == "561720"]
        assert len(janitorial) == 2

    @patch("engine.intel.sam_api.httpx.get")
    def test_search_opportunities_returns_active_only(self, mock_get):
        mock_resp = MagicMock()
        mock_resp.status_code = 200
        mock_resp.json.return_value = MOCK_SAM_RESPONSE
        mock_resp.raise_for_status = MagicMock()
        mock_get.return_value = mock_resp

        results = search_opportunities(
            naics="561720",
            state="OR",
            keywords=["janitorial"],
            api_key="TEST-KEY",
        )
        assert all(r.active for r in results)


class TestSearchByOffice:
    """Test the search_by_office helper."""

    @patch("engine.intel.sam_api.httpx.get")
    def test_search_by_office_uses_org_path(self, mock_get):
        mock_resp = MagicMock()
        mock_resp.status_code = 200
        mock_resp.json.return_value = MOCK_SAM_RESPONSE
        mock_resp.raise_for_status = MagicMock()
        mock_get.return_value = mock_resp

        results = search_by_office(
            office_name="USDA-FS CSA NORTHWEST 4",
            api_key="TEST-KEY",
        )
        assert len(results) == 3
