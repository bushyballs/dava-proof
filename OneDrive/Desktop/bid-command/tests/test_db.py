"""Tests for engine.db — TDD, SQLite bid database."""

import pytest

from engine.db import BidDB, BidRecord


# ---------------------------------------------------------------------------
# Fixture
# ---------------------------------------------------------------------------

@pytest.fixture
def db(tmp_path):
    return BidDB(tmp_path / "test.db")


# ---------------------------------------------------------------------------
# Test 1 — Create bid, verify BidRecord fields
# ---------------------------------------------------------------------------

def test_create_bid(db):
    """Create a bid and verify the returned BidRecord."""
    record = db.create_bid(
        sol_number="SOL-001",
        title="Test Janitorial Services",
        agency="USFS",
        due_date="2026-05-01",
    )
    assert isinstance(record, BidRecord)
    assert record.sol_number == "SOL-001"
    assert record.status == "draft"
    assert len(record.id) > 0


# ---------------------------------------------------------------------------
# Test 2 — Get bid by id after creation
# ---------------------------------------------------------------------------

def test_get_bid(db):
    """Create a bid then fetch it by id; sol_number must match."""
    created = db.create_bid(
        sol_number="SOL-002",
        title="Grounds Maintenance",
        agency="BLM",
        due_date="2026-06-15",
    )
    fetched = db.get_bid(created.id)
    assert fetched is not None
    assert fetched.sol_number == "SOL-002"


# ---------------------------------------------------------------------------
# Test 3 — Update status
# ---------------------------------------------------------------------------

def test_update_status(db):
    """Create a bid, update its status to 'sent', and verify."""
    record = db.create_bid(
        sol_number="SOL-003",
        title="Road Maintenance",
        agency="USDA",
        due_date="2026-07-01",
    )
    db.update_status(record.id, "sent")
    fetched = db.get_bid(record.id)
    assert fetched is not None
    assert fetched.status == "sent"


# ---------------------------------------------------------------------------
# Test 4 — List bids
# ---------------------------------------------------------------------------

def test_list_bids(db):
    """Create 2 bids and verify list returns both."""
    db.create_bid(
        sol_number="SOL-004",
        title="Fire Lookout Cleaning",
        agency="USFS",
        due_date="2026-08-01",
    )
    db.create_bid(
        sol_number="SOL-005",
        title="Ranger Station Janitorial",
        agency="BLM",
        due_date="2026-08-15",
    )
    results = db.list_bids()
    assert len(results) == 2
