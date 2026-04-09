//! invoice.rs — Core invoice data model and calculation logic.
//!
//! Parses contract JSON, calculates CLIN amounts for a billing period,
//! and generates invoice numbers.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

// ── Contract / CLIN types ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clin {
    pub number: String,
    pub description: String,
    pub unit_price: f64,
    pub quantity: f64,
    pub unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractPeriod {
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contract {
    pub contract_number: String,
    pub contractor: String,
    pub clins: Vec<Clin>,
    pub period: ContractPeriod,
}

// ── Invoice line (calculated) ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceLine {
    pub clin: String,
    pub description: String,
    pub qty: f64,
    pub unit: String,
    pub unit_price: f64,
    pub amount: f64,
}

// ── Invoice header ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    pub invoice_number: String,
    pub contract_number: String,
    pub contractor: String,
    pub billing_period: String,
    pub lines: Vec<InvoiceLine>,
    pub total: f64,
    pub generated_at: String,
}

// ── Parse contract from JSON string ─────────────────────────────────────────

pub fn parse_contract(json: &str) -> Result<Contract, String> {
    serde_json::from_str(json).map_err(|e| format!("Failed to parse contract JSON: {e}"))
}

// ── Build invoice number ─────────────────────────────────────────────────────
/// Format: HOAGS-INV-YYYYMM-NNN
pub fn build_invoice_number(period: &str, sequence: u32) -> String {
    // period is like "2026-04" — strip the dash for the compact form
    let compact = period.replace('-', "");
    format!("HOAGS-INV-{}-{:03}", compact, sequence)
}

// ── Calculate amounts per CLIN for a billing period ─────────────────────────
///
/// `billing_period` is "YYYY-MM" (e.g. "2026-04").
///
/// Strategy:
///  - Parse the month from `billing_period`.
///  - Determine how many days in that month fall within the contract period.
///  - For each CLIN, quantity = total_qty * (days_in_period / total_contract_days).
///    If total_contract_days == 0, fall back to the raw quantity.
///
/// This gives a pro-rated amount when the billing period is shorter than the
/// full contract; for simple "bill the whole quantity" scenarios the caller
/// passes the full contract period as the billing period.
pub fn calculate_invoice_lines(
    contract: &Contract,
    billing_period: &str, // "YYYY-MM"
) -> Result<(Vec<InvoiceLine>, f64), String> {
    // Parse billing month
    let (year_str, month_str) = billing_period
        .split_once('-')
        .ok_or_else(|| format!("Invalid period '{}' — expected YYYY-MM", billing_period))?;
    let year: i32 = year_str.parse().map_err(|_| "Invalid year".to_string())?;
    let month: u32 = month_str.parse().map_err(|_| "Invalid month".to_string())?;

    // First and last day of billing month
    let month_start = NaiveDate::from_ymd_opt(year, month, 1)
        .ok_or_else(|| format!("Invalid date: {}-{}-01", year, month))?;
    let month_end = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap() - chrono::Duration::days(1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap() - chrono::Duration::days(1)
    };

    // Contract period bounds
    let contract_start = NaiveDate::parse_from_str(&contract.period.start, "%Y-%m-%d")
        .map_err(|e| format!("Bad contract start: {e}"))?;
    let contract_end = NaiveDate::parse_from_str(&contract.period.end, "%Y-%m-%d")
        .map_err(|e| format!("Bad contract end: {e}"))?;

    // Overlap of billing month with contract period
    let eff_start = month_start.max(contract_start);
    let eff_end = month_end.min(contract_end);

    // If the billing month is entirely outside the contract, return empty
    if eff_start > eff_end {
        return Ok((vec![], 0.0));
    }

    let billing_days = (eff_end - eff_start).num_days() + 1;
    let total_days = (contract_end - contract_start).num_days() + 1;

    let ratio = if total_days > 0 {
        billing_days as f64 / total_days as f64
    } else {
        1.0
    };

    let mut lines = Vec::new();
    let mut grand_total = 0.0f64;

    for clin in &contract.clins {
        // Pro-rate quantity to the billing period
        let period_qty = (clin.quantity * ratio * 100.0).round() / 100.0;
        let amount = (period_qty * clin.unit_price * 100.0).round() / 100.0;
        grand_total += amount;
        lines.push(InvoiceLine {
            clin: clin.number.clone(),
            description: clin.description.clone(),
            qty: period_qty,
            unit: clin.unit.clone(),
            unit_price: clin.unit_price,
            amount,
        });
    }

    grand_total = (grand_total * 100.0).round() / 100.0;
    Ok((lines, grand_total))
}

// ── Build an Invoice struct ──────────────────────────────────────────────────

pub fn build_invoice(
    contract: &Contract,
    billing_period: &str,
    sequence: u32,
) -> Result<Invoice, String> {
    let invoice_number = build_invoice_number(billing_period, sequence);
    let (lines, total) = calculate_invoice_lines(contract, billing_period)?;
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    Ok(Invoice {
        invoice_number,
        contract_number: contract.contract_number.clone(),
        contractor: contract.contractor.clone(),
        billing_period: billing_period.to_string(),
        lines,
        total,
        generated_at: now,
    })
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_contract_json() -> &'static str {
        r#"{
          "contract_number": "W9127S26QA030",
          "contractor": "Hoags Inc.",
          "clins": [
            {"number": "0001", "description": "Daily Service", "unit_price": 91.19, "quantity": 110.0, "unit": "EA"},
            {"number": "0002", "description": "Semi-Annual Service", "unit_price": 307.61, "quantity": 2.0, "unit": "EA"}
          ],
          "period": {"start": "2026-04-09", "end": "2027-03-31"}
        }"#
    }

    #[test]
    fn test_parse_contract() {
        let c = parse_contract(sample_contract_json()).unwrap();
        assert_eq!(c.contract_number, "W9127S26QA030");
        assert_eq!(c.clins.len(), 2);
        assert!((c.clins[0].unit_price - 91.19).abs() < 0.001);
    }

    #[test]
    fn test_build_invoice_number() {
        assert_eq!(build_invoice_number("2026-04", 1), "HOAGS-INV-202604-001");
        assert_eq!(build_invoice_number("2026-12", 42), "HOAGS-INV-202612-042");
    }

    #[test]
    fn test_calculate_invoice_lines_in_period() {
        let c = parse_contract(sample_contract_json()).unwrap();
        let (lines, total) = calculate_invoice_lines(&c, "2026-04").unwrap();
        assert!(!lines.is_empty(), "should have lines");
        // April starts 2026-04-09, so there are 22 days in billing window
        // Contract is 2026-04-09 to 2027-03-31 = 357 days
        // ratio ≈ 22/357
        assert!(total > 0.0, "total should be positive");
        assert_eq!(lines.len(), 2, "2 CLINs");
    }

    #[test]
    fn test_calculate_invoice_lines_outside_period() {
        let c = parse_contract(sample_contract_json()).unwrap();
        // 2025-01 is before contract start
        let (lines, total) = calculate_invoice_lines(&c, "2025-01").unwrap();
        assert!(lines.is_empty());
        assert_eq!(total, 0.0);
    }

    #[test]
    fn test_build_invoice() {
        let c = parse_contract(sample_contract_json()).unwrap();
        let inv = build_invoice(&c, "2026-05", 1).unwrap();
        assert_eq!(inv.invoice_number, "HOAGS-INV-202605-001");
        assert_eq!(inv.contract_number, "W9127S26QA030");
        assert!(inv.total >= 0.0);
    }

    #[test]
    fn test_invoice_total_matches_line_sum() {
        let c = parse_contract(sample_contract_json()).unwrap();
        let (lines, total) = calculate_invoice_lines(&c, "2026-06").unwrap();
        let sum: f64 = lines.iter().map(|l| l.amount).sum();
        assert!((sum - total).abs() < 0.02, "line sum {sum} should match total {total}");
    }
}
