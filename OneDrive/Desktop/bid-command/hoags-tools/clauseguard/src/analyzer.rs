//! Contract analyzer — PDF text extraction, FAR/DFARS clause detection, risk pattern scanning.

use std::collections::HashMap;
use std::path::Path;

use lopdf::Document;
use regex::Regex;

use crate::clauses::{build_clause_db, ClauseInfo, RiskLevel};

// ── Risk keyword patterns ─────────────────────────────────────────────────────

/// Phrases that contribute RED risk when found in contract text.
pub const RED_PHRASES: &[&str] = &[
    "liquidated damages",
    "termination for default",
    "default termination",
    "personal liability",
    "excess reprocurement",
    "forfeiture",
    "suspend payments",
    "debarment",
    "criminal penalty",
    "fraud",
    // Additional high-risk phrases
    "indemnify",
    "hold harmless",
    "waive all claims",
    "sole discretion",
    "at contractor's expense",
    "no additional cost to the government",
    "consequential damages",
    "unlimited liability",
    "unconditional guarantee",
];

/// Phrases that contribute YELLOW risk.
pub const YELLOW_PHRASES: &[&str] = &[
    "option to extend",
    "unilateral modification",
    "government property",
    "retainage",
    "withhold",
    "audit rights",
    "certified cost",
    "prevailing wage",
    "wage determination",
    "subcontracting limitation",
    // Additional moderate-risk phrases
    "at no additional charge",
    "at the government's option",
    "as directed by the contracting officer",
    "contractor shall bear",
    "without additional compensation",
    "time is of the essence",
    "at contractor's risk",
];

/// Phrases that are GREEN (favorable to contractor) — lower overall score.
pub const GREEN_PHRASES: &[&str] = &[
    "termination for convenience",
    "disputes",
    "payment terms",
    "prompt payment",
    "equitable adjustment",
    "excusable delay",
];

// ── Per-page analysis ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PageRisk {
    pub page_number: u32,
    /// Raw text (truncated to 2000 chars for JSON output).
    pub text_preview: String,
    pub red_hits: Vec<String>,
    pub yellow_hits: Vec<String>,
    pub green_hits: Vec<String>,
    pub clause_refs: Vec<String>,
    pub risk_score: u32,
    pub risk_level: RiskLevel,
}

impl PageRisk {
    fn compute_score(red: usize, yellow: usize, green: usize) -> u32 {
        let raw = (red as u32 * RiskLevel::Red.weight())
            + (yellow as u32 * RiskLevel::Yellow.weight())
            .saturating_sub(green as u32 * RiskLevel::Green.weight() / 2);
        raw
    }

    fn score_to_level(score: u32) -> RiskLevel {
        if score >= 20 {
            RiskLevel::Red
        } else if score >= 5 {
            RiskLevel::Yellow
        } else {
            RiskLevel::Green
        }
    }
}

// ── Contract-level result ─────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContractAnalysis {
    pub file_path: String,
    pub total_pages: u32,
    pub pages: Vec<PageRisk>,
    pub all_clause_refs: Vec<String>,
    pub known_clauses: Vec<ClauseInfo>,
    pub red_phrase_total: usize,
    pub yellow_phrase_total: usize,
    pub green_phrase_total: usize,
    pub overall_score: u32,
    pub overall_risk: RiskLevel,
    pub summary: String,
}

// ── Analyzer ─────────────────────────────────────────────────────────────────

pub struct Analyzer {
    clause_db: HashMap<String, ClauseInfo>,
    clause_re: Regex,
    red_res: Vec<Regex>,
    yellow_res: Vec<Regex>,
    green_res: Vec<Regex>,
}

impl Analyzer {
    pub fn new() -> Self {
        let clause_db = build_clause_db();

        // Matches both FAR (52.xxx-xx) and DFARS (252.xxx-xxxx) style clause numbers.
        let clause_re =
            Regex::new(r"\b(252\.\d{3}-\d{4}|52\.\d{3}-\d{1,2})\b").expect("clause regex");

        let compile = |phrases: &[&str]| {
            phrases
                .iter()
                .map(|p| {
                    Regex::new(&format!("(?i){}", regex::escape(p))).expect("phrase regex")
                })
                .collect::<Vec<_>>()
        };

        Self {
            clause_db,
            clause_re,
            red_res: compile(RED_PHRASES),
            yellow_res: compile(YELLOW_PHRASES),
            green_res: compile(GREEN_PHRASES),
        }
    }

    /// Analyze a PDF file at `path`. Returns `ContractAnalysis` or an error string.
    pub fn analyze<P: AsRef<Path>>(&self, path: P) -> Result<ContractAnalysis, String> {
        let path_str = path.as_ref().display().to_string();
        let doc = Document::load(path.as_ref())
            .map_err(|e| format!("Failed to load PDF '{}': {e}", path_str))?;

        // get_pages() returns BTreeMap<u32, ObjectId> (1-based page number -> object id)
        let pages_map = doc.get_pages();
        let total_pages = pages_map.len() as u32;

        let mut pages: Vec<PageRisk> = Vec::with_capacity(total_pages as usize);
        let mut all_clause_set: Vec<String> = Vec::new();

        for (page_num, object_id) in &pages_map {
            let page_num = *page_num;

            // Extract text using the ObjectId directly.
            let text = self.extract_page_text_by_id(&doc, *object_id);

            let red_hits = self.find_hits(&text, &self.red_res, RED_PHRASES);
            let yellow_hits = self.find_hits(&text, &self.yellow_res, YELLOW_PHRASES);
            let green_hits = self.find_hits(&text, &self.green_res, GREEN_PHRASES);
            let clause_refs = self.find_clauses(&text);

            all_clause_set.extend(clause_refs.clone());

            let score =
                PageRisk::compute_score(red_hits.len(), yellow_hits.len(), green_hits.len());
            let risk_level = PageRisk::score_to_level(score);

            let text_preview = if text.len() > 2000 {
                text[..2000].to_string()
            } else {
                text.clone()
            };

            pages.push(PageRisk {
                page_number: page_num,
                text_preview,
                red_hits,
                yellow_hits,
                green_hits,
                clause_refs,
                risk_score: score,
                risk_level,
            });
        }

        // Deduplicate clause refs preserving order.
        let mut seen = std::collections::HashSet::new();
        let all_clause_refs: Vec<String> = all_clause_set
            .into_iter()
            .filter(|c| seen.insert(c.clone()))
            .collect();

        // Look up known clauses.
        let known_clauses: Vec<ClauseInfo> = all_clause_refs
            .iter()
            .filter_map(|num| self.clause_db.get(num).cloned())
            .collect();

        let red_total: usize = pages.iter().map(|p| p.red_hits.len()).sum();
        let yellow_total: usize = pages.iter().map(|p| p.yellow_hits.len()).sum();
        let green_total: usize = pages.iter().map(|p| p.green_hits.len()).sum();

        // Add known-clause weight to overall score.
        let clause_score: u32 = known_clauses.iter().map(|c| c.risk.weight()).sum();
        let phrase_score =
            PageRisk::compute_score(red_total, yellow_total, green_total);
        let overall_score = phrase_score + clause_score;
        let overall_risk = if overall_score >= 40 {
            RiskLevel::Red
        } else if overall_score >= 12 {
            RiskLevel::Yellow
        } else {
            RiskLevel::Green
        };

        let summary = format!(
            "{} pages | {} clauses detected ({} known) | {} red / {} yellow / {} green phrase hits | Overall: {}",
            total_pages,
            all_clause_refs.len(),
            known_clauses.len(),
            red_total,
            yellow_total,
            green_total,
            overall_risk.as_str()
        );

        Ok(ContractAnalysis {
            file_path: path_str,
            total_pages,
            pages,
            all_clause_refs,
            known_clauses,
            red_phrase_total: red_total,
            yellow_phrase_total: yellow_total,
            green_phrase_total: green_total,
            overall_score,
            overall_risk,
            summary,
        })
    }

    /// Compare two PDFs, returning a diff of their detected clauses.
    pub fn compare<P: AsRef<Path>>(
        &self,
        path1: P,
        path2: P,
    ) -> Result<ContractDiff, String> {
        let a = self.analyze(path1)?;
        let b = self.analyze(path2)?;

        let a_set: std::collections::HashSet<String> =
            a.all_clause_refs.iter().cloned().collect();
        let b_set: std::collections::HashSet<String> =
            b.all_clause_refs.iter().cloned().collect();

        let only_in_a: Vec<String> = a_set.difference(&b_set).cloned().collect();
        let only_in_b: Vec<String> = b_set.difference(&a_set).cloned().collect();
        let in_both: Vec<String> = a_set.intersection(&b_set).cloned().collect();

        let risk_delta: i32 = b.overall_score as i32 - a.overall_score as i32;

        Ok(ContractDiff {
            file_a: a.file_path.clone(),
            file_b: b.file_path.clone(),
            analysis_a: a,
            analysis_b: b,
            only_in_a,
            only_in_b,
            in_both,
            risk_delta,
        })
    }

    /// Check whether a specific clause number appears in the PDF.
    pub fn check_clause<P: AsRef<Path>>(
        &self,
        path: P,
        clause_number: &str,
    ) -> Result<ClauseCheckResult, String> {
        let analysis = self.analyze(path)?;
        let found = analysis
            .all_clause_refs
            .contains(&clause_number.to_string());
        let info = self.clause_db.get(clause_number).cloned();
        // Find pages where it appears.
        let pages_found: Vec<u32> = analysis
            .pages
            .iter()
            .filter(|p| p.clause_refs.contains(&clause_number.to_string()))
            .map(|p| p.page_number)
            .collect();

        Ok(ClauseCheckResult {
            clause_number: clause_number.to_string(),
            found,
            pages_found,
            info,
        })
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    fn extract_page_text_by_id(&self, doc: &Document, object_id: lopdf::ObjectId) -> String {
        if let Ok(content) = doc.get_page_content(object_id) {
            return extract_readable_text(&content);
        }
        String::new()
    }

    fn find_hits(&self, text: &str, patterns: &[Regex], phrases: &[&str]) -> Vec<String> {
        let mut hits = Vec::new();
        for (re, phrase) in patterns.iter().zip(phrases.iter()) {
            if re.is_match(text) {
                hits.push(phrase.to_string());
            }
        }
        hits
    }

    fn find_clauses(&self, text: &str) -> Vec<String> {
        let mut found = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for cap in self.clause_re.find_iter(text) {
            let s = cap.as_str().to_string();
            if seen.insert(s.clone()) {
                found.push(s);
            }
        }
        found
    }
}

impl Default for Analyzer {
    fn default() -> Self {
        Self::new()
    }
}

// ── Supporting result types ───────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClauseCheckResult {
    pub clause_number: String,
    pub found: bool,
    pub pages_found: Vec<u32>,
    pub info: Option<ClauseInfo>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContractDiff {
    pub file_a: String,
    pub file_b: String,
    pub analysis_a: ContractAnalysis,
    pub analysis_b: ContractAnalysis,
    /// Clauses present in A but not B.
    pub only_in_a: Vec<String>,
    /// Clauses present in B but not A.
    pub only_in_b: Vec<String>,
    pub in_both: Vec<String>,
    /// Positive = B is riskier than A.
    pub risk_delta: i32,
}

// ── PDF text helper ───────────────────────────────────────────────────────────

/// Extract human-readable text from raw PDF content stream bytes.
/// Uses a simple state machine that collects text from Tj/TJ operators.
pub fn extract_readable_text(content: &[u8]) -> String {
    // Decode as UTF-8 lossy, then extract runs from parenthesised strings and BT..ET blocks.
    let raw = String::from_utf8_lossy(content);
    let mut out = String::with_capacity(raw.len() / 2);

    // State: inside a parenthesised string literal
    let mut in_str = false;
    let mut escape = false;
    let mut chars = raw.chars().peekable();

    while let Some(ch) = chars.next() {
        if escape {
            escape = false;
            if in_str {
                match ch {
                    'n' => out.push('\n'),
                    'r' => out.push('\r'),
                    't' => out.push('\t'),
                    _ => out.push(ch),
                }
            }
            continue;
        }
        match ch {
            '\\' => {
                escape = true;
            }
            '(' if !in_str => {
                in_str = true;
            }
            ')' if in_str => {
                in_str = false;
                out.push(' ');
            }
            _ if in_str => {
                if ch.is_ascii_graphic() || ch == ' ' {
                    out.push(ch);
                }
            }
            _ => {}
        }
    }

    // Clean up: collapse whitespace runs.
    let cleaned = out
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    cleaned
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_text_from_pdf_stream() {
        // Simple parenthesised string as seen in PDF content streams.
        let stream = b"BT (Hello, World!) Tj (FAR 52.249-8) Tj ET";
        let text = extract_readable_text(stream);
        assert!(text.contains("Hello, World!"), "got: {text}");
        assert!(text.contains("FAR 52.249-8"), "got: {text}");
    }

    #[test]
    fn clause_regex_matches_far_and_dfars() {
        let a = Analyzer::new();
        let text = "See 52.222-41 and also 252.204-7012 for details.";
        let clauses = a.find_clauses(text);
        assert!(clauses.contains(&"52.222-41".to_string()));
        assert!(clauses.contains(&"252.204-7012".to_string()));
    }

    #[test]
    fn red_phrase_detected() {
        let a = Analyzer::new();
        let text = "This contract includes liquidated damages provisions.";
        let hits = a.find_hits(text, &a.red_res, RED_PHRASES);
        assert!(hits.contains(&"liquidated damages".to_string()), "got: {hits:?}");
    }

    #[test]
    fn green_phrase_detected() {
        let a = Analyzer::new();
        let text = "Termination for convenience clause applies here.";
        let hits = a.find_hits(text, &a.green_res, GREEN_PHRASES);
        assert!(hits.contains(&"termination for convenience".to_string()));
    }

    #[test]
    fn page_risk_score_red_threshold() {
        // 2 red hits, no yellow, no green → 20 → RED
        let score = PageRisk::compute_score(2, 0, 0);
        assert_eq!(PageRisk::score_to_level(score), RiskLevel::Red);
    }

    #[test]
    fn page_risk_score_green_threshold() {
        // 0 hits → GREEN
        let score = PageRisk::compute_score(0, 0, 0);
        assert_eq!(PageRisk::score_to_level(score), RiskLevel::Green);
    }

    #[test]
    fn clause_dedup_in_find() {
        let a = Analyzer::new();
        let text = "52.222-41 and again 52.222-41 here.";
        let clauses = a.find_clauses(text);
        assert_eq!(clauses.len(), 1, "should deduplicate");
    }

    // ── New phrase-based risk detection tests ─────────────────────────────────

    #[test]
    fn new_red_phrase_indemnify_detected() {
        let a = Analyzer::new();
        let text = "The contractor shall indemnify the Government against all third-party claims.";
        let hits = a.find_hits(text, &a.red_res, RED_PHRASES);
        assert!(hits.contains(&"indemnify".to_string()), "got: {hits:?}");
    }

    #[test]
    fn new_red_phrase_hold_harmless_detected() {
        let a = Analyzer::new();
        let text = "Contractor agrees to hold harmless the United States from any liability.";
        let hits = a.find_hits(text, &a.red_res, RED_PHRASES);
        assert!(hits.contains(&"hold harmless".to_string()), "got: {hits:?}");
    }

    #[test]
    fn new_red_phrase_sole_discretion_detected() {
        let a = Analyzer::new();
        let text = "The Contracting Officer may, at sole discretion, require additional work.";
        let hits = a.find_hits(text, &a.red_res, RED_PHRASES);
        assert!(hits.contains(&"sole discretion".to_string()), "got: {hits:?}");
    }

    #[test]
    fn new_red_phrase_at_contractors_expense_detected() {
        let a = Analyzer::new();
        let text = "Any re-performance shall be done at contractor's expense.";
        let hits = a.find_hits(text, &a.red_res, RED_PHRASES);
        assert!(hits.contains(&"at contractor's expense".to_string()), "got: {hits:?}");
    }

    #[test]
    fn new_red_phrase_no_additional_cost_detected() {
        let a = Analyzer::new();
        let text = "Repairs must be completed at no additional cost to the government.";
        let hits = a.find_hits(text, &a.red_res, RED_PHRASES);
        assert!(hits.contains(&"no additional cost to the government".to_string()), "got: {hits:?}");
    }

    #[test]
    fn new_yellow_phrase_time_is_of_essence_detected() {
        let a = Analyzer::new();
        let text = "Time is of the essence for all deliverables under this contract.";
        let hits = a.find_hits(text, &a.yellow_res, YELLOW_PHRASES);
        assert!(hits.contains(&"time is of the essence".to_string()), "got: {hits:?}");
    }

    // ── Integration test: realistic contract text → risk detection ─────────────

    /// Simulate analysis of contract text containing multiple risk indicators
    /// by exercising the phrase and clause detection on inline text.
    #[test]
    fn integration_realistic_contract_text_risk_detection() {
        let a = Analyzer::new();

        let contract_text = "\
            CONTRACT NO. W912HN-26-C-0042 \
            This contract incorporates FAR clauses 52.249-8 (Default), 52.211-11 (Liquidated Damages), \
            52.222-41 (Service Contract Labor Standards), and DFARS 252.204-7012. \
            \
            SECTION H — SPECIAL CONTRACT REQUIREMENTS \
            H.1 The contractor shall indemnify and hold harmless the Government from all claims \
            arising out of contractor negligence. \
            H.2 Time is of the essence for all deliverables. Failure to deliver on schedule will \
            trigger liquidated damages of $500 per calendar day. \
            H.3 Any re-performance required due to defective work shall be at contractor's expense \
            and at no additional cost to the government. \
            H.4 The Contracting Officer, at sole discretion, may direct contractor to perform \
            additional cleanup tasks without additional compensation. \
            H.5 Termination for default may be issued with 10 days notice. Excess reprocurement \
            costs shall be charged to the defaulting contractor. \
            H.6 Option to extend services may be exercised by the Government with 30-day notice. \
            H.7 Prevailing wage rates per current wage determination apply to all service employees. \
        ";

        // Clause detection
        let clauses = a.find_clauses(contract_text);
        assert!(clauses.contains(&"52.249-8".to_string()), "should find 52.249-8");
        assert!(clauses.contains(&"52.211-11".to_string()), "should find 52.211-11");
        assert!(clauses.contains(&"52.222-41".to_string()), "should find 52.222-41");
        assert!(clauses.contains(&"252.204-7012".to_string()), "should find 252.204-7012");

        // Red phrase detection
        let red_hits = a.find_hits(contract_text, &a.red_res, RED_PHRASES);
        assert!(red_hits.contains(&"indemnify".to_string()), "should detect indemnify");
        assert!(red_hits.contains(&"hold harmless".to_string()), "should detect hold harmless");
        assert!(red_hits.contains(&"liquidated damages".to_string()), "should detect liquidated damages");
        assert!(red_hits.contains(&"sole discretion".to_string()), "should detect sole discretion");
        assert!(red_hits.contains(&"at contractor's expense".to_string()), "should detect at contractor's expense");
        assert!(red_hits.contains(&"no additional cost to the government".to_string()), "should detect no additional cost");
        assert!(red_hits.contains(&"excess reprocurement".to_string()), "should detect excess reprocurement");
        assert!(red_hits.contains(&"termination for default".to_string()), "should detect termination for default");

        // Yellow phrase detection
        let yellow_hits = a.find_hits(contract_text, &a.yellow_res, YELLOW_PHRASES);
        assert!(yellow_hits.contains(&"option to extend".to_string()), "should detect option to extend");
        assert!(yellow_hits.contains(&"prevailing wage".to_string()), "should detect prevailing wage");
        assert!(yellow_hits.contains(&"without additional compensation".to_string()), "should detect without additional compensation");
        assert!(yellow_hits.contains(&"time is of the essence".to_string()), "should detect time is of the essence");

        // Score should be RED-level
        let score = PageRisk::compute_score(red_hits.len(), yellow_hits.len(), 0);
        assert_eq!(
            PageRisk::score_to_level(score),
            RiskLevel::Red,
            "realistic high-risk contract should score RED, score={}",
            score
        );
    }

    // ── Summary paragraph test ─────────────────────────────────────────────────

    // ── 7 new analyzer tests ─────────────────────────────────────────────────

    #[test]
    fn test_analyze_empty_text() {
        // extract_readable_text on an empty slice should return an empty string
        let text = extract_readable_text(b"");
        assert!(text.is_empty(), "empty content should give empty text");
    }

    #[test]
    fn test_analyze_no_clauses_green() {
        // Text with no risk phrases and no clause numbers should score GREEN
        let a = Analyzer::new();
        let text = "This is a standard statement of work for routine cleaning services.";
        let red = a.find_hits(text, &a.red_res, RED_PHRASES);
        let yellow = a.find_hits(text, &a.yellow_res, YELLOW_PHRASES);
        let green_hits = a.find_hits(text, &a.green_res, GREEN_PHRASES);
        let score = PageRisk::compute_score(red.len(), yellow.len(), green_hits.len());
        assert_eq!(PageRisk::score_to_level(score), RiskLevel::Green);
    }

    #[test]
    fn test_analyze_single_red_clause() {
        let a = Analyzer::new();
        let text = "Contractor shall not seek excess reprocurement costs under any circumstance.";
        let hits = a.find_hits(text, &a.red_res, RED_PHRASES);
        assert!(!hits.is_empty(), "should detect at least one red phrase");
        let score = PageRisk::compute_score(hits.len(), 0, 0);
        assert!(score >= RiskLevel::Red.weight(), "single red hit should push score above 0");
    }

    #[test]
    fn test_far_pattern_matches() {
        let a = Analyzer::new();
        let text = "Clause 52.222-41 applies to all service employees.";
        let clauses = a.find_clauses(text);
        assert!(clauses.contains(&"52.222-41".to_string()), "should match FAR 52.222-41");
    }

    #[test]
    fn test_dfars_pattern_matches() {
        let a = Analyzer::new();
        let text = "Contractor must comply with 252.204-7012 cybersecurity requirements.";
        let clauses = a.find_clauses(text);
        assert!(clauses.contains(&"252.204-7012".to_string()), "should match DFARS 252.204-7012");
    }

    #[test]
    fn test_phrase_detection_indemnify() {
        let a = Analyzer::new();
        let text = "The contractor must indemnify the Government from third-party claims.";
        let hits = a.find_hits(text, &a.red_res, RED_PHRASES);
        assert!(hits.contains(&"indemnify".to_string()), "indemnify not found in: {hits:?}");
    }

    #[test]
    fn test_phrase_detection_hold_harmless() {
        let a = Analyzer::new();
        let text = "Contractor agrees to hold harmless and defend the United States.";
        let hits = a.find_hits(text, &a.red_res, RED_PHRASES);
        assert!(hits.contains(&"hold harmless".to_string()), "hold harmless not found in: {hits:?}");
    }

    #[test]
    fn test_risk_score_calculation() {
        // 3 red hits, 2 yellow, 0 green — should be RED level
        let score = PageRisk::compute_score(3, 2, 0);
        assert_eq!(PageRisk::score_to_level(score), RiskLevel::Red,
            "3 red + 2 yellow should be RED, score={}", score);
    }

    #[test]
    fn test_summary_paragraph_contains_risk_level() {
        use crate::report::build_risk_paragraph;
        use crate::clauses::ClauseInfo;

        let analysis = crate::analyzer::ContractAnalysis {
            file_path: "test.pdf".to_string(),
            total_pages: 5,
            pages: vec![],
            all_clause_refs: vec!["52.249-8".to_string(), "52.222-41".to_string()],
            known_clauses: vec![
                ClauseInfo {
                    number: "52.249-8".to_string(),
                    title: "Default".to_string(),
                    risk: RiskLevel::Red,
                    description: "desc".to_string(),
                    recommendation: "rec".to_string(),
                },
                ClauseInfo {
                    number: "52.222-41".to_string(),
                    title: "Service Contract Labor Standards".to_string(),
                    risk: RiskLevel::Yellow,
                    description: "desc".to_string(),
                    recommendation: "rec".to_string(),
                },
            ],
            red_phrase_total: 3,
            yellow_phrase_total: 2,
            green_phrase_total: 1,
            overall_score: 55,
            overall_risk: RiskLevel::Red,
            summary: "test summary".to_string(),
        };

        let para = build_risk_paragraph(&analysis);
        assert!(para.contains("HIGH"), "should mention HIGH risk: {para}");
        assert!(para.contains("52.249-8"), "should mention red clause: {para}");
        assert!(para.contains("52.222-41"), "should mention yellow clause: {para}");
        assert!(para.contains("legal review"), "should recommend legal review: {para}");
    }
}
