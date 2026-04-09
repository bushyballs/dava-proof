use rusqlite::Connection;
use crate::expense::Expense;
use crate::tracker;

// ── Deductibility classification ─────────────────────────────────────────────

/// Classify a category as deductible for federal contracting purposes.
/// Returns a human-readable label for the tax report.
fn deductibility_label(category: &str) -> &'static str {
    match category {
        "supplies"  => "Deductible — Business Supplies",
        "fuel"      => "Deductible — Business Travel / Vehicle",
        "labor"     => "Deductible — Contract Labor",
        "equipment" => "Deductible — Business Equipment",
        "travel"    => "Deductible — Business Travel",
        "office"    => "Deductible — Office Expense",
        "other"     => "Review Required",
        _           => "Review Required",
    }
}

// ── Print summary ─────────────────────────────────────────────────────────────

/// Print a full summary: totals by category, monthly breakdown, per-contract,
/// and a monthly trend analysis.
pub fn print_summary(conn: &Connection) {
    let grand = tracker::grand_total(conn).unwrap_or(0.0);

    println!("\n=== EXPENSE SUMMARY ===");
    println!("Grand Total: ${:.2}\n", grand);

    // By category
    let cat_sums = tracker::sum_by_category(conn).unwrap_or_default();
    if cat_sums.is_empty() {
        println!("No expenses recorded.");
        return;
    }
    println!("{:<14} {:>12}", "Category", "Total");
    println!("{}", "-".repeat(28));
    for (cat, total) in &cat_sums {
        println!("{:<14} ${:>11.2}", cat, total);
    }

    // Monthly
    let month_sums = tracker::sum_by_month(conn).unwrap_or_default();
    if !month_sums.is_empty() {
        println!("\n{:<10} {:>12}", "Month", "Total");
        println!("{}", "-".repeat(24));
        for (month, total) in &month_sums {
            println!("{:<10} ${:>11.2}", month, total);
        }
        // Monthly trend analysis
        print_monthly_trends(&month_sums);
    }

    // Per contract
    let contract_sums = tracker::sum_by_contract(conn).unwrap_or_default();
    if !contract_sums.is_empty() {
        println!("\n{:<20} {:>12}", "Contract", "Total");
        println!("{}", "-".repeat(34));
        for (contract, total) in &contract_sums {
            println!("{:<20} ${:>11.2}", contract, total);
        }
    }
}

/// Print a simple month-over-month trend analysis.
fn print_monthly_trends(month_sums: &[(String, f64)]) {
    // month_sums is DESC by month; reverse for chronological order
    let mut chronological: Vec<_> = month_sums.to_vec();
    chronological.reverse();

    if chronological.len() < 2 {
        return; // need at least 2 months for a trend
    }

    println!("\n=== MONTHLY TREND ===");
    println!("{:<10} {:>12}  {:>12}  {:>8}", "Month", "Total", "MoM Change", "Direction");
    println!("{}", "-".repeat(50));

    for i in 0..chronological.len() {
        let (ref month, total) = chronological[i];
        if i == 0 {
            println!("{:<10} ${:>11.2}  {:>12}  —", month, total, "(baseline)");
        } else {
            let prev = chronological[i - 1].1;
            let delta = total - prev;
            let pct = if prev > 0.0 { (delta / prev) * 100.0 } else { 0.0 };
            let direction = if delta > 0.0 { "UP" } else if delta < 0.0 { "DOWN" } else { "FLAT" };
            println!(
                "{:<10} ${:>11.2}  {:>+12.2}  {} ({:+.1}%)",
                month, total, delta, direction, pct
            );
        }
    }
}

// ── Tax year report ───────────────────────────────────────────────────────────

/// Generate a tax-year summary report.
pub fn print_tax_year_report(conn: &Connection, year: u32) {
    let total = tracker::grand_total_for_year(conn, year).unwrap_or(0.0);
    let cat_sums = tracker::sum_by_category_for_year(conn, year).unwrap_or_default();
    let contract_sums = tracker::sum_by_contract_for_year(conn, year).unwrap_or_default();
    let expenses = tracker::list_by_tax_year(conn, year).unwrap_or_default();

    println!("\n=== TAX YEAR {} REPORT ===", year);
    println!("Hoags Inc. — Federal Contracting Expense Summary");
    println!("{}", "=".repeat(60));
    println!("Total Expenses:  ${:.2}", total);
    println!("Total Records:   {}", expenses.len());
    println!();

    if cat_sums.is_empty() {
        println!("No expenses recorded for {}.", year);
        return;
    }

    // Category breakdown with deductibility classification
    println!("{:<14} {:>10}  {:<40}", "Category", "Total", "Tax Treatment");
    println!("{}", "-".repeat(68));
    let mut deductible_total = 0.0;
    let mut review_total = 0.0;
    for (cat, amount) in &cat_sums {
        let label = deductibility_label(cat);
        println!("{:<14} ${:>9.2}  {:<40}", cat, amount, label);
        if label.starts_with("Deductible") {
            deductible_total += amount;
        } else {
            review_total += amount;
        }
    }
    println!("{}", "-".repeat(68));
    println!("{:<14} ${:>9.2}  Likely deductible (verify with CPA)", "Subtotal", deductible_total);
    if review_total > 0.0 {
        println!("{:<14} ${:>9.2}  Requires review", "Review", review_total);
    }

    // Per-contract totals
    if !contract_sums.is_empty() {
        println!();
        println!("{:<24} {:>10}", "Contract", "Expenses");
        println!("{}", "-".repeat(36));
        for (contract, amount) in &contract_sums {
            println!("{:<24} ${:>9.2}", contract, amount);
        }
    }

    // Disclaimer
    println!();
    println!("DISCLAIMER: This report is for record-keeping purposes only.");
    println!("Consult a licensed CPA or tax professional before filing.");
}

// ── CSV export ────────────────────────────────────────────────────────────────

/// Export expenses to CSV on stdout.
pub fn export_csv(expenses: &[Expense]) {
    println!("id,amount,vendor,category,date,contract_number,description,created_at");
    for e in expenses {
        println!(
            "{},{:.2},{},{},{},{},{},{}",
            e.id,
            e.amount,
            csv_escape(&e.vendor),
            csv_escape(&e.category),
            e.date,
            csv_escape(e.contract_number.as_deref().unwrap_or("")),
            csv_escape(e.description.as_deref().unwrap_or("")),
            e.created_at,
        );
    }
}

/// Minimal CSV escaping: wrap in quotes if value contains comma, quote, or newline.
fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csv_escape_plain() {
        assert_eq!(csv_escape("Home Depot"), "Home Depot");
    }

    #[test]
    fn test_csv_escape_comma() {
        assert_eq!(csv_escape("Hoags, Inc"), "\"Hoags, Inc\"");
    }

    #[test]
    fn test_csv_escape_quote() {
        assert_eq!(csv_escape("say \"hi\""), "\"say \"\"hi\"\"\"");
    }

    #[test]
    fn test_export_csv_headers_and_rows() {
        let expenses = vec![Expense {
            id: 1,
            amount: 99.50,
            vendor: "Costco".to_string(),
            category: "supplies".to_string(),
            date: "2026-04-08".to_string(),
            contract_number: Some("W123".to_string()),
            description: None,
            created_at: "2026-04-08T10:00:00Z".to_string(),
        }];
        export_csv(&expenses);
    }

    #[test]
    fn test_deductibility_labels() {
        assert!(deductibility_label("supplies").contains("Deductible"));
        assert!(deductibility_label("fuel").contains("Deductible"));
        assert!(deductibility_label("other").contains("Review"));
        assert!(deductibility_label("random").contains("Review"));
    }

    #[test]
    fn test_monthly_trend_single_month_no_panic() {
        // Should return without printing a trend (not enough data)
        let single = vec![("2026-04".to_string(), 100.0)];
        print_monthly_trends(&single); // must not panic
    }

    #[test]
    fn test_monthly_trend_two_months_no_panic() {
        let two = vec![
            ("2026-04".to_string(), 100.0),
            ("2026-03".to_string(), 80.0),
        ];
        print_monthly_trends(&two); // must not panic
    }
}
