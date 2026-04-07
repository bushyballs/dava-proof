# engine/intel/pricing.py
"""Pricing engine — SCA burden calculator + competitive bid pricing.

Calculates fully-loaded hourly rates from SCA wage determinations,
recommends competitive CLIN pricing based on incumbent data, and
detects red flags when prices are below cost or margins are thin.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from engine.parse.clins import Clin
from engine.parse.wage_det import WageData
from engine.intel.usaspending import AwardResult


# ---------------------------------------------------------------------------
# Constants — default burden rates
# ---------------------------------------------------------------------------

# Federal payroll taxes
DEFAULT_FICA_RATE = 0.0765     # Social Security (6.2%) + Medicare (1.45%)
DEFAULT_FUTA_RATE = 0.006      # Federal Unemployment (0.6% on first $7K)
DEFAULT_SUTA_RATE = 0.02       # State Unemployment (~2% average)
DEFAULT_WC_RATE = 0.03         # Workers Compensation (~3% for janitorial)

# Business margins
DEFAULT_OVERHEAD_RATE = 0.10   # 10% overhead (admin, insurance, bonding)
DEFAULT_PROFIT_RATE = 0.10     # 10% profit margin

# Competitive positioning
DEFAULT_UNDERCUT_PCT = 0.10    # 10% below incumbent

# Fallback hourly rate when no wage data available (federal minimum for janitor)
FALLBACK_WAGE = 15.00
FALLBACK_HW = 4.22

# Default hours estimate per CLIN unit type
_HOURS_PER_UNIT: dict[str, float] = {
    "MO": 86.67,    # ~20 hrs/week average for monthly janitorial
    "HR": 1.0,      # per hour
    "EA": 4.0,      # per-each job, ~4 hours
    "JC": 8.0,      # per-job-call, ~8 hours (carpet/wax)
    "JB": 8.0,      # per-job, ~8 hours
    "LO": 40.0,     # per-lot, ~40 hours
    "LS": 40.0,     # lump sum, ~40 hours
}

# Red flag thresholds
MIN_MARGIN_PCT = 0.10          # Warn if margin < 10%


# ---------------------------------------------------------------------------
# Data classes
# ---------------------------------------------------------------------------

@dataclass
class BurdenCalc:
    """Result of SCA burden calculation — all dollar amounts per hour."""

    base_wage: float = 0.0
    hw_fringe: float = 0.0
    base_loaded: float = 0.0       # wage + H&W
    fica: float = 0.0
    futa: float = 0.0
    suta: float = 0.0
    workers_comp: float = 0.0
    labor_cost: float = 0.0        # base_loaded + all taxes
    overhead: float = 0.0
    profit: float = 0.0
    fully_loaded: float = 0.0      # final $/hr to charge


@dataclass
class ClinPrice:
    """Recommended price for a single CLIN."""

    clin_number: str = ""
    description: str = ""
    quantity: int = 0
    unit: str = ""
    hours_per_unit: float = 0.0
    hourly_rate: float = 0.0
    unit_price: float = 0.0
    total: float = 0.0
    year: str = ""


@dataclass
class PricingResult:
    """Complete pricing recommendation for a solicitation."""

    clin_prices: list[ClinPrice] = field(default_factory=list)
    grand_total: float = 0.0
    cost_floor: float = 0.0        # minimum viable price (no profit)
    margin_pct: float = 0.0
    burden: BurdenCalc | None = None
    incumbent_rate: float = 0.0
    recommended_rate: float = 0.0
    red_flags: list[str] = field(default_factory=list)


# ---------------------------------------------------------------------------
# SCA Burden Calculation
# ---------------------------------------------------------------------------

def calculate_sca_burden(
    base_wage: float,
    hw_fringe: float,
    fica_rate: float = DEFAULT_FICA_RATE,
    futa_rate: float = DEFAULT_FUTA_RATE,
    suta_rate: float = DEFAULT_SUTA_RATE,
    wc_rate: float = DEFAULT_WC_RATE,
    overhead_rate: float = DEFAULT_OVERHEAD_RATE,
    profit_rate: float = DEFAULT_PROFIT_RATE,
) -> BurdenCalc:
    """Calculate fully-loaded hourly cost from SCA base wage + H&W fringe.

    The burden chain:
        base_loaded = wage + H&W
        labor_cost  = base_loaded + FICA + FUTA + SUTA + workers_comp
        with_overhead = labor_cost * (1 + overhead_rate)
        fully_loaded  = with_overhead * (1 + profit_rate)

    Args:
        base_wage: SCA base hourly wage (e.g. 16.40).
        hw_fringe: Health & Welfare fringe per hour (e.g. 5.36).
        fica_rate: FICA tax rate (default 7.65%).
        futa_rate: FUTA tax rate (default 0.6%).
        suta_rate: SUTA tax rate (default 2%).
        wc_rate: Workers comp rate (default 3%).
        overhead_rate: Overhead percentage (default 10%).
        profit_rate: Profit percentage (default 10%).

    Returns:
        BurdenCalc with all components broken out.
    """
    base_loaded = round(base_wage + hw_fringe, 2)

    if base_loaded == 0.0:
        return BurdenCalc()

    fica = round(base_loaded * fica_rate, 2)
    futa = round(base_loaded * futa_rate, 2)
    suta = round(base_loaded * suta_rate, 2)
    workers_comp = round(base_loaded * wc_rate, 2)

    labor_cost = round(base_loaded + fica + futa + suta + workers_comp, 2)

    with_overhead = round(labor_cost * (1 + overhead_rate), 2)
    overhead = round(with_overhead - labor_cost, 2)

    fully_loaded = round(with_overhead * (1 + profit_rate), 2)
    profit = round(fully_loaded - with_overhead, 2)

    return BurdenCalc(
        base_wage=base_wage,
        hw_fringe=hw_fringe,
        base_loaded=base_loaded,
        fica=fica,
        futa=futa,
        suta=suta,
        workers_comp=workers_comp,
        labor_cost=labor_cost,
        overhead=overhead,
        profit=profit,
        fully_loaded=fully_loaded,
    )


# ---------------------------------------------------------------------------
# CLIN Pricing
# ---------------------------------------------------------------------------

def _price_clin(
    clin: Clin,
    hourly_rate: float,
) -> ClinPrice:
    """Calculate price for a single CLIN based on estimated hours and hourly rate.

    Args:
        clin: Parsed CLIN data (number, description, quantity, unit).
        hourly_rate: Fully-loaded hourly rate from burden calc.

    Returns:
        ClinPrice with unit_price and total computed.
    """
    hours = _HOURS_PER_UNIT.get(clin.unit, 8.0)
    unit_price = round(hourly_rate * hours, 2)
    total = round(unit_price * clin.quantity, 2)

    return ClinPrice(
        clin_number=clin.number,
        description=clin.description,
        quantity=clin.quantity,
        unit=clin.unit,
        hours_per_unit=hours,
        hourly_rate=hourly_rate,
        unit_price=unit_price,
        total=total,
        year=clin.year,
    )


def calculate_price(
    clins: list[Clin],
    wage_data: WageData,
    incumbent: AwardResult | None = None,
    comps: list[AwardResult] | None = None,
    undercut_pct: float = DEFAULT_UNDERCUT_PCT,
    overhead_rate: float = DEFAULT_OVERHEAD_RATE,
    profit_rate: float = DEFAULT_PROFIT_RATE,
) -> PricingResult:
    """Calculate recommended pricing for all CLINs in a solicitation.

    Strategy:
        1. Calculate SCA burden to get fully-loaded hourly rate
        2. Price each CLIN using hours estimate * hourly rate
        3. If incumbent data available, position 5-15% below their rate
        4. Never go below cost floor (0% profit)
        5. Flag any red flags (below cost, low margin)

    Args:
        clins: Parsed CLINs from the solicitation.
        wage_data: SCA wage determination data.
        incumbent: Most recent incumbent award (if found).
        comps: List of comparable awards (for market context).
        undercut_pct: How far below incumbent to price (default 10%).
        overhead_rate: Overhead percentage (default 10%).
        profit_rate: Profit percentage (default 10%).

    Returns:
        PricingResult with per-CLIN prices and grand total.
    """
    # Use fallback wages if WD has none
    base_wage = wage_data.janitor_rate if wage_data.janitor_rate > 0 else FALLBACK_WAGE
    hw_fringe = wage_data.hw_fringe if wage_data.hw_fringe > 0 else FALLBACK_HW

    # Step 1: Calculate burden
    burden = calculate_sca_burden(
        base_wage=base_wage,
        hw_fringe=hw_fringe,
        overhead_rate=overhead_rate,
        profit_rate=profit_rate,
    )

    # Step 2: Also calculate cost floor (0% profit burden)
    floor_burden = calculate_sca_burden(
        base_wage=base_wage,
        hw_fringe=hw_fringe,
        overhead_rate=overhead_rate,
        profit_rate=0.0,  # no profit = cost floor
    )

    # Step 3: Price each CLIN at the fully-loaded rate
    clin_prices = [_price_clin(c, burden.fully_loaded) for c in clins]
    grand_total = round(sum(cp.total for cp in clin_prices), 2)

    # Cost floor: same CLINs priced at zero-profit rate
    floor_prices = [_price_clin(c, floor_burden.fully_loaded) for c in clins]
    cost_floor = round(sum(cp.total for cp in floor_prices), 2)

    # Step 4: If incumbent data, try to position below them
    incumbent_rate = 0.0
    recommended_rate = grand_total

    if incumbent and incumbent.annual_rate > 0:
        incumbent_rate = incumbent.annual_rate
        target = round(incumbent_rate * (1.0 - undercut_pct), 2)

        # Only undercut if we can stay above cost floor
        if target >= cost_floor:
            recommended_rate = target
            # Scale CLIN prices proportionally
            if grand_total > 0:
                scale = recommended_rate / grand_total
                for cp in clin_prices:
                    cp.unit_price = round(cp.unit_price * scale, 2)
                    cp.total = round(cp.unit_price * cp.quantity, 2)
                grand_total = round(sum(cp.total for cp in clin_prices), 2)
        else:
            # Can't undercut — price at cost + minimum margin
            recommended_rate = cost_floor

    # Step 5: Calculate margin
    margin_pct = 0.0
    if cost_floor > 0:
        margin_pct = round((grand_total - cost_floor) / grand_total, 4)

    # Step 6: Check red flags
    red_flags = check_red_flags(
        grand_total=grand_total,
        cost_floor=cost_floor,
        margin_pct=margin_pct,
    )

    return PricingResult(
        clin_prices=clin_prices,
        grand_total=grand_total,
        cost_floor=cost_floor,
        margin_pct=margin_pct,
        burden=burden,
        incumbent_rate=incumbent_rate,
        recommended_rate=recommended_rate,
        red_flags=red_flags,
    )


# ---------------------------------------------------------------------------
# Red Flag Detection
# ---------------------------------------------------------------------------

def check_red_flags(
    grand_total: float,
    cost_floor: float,
    margin_pct: float,
) -> list[str]:
    """Check pricing for red flags.

    Flags:
        - Price below cost floor
        - Margin below minimum threshold (10%)

    Args:
        grand_total: Proposed total price.
        cost_floor: Minimum viable price (zero profit).
        margin_pct: Profit margin as a decimal (e.g. 0.15 = 15%).

    Returns:
        List of warning strings (empty if no issues).
    """
    flags: list[str] = []

    if cost_floor > 0 and grand_total < cost_floor:
        gap = round(cost_floor - grand_total, 2)
        flags.append(
            f"BELOW COST: price ${grand_total:,.2f} is ${gap:,.2f} below "
            f"cost floor ${cost_floor:,.2f}"
        )

    if 0 < margin_pct < MIN_MARGIN_PCT:
        flags.append(
            f"LOW MARGIN: {margin_pct:.1%} is below minimum {MIN_MARGIN_PCT:.0%} threshold"
        )

    return flags


# ---------------------------------------------------------------------------
# Option Year Escalation
# ---------------------------------------------------------------------------

def escalate_price(
    base_price: float,
    escalation_pct: float = 0.03,
    option_years: int = 4,
) -> list[float]:
    """Calculate escalated prices across base year + option years.

    Args:
        base_price: Base year price.
        escalation_pct: Annual escalation rate (default 3%).
        option_years: Number of option years (default 4).

    Returns:
        List of prices: [base, oy1, oy2, ...].
    """
    prices = [base_price]
    for year in range(1, option_years + 1):
        escalated = round(base_price * (1.0 + escalation_pct) ** year, 2)
        prices.append(escalated)
    return prices
