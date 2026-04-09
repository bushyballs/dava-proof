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
/// Format: HOAGS-INV-<CONTRACT_SHORT>-YYYYMM-NNN
///
/// `contract_short` is derived from the contract number by taking the first
/// token (up to the first space or dash-separated prefix), capped at 8 chars.
/// e.g. "W9127S26QA030" → "W9127S", "W912BV-21-C-0001" → "W912BV".
pub fn build_invoice_number(contract_number: &str, period: &str, sequence: u32) -> String {
    // period is like "2026-04" — strip the dash for the compact form
    let compact = period.replace('-', "");
    // Extract a short contract tag (up to 6 chars, alpha-numeric prefix)
    let contract_tag: String = contract_number
        .chars()
        .take(6)
        .collect();
    format!("HOAGS-INV-{}-{}-{:03}", contract_tag, compact, sequence)
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

// ── Build invoice lines with cumulative deduction ─────────────────────────────
/// Same as `calculate_invoice_lines` but subtracts already-invoiced amounts
/// per CLIN so the new invoice only bills what hasn't been invoiced yet.
///
/// `already_invoiced` maps clin_number → total amount already submitted/paid.
pub fn calculate_invoice_lines_with_deduction(
    contract: &Contract,
    billing_period: &str,
    already_invoiced: &std::collections::HashMap<String, f64>,
) -> Result<(Vec<InvoiceLine>, f64), String> {
    let (mut lines, _) = calculate_invoice_lines(contract, billing_period)?;
    let mut grand_total = 0.0f64;

    for line in &mut lines {
        let already = already_invoiced.get(&line.clin).copied().unwrap_or(0.0);
        // Deduct what's already been invoiced; floor at zero
        line.amount = (line.amount - already).max(0.0);
        line.amount = (line.amount * 100.0).round() / 100.0;
        grand_total += line.amount;
    }
    grand_total = (grand_total * 100.0).round() / 100.0;
    Ok((lines, grand_total))
}

// ── Build an Invoice struct ──────────────────────────────────────────────────

/// Build an invoice, optionally subtracting already-invoiced amounts.
/// Pass `Some(map)` from `tracker.total_invoiced_per_clin()` to enable
/// cumulative tracking; pass `None` to bill the full period amounts.
pub fn build_invoice(
    contract: &Contract,
    billing_period: &str,
    sequence: u32,
    already_invoiced: Option<&std::collections::HashMap<String, f64>>,
) -> Result<Invoice, String> {
    let invoice_number = build_invoice_number(&contract.contract_number, billing_period, sequence);
    let (lines, total) = match already_invoiced {
        Some(map) => calculate_invoice_lines_with_deduction(contract, billing_period, map)?,
        None => calculate_invoice_lines(contract, billing_period)?,
    };
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

// ── Contract validation ────────────────────────────────────────────────────────

/// Validation result for a contract JSON.
#[derive(Debug)]
pub struct ValidationResult {
    pub ok: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Validate a contract for completeness and sensibility.
pub fn validate_contract(contract: &Contract) -> ValidationResult {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Required string fields
    if contract.contract_number.trim().is_empty() {
        errors.push("contract_number is empty".to_string());
    }
    if contract.contractor.trim().is_empty() {
        errors.push("contractor name is empty".to_string());
    }

    // Period dates
    match NaiveDate::parse_from_str(&contract.period.start, "%Y-%m-%d") {
        Err(e) => errors.push(format!("period.start '{}' is not a valid YYYY-MM-DD date: {e}", contract.period.start)),
        Ok(_) => {}
    }
    match NaiveDate::parse_from_str(&contract.period.end, "%Y-%m-%d") {
        Err(e) => errors.push(format!("period.end '{}' is not a valid YYYY-MM-DD date: {e}", contract.period.end)),
        Ok(_) => {}
    }
    if let (Ok(s), Ok(e)) = (
        NaiveDate::parse_from_str(&contract.period.start, "%Y-%m-%d"),
        NaiveDate::parse_from_str(&contract.period.end, "%Y-%m-%d"),
    ) {
        if e <= s {
            errors.push(format!(
                "period.end ({}) must be after period.start ({})",
                contract.period.end, contract.period.start
            ));
        }
    }

    // CLINs
    if contract.clins.is_empty() {
        errors.push("contract has no CLINs".to_string());
    }
    for (i, clin) in contract.clins.iter().enumerate() {
        if clin.number.trim().is_empty() {
            errors.push(format!("clins[{i}].number is empty"));
        }
        if clin.description.trim().is_empty() {
            warnings.push(format!("clins[{i}] ({}) has no description", clin.number));
        }
        if clin.unit_price <= 0.0 {
            errors.push(format!("clins[{i}] ({}) has non-positive unit_price {}", clin.number, clin.unit_price));
        }
        if clin.quantity <= 0.0 {
            errors.push(format!("clins[{i}] ({}) has non-positive quantity {}", clin.number, clin.quantity));
        }
        if clin.unit.trim().is_empty() {
            warnings.push(format!("clins[{i}] ({}) has no unit", clin.number));
        }
    }

    // Check for duplicate CLIN numbers
    let mut seen = std::collections::HashSet::new();
    for clin in &contract.clins {
        if !seen.insert(&clin.number) {
            errors.push(format!("duplicate CLIN number '{}'", clin.number));
        }
    }

    ValidationResult {
        ok: errors.is_empty(),
        errors,
        warnings,
    }
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
        assert_eq!(
            build_invoice_number("W9127S26QA030", "2026-04", 1),
            "HOAGS-INV-W9127S-202604-001"
        );
        assert_eq!(
            build_invoice_number("W9127S26QA030", "2026-12", 42),
            "HOAGS-INV-W9127S-202612-042"
        );
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
        let inv = build_invoice(&c, "2026-05", 1, None).unwrap();
        assert_eq!(inv.invoice_number, "HOAGS-INV-W9127S-202605-001");
        assert_eq!(inv.contract_number, "W9127S26QA030");
        assert!(inv.total >= 0.0);
    }

    #[test]
    fn test_build_invoice_with_cumulative_deduction() {
        use std::collections::HashMap;
        let c = parse_contract(sample_contract_json()).unwrap();
        // Simulate that CLIN 0001 already had $50 invoiced
        let mut already: HashMap<String, f64> = HashMap::new();
        already.insert("0001".to_string(), 50.0);
        let inv_full = build_invoice(&c, "2026-05", 1, None).unwrap();
        let inv_deducted = build_invoice(&c, "2026-05", 2, Some(&already)).unwrap();
        // Deducted invoice line 0001 should be $50 less (or 0 if < 50)
        let full_0001 = inv_full.lines.iter().find(|l| l.clin == "0001").unwrap().amount;
        let ded_0001 = inv_deducted.lines.iter().find(|l| l.clin == "0001").unwrap().amount;
        let expected = (full_0001 - 50.0).max(0.0);
        assert!((ded_0001 - expected).abs() < 0.02, "expected {expected} got {ded_0001}");
    }

    #[test]
    fn test_invoice_total_matches_line_sum() {
        let c = parse_contract(sample_contract_json()).unwrap();
        let (lines, total) = calculate_invoice_lines(&c, "2026-06").unwrap();
        let sum: f64 = lines.iter().map(|l| l.amount).sum();
        assert!((sum - total).abs() < 0.02, "line sum {sum} should match total {total}");
    }

    #[test]
    fn test_validate_contract_valid() {
        let c = parse_contract(sample_contract_json()).unwrap();
        let result = validate_contract(&c);
        assert!(result.ok, "valid contract should pass: {:?}", result.errors);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validate_contract_empty_number() {
        let json = r#"{
          "contract_number": "",
          "contractor": "Hoags Inc.",
          "clins": [
            {"number": "0001", "description": "Service", "unit_price": 100.0, "quantity": 1.0, "unit": "EA"}
          ],
          "period": {"start": "2026-04-01", "end": "2026-12-31"}
        }"#;
        let c = parse_contract(json).unwrap();
        let result = validate_contract(&c);
        assert!(!result.ok);
        assert!(result.errors.iter().any(|e| e.contains("contract_number")));
    }

    #[test]
    fn test_validate_contract_bad_period() {
        let json = r#"{
          "contract_number": "W9127S26QA030",
          "contractor": "Hoags Inc.",
          "clins": [
            {"number": "0001", "description": "Service", "unit_price": 100.0, "quantity": 1.0, "unit": "EA"}
          ],
          "period": {"start": "2026-12-31", "end": "2026-01-01"}
        }"#;
        let c = parse_contract(json).unwrap();
        let result = validate_contract(&c);
        assert!(!result.ok);
        assert!(result.errors.iter().any(|e| e.contains("period.end")));
    }

    #[test]
    fn test_validate_contract_no_clins() {
        let json = r#"{
          "contract_number": "W9127S26QA030",
          "contractor": "Hoags Inc.",
          "clins": [],
          "period": {"start": "2026-04-01", "end": "2026-12-31"}
        }"#;
        let c = parse_contract(json).unwrap();
        let result = validate_contract(&c);
        assert!(!result.ok);
        assert!(result.errors.iter().any(|e| e.contains("no CLINs")));
    }

    #[test]
    fn test_validate_contract_duplicate_clins() {
        let json = r#"{
          "contract_number": "W9127S26QA030",
          "contractor": "Hoags Inc.",
          "clins": [
            {"number": "0001", "description": "Service A", "unit_price": 100.0, "quantity": 1.0, "unit": "EA"},
            {"number": "0001", "description": "Service B", "unit_price": 50.0, "quantity": 2.0, "unit": "EA"}
          ],
          "period": {"start": "2026-04-01", "end": "2026-12-31"}
        }"#;
        let c = parse_contract(json).unwrap();
        let result = validate_contract(&c);
        assert!(!result.ok);
        assert!(result.errors.iter().any(|e| e.contains("duplicate CLIN")));
    }
}
