# tests/test_usaspending.py
"""Tests for USASpending API client — all API calls mocked."""

import json
import pytest
from unittest.mock import patch, MagicMock
from engine.intel.usaspending import (
    USASpendingClient,
    AwardResult,
    find_incumbent,
    find_comps,
)


# --- Realistic mock response from USASpending API ---

MOCK_SPENDING_RESPONSE = {
    "page_metadata": {"page": 1, "hasNext": False, "total": 2},
    "results": [
        {
            "Recipient Name": "ABC Janitorial Services LLC",
            "Award Amount": 185000.00,
            "Start Date": "2024-05-01",
            "End Date": "2025-04-30",
            "Description": "Janitorial Services for High Cascades Ranger District",
            "Award ID": "1240BF24Q0015",
            "Awarding Agency": "Department of Agriculture",
            "Awarding Sub Agency": "Forest Service",
            "Place of Performance State Code": "OR",
            "Place of Performance City": "Prospect",
        },
        {
            "Recipient Name": "Pacific NW Cleaning Corp",
            "Award Amount": 210000.00,
            "Start Date": "2023-05-01",
            "End Date": "2024-04-30",
            "Description": "Janitorial Services Rogue River-Siskiyou NF",
            "Award ID": "1240BF23Q0009",
            "Awarding Agency": "Department of Agriculture",
            "Awarding Sub Agency": "Forest Service",
            "Place of Performance State Code": "OR",
            "Place of Performance City": "Prospect",
        },
    ],
}

MOCK_EMPTY_RESPONSE = {
    "page_metadata": {"page": 1, "hasNext": False, "total": 0},
    "results": [],
}


class TestUSASpendingClient:
    """Test the low-level client wrapper."""

    @patch("engine.intel.usaspending.httpx.post")
    def test_search_awards_returns_results(self, mock_post):
        mock_resp = MagicMock()
        mock_resp.status_code = 200
        mock_resp.json.return_value = MOCK_SPENDING_RESPONSE
        mock_resp.raise_for_status = MagicMock()
        mock_post.return_value = mock_resp

        client = USASpendingClient()
        results = client.search_awards(
            keywords=["janitorial", "High Cascades"],
            state="OR",
            naics="561720",
        )
        assert len(results) == 2
        assert all(isinstance(r, AwardResult) for r in results)
        assert results[0].recipient_name == "ABC Janitorial Services LLC"
        assert results[0].award_amount == 185000.00
        assert results[0].state == "OR"

    @patch("engine.intel.usaspending.httpx.post")
    def test_search_awards_empty_results(self, mock_post):
        mock_resp = MagicMock()
        mock_resp.status_code = 200
        mock_resp.json.return_value = MOCK_EMPTY_RESPONSE
        mock_resp.raise_for_status = MagicMock()
        mock_post.return_value = mock_resp

        client = USASpendingClient()
        results = client.search_awards(keywords=["nonexistent"])
        assert results == []

    @patch("engine.intel.usaspending.httpx.post")
    def test_search_awards_builds_correct_payload(self, mock_post):
        mock_resp = MagicMock()
        mock_resp.status_code = 200
        mock_resp.json.return_value = MOCK_EMPTY_RESPONSE
        mock_resp.raise_for_status = MagicMock()
        mock_post.return_value = mock_resp

        client = USASpendingClient()
        client.search_awards(
            keywords=["janitorial"],
            state="OR",
            naics="561720",
            limit=25,
        )

        # Verify the POST body structure
        call_args = mock_post.call_args
        body = call_args[1]["json"] if "json" in call_args[1] else call_args[0][1]
        assert "filters" in body
        assert body["filters"]["keywords"] == ["janitorial"]
        assert body["limit"] == 25

    @patch("engine.intel.usaspending.httpx.post")
    def test_search_awards_retry_on_failure(self, mock_post):
        """Retries once on HTTP 500, then succeeds."""
        fail_resp = MagicMock()
        fail_resp.status_code = 500
        fail_resp.raise_for_status.side_effect = Exception("Server Error")

        ok_resp = MagicMock()
        ok_resp.status_code = 200
        ok_resp.json.return_value = MOCK_EMPTY_RESPONSE
        ok_resp.raise_for_status = MagicMock()

        mock_post.side_effect = [fail_resp, ok_resp]

        client = USASpendingClient(max_retries=2)
        results = client.search_awards(keywords=["test"])
        assert results == []
        assert mock_post.call_count == 2


class TestFindIncumbent:
    """Test the high-level find_incumbent helper."""

    @patch("engine.intel.usaspending.httpx.post")
    def test_find_incumbent_returns_most_recent(self, mock_post):
        mock_resp = MagicMock()
        mock_resp.status_code = 200
        mock_resp.json.return_value = MOCK_SPENDING_RESPONSE
        mock_resp.raise_for_status = MagicMock()
        mock_post.return_value = mock_resp

        result = find_incumbent(
            agency="Forest Service",
            location="Prospect, OR",
            keywords=["janitorial", "High Cascades"],
        )
        assert result is not None
        assert result.recipient_name == "ABC Janitorial Services LLC"
        assert result.award_amount == 185000.00

    @patch("engine.intel.usaspending.httpx.post")
    def test_find_incumbent_returns_none_when_empty(self, mock_post):
        mock_resp = MagicMock()
        mock_resp.status_code = 200
        mock_resp.json.return_value = MOCK_EMPTY_RESPONSE
        mock_resp.raise_for_status = MagicMock()
        mock_post.return_value = mock_resp

        result = find_incumbent(
            agency="Nonexistent Agency",
            location="Nowhere, XX",
            keywords=["nothing"],
        )
        assert result is None


class TestFindComps:
    """Test the find_comps helper for comparable awards."""

    @patch("engine.intel.usaspending.httpx.post")
    def test_find_comps_returns_sorted_by_date(self, mock_post):
        mock_resp = MagicMock()
        mock_resp.status_code = 200
        mock_resp.json.return_value = MOCK_SPENDING_RESPONSE
        mock_resp.raise_for_status = MagicMock()
        mock_post.return_value = mock_resp

        results = find_comps(
            naics="561720",
            state="OR",
            keywords=["janitorial"],
        )
        assert len(results) == 2
        # Most recent first
        assert results[0].start_date >= results[1].start_date

    @patch("engine.intel.usaspending.httpx.post")
    def test_find_comps_calculates_annual_rate(self, mock_post):
        mock_resp = MagicMock()
        mock_resp.status_code = 200
        mock_resp.json.return_value = MOCK_SPENDING_RESPONSE
        mock_resp.raise_for_status = MagicMock()
        mock_post.return_value = mock_resp

        results = find_comps(naics="561720", state="OR", keywords=["janitorial"])
        for r in results:
            assert r.annual_rate > 0
