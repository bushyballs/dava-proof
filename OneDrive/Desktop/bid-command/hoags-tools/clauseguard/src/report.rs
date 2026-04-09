//! Report generation — terminal (colored) and JSON output.

use crate::analyzer::{ClauseCheckResult, ContractAnalysis, ContractDiff};
use crate::clauses::RiskLevel;

// ANSI color codes
const RED: &str = "\x1b[31;1m";
const YELLOW: &str = "\x1b[33;1m";
const GREEN: &str = "\x1b[32;1m";
const CYAN: &str = "\x1b[36;1m";
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";

fn risk_color(risk: &RiskLevel) -> &'static str {
    match risk {
        RiskLevel::Red => RED,
        RiskLevel::Yellow => YELLOW,
        RiskLevel::Green => GREEN,
    }
}

fn risk_badge(risk: &RiskLevel) -> String {
    let color = risk_color(risk);
    format!("{color}[{}]{RESET}", risk.as_str())
}

// ── Analysis terminal report ─────────────────────────────────────────────────

pub fn print_analysis(analysis: &ContractAnalysis) {
    let border = "═".repeat(70);
    println!("\n{BOLD}{CYAN}{border}{RESET}");
    println!(
        "{BOLD}  CLAUSEGUARD — Contract Risk Analysis{RESET}"
    );
    println!("{BOLD}{CYAN}{border}{RESET}");

    println!("\n{BOLD}File:{RESET}    {}", analysis.file_path);
    println!("{BOLD}Pages:{RESET}   {}", analysis.total_pages);
    println!(
        "{BOLD}Risk:{RESET}    {} (score {})",
        risk_badge(&analysis.overall_risk),
        analysis.overall_score
    );
    println!("{BOLD}Summary:{RESET} {}", analysis.summary);

    // ── Known clauses ────────────────────────────────────────────────────────
    if !analysis.known_clauses.is_empty() {
        println!("\n{BOLD}{CYAN}── Identified FAR/DFARS Clauses ─────────────────────────────────{RESET}");
        for ci in &analysis.known_clauses {
            let badge = risk_badge(&ci.risk);
            println!("  {} {BOLD}{}{RESET}  {}", badge, ci.number, ci.title);
            println!("     {DIM}{}{RESET}", ci.description);
            println!("     {BOLD}→ {}{RESET}", ci.recommendation);
            println!();
        }
    }

    // ── Unrecognised clause refs ─────────────────────────────────────────────
    let unknown: Vec<&str> = analysis
        .all_clause_refs
        .iter()
        .filter(|r| !analysis.known_clauses.iter().any(|k| &k.number == *r))
        .map(|s| s.as_str())
        .collect();
    if !unknown.is_empty() {
        println!("{BOLD}{CYAN}── Unrecognised Clause References ───────────────────────────────{RESET}");
        for u in &unknown {
            println!("  {DIM}{u}{RESET}");
        }
        println!();
    }

    // ── Risk phrase summary ──────────────────────────────────────────────────
    println!("{BOLD}{CYAN}── Risk Phrase Hits ──────────────────────────────────────────────{RESET}");
    println!(
        "  {RED}RED{RESET}    phrases: {}",
        analysis.red_phrase_total
    );
    println!(
        "  {YELLOW}YELLOW{RESET} phrases: {}",
        analysis.yellow_phrase_total
    );
    println!(
        "  {GREEN}GREEN{RESET}  phrases: {}",
        analysis.green_phrase_total
    );
    println!();

    // ── Per-page breakdown (only non-green pages) ────────────────────────────
    let flagged: Vec<_> = analysis
        .pages
        .iter()
        .filter(|p| p.risk_level != RiskLevel::Green || !p.clause_refs.is_empty())
        .collect();

    if !flagged.is_empty() {
        println!("{BOLD}{CYAN}── Flagged Pages ────────────────────────────────────────────────{RESET}");
        for page in &flagged {
            print!(
                "  Page {:>3}  {}  score={}",
                page.page_number,
                risk_badge(&page.risk_level),
                page.risk_score
            );
            if !page.clause_refs.is_empty() {
                print!("  clauses: {}", page.clause_refs.join(", "));
            }
            println!();
            if !page.red_hits.is_empty() {
                println!("    {RED}▶ {}{RESET}", page.red_hits.join(", "));
            }
            if !page.yellow_hits.is_empty() {
                println!("    {YELLOW}▶ {}{RESET}", page.yellow_hits.join(", "));
            }
        }
        println!();
    }

    println!("{BOLD}{CYAN}{border}{RESET}\n");
}

// ── Clause check terminal report ─────────────────────────────────────────────

pub fn print_clause_check(result: &ClauseCheckResult) {
    println!("\n{BOLD}Clause Check: {}{RESET}", result.clause_number);
    if result.found {
        let pages = result
            .pages_found
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        println!("  {GREEN}FOUND{RESET} on page(s): {pages}");
        if let Some(ci) = &result.info {
            println!(
                "  {} {BOLD}{}{RESET}",
                risk_badge(&ci.risk),
                ci.title
            );
            println!("  {DIM}{}{RESET}", ci.description);
            println!("  {BOLD}→ {}{RESET}", ci.recommendation);
        } else {
            println!("  {DIM}(clause not in local database){RESET}");
        }
    } else {
        println!("  {DIM}NOT FOUND in this contract.{RESET}");
    }
    println!();
}

// ── Diff terminal report ─────────────────────────────────────────────────────

pub fn print_diff(diff: &ContractDiff) {
    let border = "─".repeat(70);
    println!("\n{BOLD}{CYAN}CLAUSEGUARD — Contract Comparison{RESET}");
    println!("{CYAN}{border}{RESET}");
    println!("  A: {}", diff.file_a);
    println!("  B: {}", diff.file_b);
    println!();

    let delta_str = if diff.risk_delta > 0 {
        format!("{RED}+{}{RESET} (B is riskier)", diff.risk_delta)
    } else if diff.risk_delta < 0 {
        format!("{GREEN}{}{RESET} (B is safer)", diff.risk_delta)
    } else {
        format!("{GREEN}0{RESET} (same risk score)")
    };
    println!("  Risk delta (B − A): {delta_str}");
    println!(
        "  A overall: {} (score {})",
        risk_badge(&diff.analysis_a.overall_risk),
        diff.analysis_a.overall_score
    );
    println!(
        "  B overall: {} (score {})",
        risk_badge(&diff.analysis_b.overall_risk),
        diff.analysis_b.overall_score
    );
    println!();

    if !diff.only_in_a.is_empty() {
        println!("{BOLD}Clauses only in A:{RESET}");
        for c in &diff.only_in_a {
            println!("  {DIM}-{RESET} {c}");
        }
        println!();
    }
    if !diff.only_in_b.is_empty() {
        println!("{BOLD}Clauses only in B:{RESET}");
        for c in &diff.only_in_b {
            println!("  {GREEN}+{RESET} {c}");
        }
        println!();
    }
    if !diff.in_both.is_empty() {
        println!("{BOLD}Clauses in both:{RESET}");
        let mut shared = diff.in_both.clone();
        shared.sort();
        for c in &shared {
            println!("  {DIM}={RESET} {c}");
        }
        println!();
    }

    println!("{CYAN}{border}{RESET}\n");
}

// ── JSON output ───────────────────────────────────────────────────────────────

pub fn to_json_analysis(analysis: &ContractAnalysis) -> Result<String, String> {
    serde_json::to_string_pretty(analysis).map_err(|e| e.to_string())
}

pub fn to_json_clause_check(result: &ClauseCheckResult) -> Result<String, String> {
    serde_json::to_string_pretty(result).map_err(|e| e.to_string())
}

pub fn to_json_diff(diff: &ContractDiff) -> Result<String, String> {
    serde_json::to_string_pretty(diff).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clauses::RiskLevel;

    #[test]
    fn risk_badge_contains_level() {
        let b = risk_badge(&RiskLevel::Red);
        assert!(b.contains("RED"), "badge: {b}");
    }

    #[test]
    fn clause_check_json_serializable() {
        let r = ClauseCheckResult {
            clause_number: "52.249-8".to_string(),
            found: true,
            pages_found: vec![3, 7],
            info: None,
        };
        let json = to_json_clause_check(&r).unwrap();
        assert!(json.contains("52.249-8"));
        assert!(json.contains("true"));
    }
}
