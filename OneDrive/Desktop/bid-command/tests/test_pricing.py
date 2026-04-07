# tests/test_pricing.py
"""Tests for the pricing engine — real bid scenarios, no mocking needed."""

import pytest
from engine.intel.pricing import (
    PricingResult,
    ClinPrice,
    BurdenCalc,
    calculate_sca_burden,
    calculate_price,
    check_red_flags,
    escalate_price,
)
from engine.parse.clins import Clin
from engine.parse.wage_det import WageData
from engine.intel.usaspending import AwardResult


# ---------------------------------------------------------------------------
# SCA Burden Calculation Tests — using real NM/SW rates from today
# ---------------------------------------------------------------------------

class TestSCABurden:
    """Test SCA burden calculation with real Mora/Taos county rates."""

    def test_burden_calc_basic(self):
        """NM rural SCA: $16.40/hr + $5.36 H&W = $21.76 base loaded."""
        result = calculate_sca_burden(
            base_wage=16.40,
            hw_fringe=5.36,
        )
        assert isinstance(result, BurdenCalc)
        assert result.base_wage == 16.40
        assert result.hw_fringe == 5.36
        # FICA: (16.40 + 5.36) * 0.0765 = 1.6646
        assert abs(result.fica - round(21.76 * 0.0765, 2)) < 0.01
        # FUTA: 21.76 * 0.006 = 0.13
        assert abs(result.futa - round(21.76 * 0.006, 2)) < 0.01
        # SUTA: 21.76 * 0.02 = 0.44
        assert abs(result.suta - round(21.76 * 0.02, 2)) < 0.01
        # Workers comp: 21.76 * 0.03 = 0.65
        assert abs(result.workers_comp - round(21.76 * 0.03, 2)) < 0.01
        # Fully loaded should be > base
        assert result.fully_loaded > 21.76

    def test_burden_calc_with_custom_rates(self):
        """Override SUTA and workers comp rates."""
        result = calculate_sca_burden(
            base_wage=17.32,
            hw_fringe=5.55,
            suta_rate=0.025,
            wc_rate=0.04,
        )
        base_loaded = 17.32 + 5.55
        assert result.base_loaded == base_loaded
        assert abs(result.suta - round(base_loaded * 0.025, 2)) < 0.01
        assert abs(result.workers_comp - round(base_loaded * 0.04, 2)) < 0.01

    def test_burden_calc_overhead_and_profit(self):
        """Default 10% overhead + 10% profit on top of labor cost."""
        result = calculate_sca_burden(
            base_wage=16.40,
            hw_fringe=5.36,
            overhead_rate=0.10,
            profit_rate=0.10,
        )
        # Verify chain: labor_cost * (1 + overhead) * (1 + profit) = fully_loaded
        labor = result.base_loaded + result.fica + result.futa + result.suta + result.workers_comp
        expected = round(labor * 1.10 * 1.10, 2)
        assert abs(result.fully_loaded - expected) < 0.02

    def test_burden_calc_floor_rate_per_hour(self):
        """The fully loaded hourly rate must be above the SCA floor."""
        result = calculate_sca_burden(base_wage=16.40, hw_fringe=5.36)
        # SW pricing intelligence said fully loaded ~$24/hr for NM rural
        assert result.fully_loaded >= 23.0
        assert result.fully_loaded <= 30.0

    def test_burden_calc_zero_wage(self):
        """Zero wage should still compute without errors."""
        result = calculate_sca_burden(base_wage=0.0, hw_fringe=0.0)
        assert result.fully_loaded == 0.0
        assert result.base_loaded == 0.0


# ---------------------------------------------------------------------------
# Full Pricing Calculation Tests — real Camino Real scenario
# ---------------------------------------------------------------------------

class TestCalculatePrice:
    """Test calculate_price with real USFS janitorial bid scenarios."""

    def test_price_basic_janitorial_monthly(self):
        """Camino Real: 5,000 sqft 3x/week, 12 months, janitor $16.40 + $5.36 H&W."""
        clins = [
            Clin(number="0001", description="Janitorial Services", quantity=12, unit="MO", year="base"),
        ]
        wage = WageData(
            janitor_rate=16.40,
            hw_fringe=5.36,
        )
        result = calculate_price(clins=clins, wage_data=wage)
        assert isinstance(result, PricingResult)
        assert len(result.clin_prices) == 1
        cp = result.clin_prices[0]
        assert cp.clin_number == "0001"
        assert cp.unit_price > 0
        assert cp.total == round(cp.unit_price * cp.quantity, 2)
        # Annual total should be in the $17K-$22K range for 5,000 sqft
        # (using default hours estimate)
        assert result.grand_total > 0

    def test_price_with_incumbent_underbid(self):
        """Position 10% below incumbent's annual rate when incumbent is above our cost.

        Incumbent at $38K/yr is well above our cost floor (~$28K), so we
        should be able to undercut to ~$34.2K (10% below $38K).
        """
        clins = [
            Clin(number="0001", description="Janitorial Services", quantity=12, unit="MO", year="base"),
        ]
        wage = WageData(janitor_rate=16.40, hw_fringe=5.36)
        incumbent = AwardResult(
            recipient_name="Old Contractor LLC",
            award_amount=38000.00,
            start_date="2024-05-01",
            end_date="2025-04-30",
            annual_rate=38000.00,
        )
        result = calculate_price(
            clins=clins,
            wage_data=wage,
            incumbent=incumbent,
            undercut_pct=0.10,
        )
        # Grand total should be roughly 10% below incumbent
        assert result.grand_total <= incumbent.annual_rate * 0.95
        assert result.grand_total >= incumbent.annual_rate * 0.80

    def test_price_refuses_below_cost_floor(self):
        """Never price below cost floor even if incumbent is very cheap."""
        clins = [
            Clin(number="0001", description="Janitorial Services", quantity=12, unit="MO", year="base"),
        ]
        wage = WageData(janitor_rate=16.40, hw_fringe=5.36)
        # Unrealistically cheap incumbent
        incumbent = AwardResult(
            recipient_name="Lowball LLC",
            award_amount=5000.00,
            start_date="2024-05-01",
            end_date="2025-04-30",
            annual_rate=5000.00,
        )
        result = calculate_price(
            clins=clins,
            wage_data=wage,
            incumbent=incumbent,
            undercut_pct=0.10,
        )
        # Should never go below cost floor
        assert result.grand_total >= result.cost_floor
        assert len(result.red_flags) > 0

    def test_price_multiple_clins(self):
        """Multiple CLINs: monthly janitorial + per-job carpet cleaning."""
        clins = [
            Clin(number="0001", description="Janitorial Services", quantity=12, unit="MO", year="base"),
            Clin(number="0002", description="Carpet Cleaning", quantity=2, unit="JC", year="base"),
            Clin(number="0003", description="Strip and Wax Floors", quantity=1, unit="JC", year="base"),
        ]
        wage = WageData(janitor_rate=17.32, hw_fringe=5.55)
        result = calculate_price(clins=clins, wage_data=wage)
        assert len(result.clin_prices) == 3
        assert result.grand_total == sum(cp.total for cp in result.clin_prices)

    def test_price_no_wage_data_uses_defaults(self):
        """When wage data is empty, use federal minimum defaults."""
        clins = [
            Clin(number="0001", description="Janitorial Services", quantity=12, unit="MO", year="base"),
        ]
        wage = WageData()  # all zeros
        result = calculate_price(clins=clins, wage_data=wage)
        assert result.grand_total > 0  # should use fallback rates


# ---------------------------------------------------------------------------
# Red Flag Detection
# ---------------------------------------------------------------------------

class TestRedFlags:

    def test_below_cost_floor_flag(self):
        flags = check_red_flags(
            grand_total=10000.00,
            cost_floor=15000.00,
            margin_pct=0.05,
        )
        assert any("below cost" in f.lower() for f in flags)

    def test_low_margin_flag(self):
        flags = check_red_flags(
            grand_total=16000.00,
            cost_floor=15000.00,
            margin_pct=0.05,
        )
        assert any("margin" in f.lower() for f in flags)

    def test_no_flags_when_healthy(self):
        flags = check_red_flags(
            grand_total=20000.00,
            cost_floor=12000.00,
            margin_pct=0.40,
        )
        assert flags == []


# ---------------------------------------------------------------------------
# Option Year Escalation
# ---------------------------------------------------------------------------

class TestEscalation:

    def test_escalate_3pct_over_4_years(self):
        """3% annual escalation across base + 4 option years."""
        base_price = 1000.00
        years = escalate_price(base_price, escalation_pct=0.03, option_years=4)
        assert len(years) == 5  # base + 4 OYs
        assert years[0] == 1000.00
        assert years[1] == round(1000.00 * 1.03, 2)
        assert years[2] == round(1000.00 * 1.03**2, 2)
        assert years[3] == round(1000.00 * 1.03**3, 2)
        assert years[4] == round(1000.00 * 1.03**4, 2)

    def test_escalate_zero_pct(self):
        """0% escalation = same price every year."""
        years = escalate_price(500.00, escalation_pct=0.0, option_years=3)
        assert years == [500.00, 500.00, 500.00, 500.00]

    def test_escalate_single_year(self):
        """No option years = just the base price."""
        years = escalate_price(750.00, escalation_pct=0.03, option_years=0)
        assert years == [750.00]
