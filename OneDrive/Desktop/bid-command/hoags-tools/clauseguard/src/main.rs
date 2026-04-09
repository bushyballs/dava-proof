//! clauseguard — FAR/DFARS contract risk analyzer
//!
//! Usage:
//!   clauseguard analyze <pdf>
//!   clauseguard analyze <pdf> --threshold red
//!   clauseguard check <pdf> --clause "52.222-41"
//!   clauseguard compare <pdf1> <pdf2>
//!   clauseguard summary <pdf>

mod analyzer;
mod clauses;
mod report;

use clap::{Parser, Subcommand, ValueEnum};
use rayon::prelude::*;
use std::path::PathBuf;

use analyzer::Analyzer;
use clauses::RiskLevel;
use hoags_core::bus::EventBus;
use report::{
    print_analysis, print_clause_check, print_diff, print_summary, to_json_analysis,
    to_json_clause_check, to_json_diff, to_json_summary,
};

// ── CLI definition ────────────────────────────────────────────────────────────

/// Risk threshold filter — only show items at or above this level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum ThresholdArg {
    Red,
    Yellow,
    Green,
}

impl ThresholdArg {
    fn to_risk_level(self) -> RiskLevel {
        match self {
            ThresholdArg::Red => RiskLevel::Red,
            ThresholdArg::Yellow => RiskLevel::Yellow,
            ThresholdArg::Green => RiskLevel::Green,
        }
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "clauseguard",
    version = env!("CARGO_PKG_VERSION"),
    about = "FAR/DFARS contract clause analyzer — risk scoring for federal procurement PDFs"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Output machine-readable JSON instead of terminal report.
    #[arg(long, global = true)]
    json: bool,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Analyze a contract PDF for risky FAR/DFARS clauses and phrases.
    Analyze {
        /// Path to the contract PDF file.
        pdf: PathBuf,

        /// Only show clauses and phrases at or above this risk level.
        #[arg(long, value_enum)]
        threshold: Option<ThresholdArg>,
    },

    /// Batch analyze all PDFs in a directory (parallel via rayon), output summary of riskiest documents.
    Batch {
        /// Directory containing contract PDFs to analyze.
        dir: PathBuf,

        /// Only show clauses and phrases at or above this risk level.
        #[arg(long, value_enum)]
        threshold: Option<ThresholdArg>,
    },

    /// Check whether a specific FAR/DFARS clause number appears in the contract.
    Check {
        /// Path to the contract PDF file.
        pdf: PathBuf,

        /// Clause number to search for (e.g. "52.222-41" or "252.204-7012").
        #[arg(long)]
        clause: String,
    },

    /// Compare two contract PDFs and diff their clause sets and risk scores.
    Compare {
        /// First contract PDF.
        pdf1: PathBuf,
        /// Second contract PDF.
        pdf2: PathBuf,
    },

    /// Print a one-paragraph plain-English risk summary of a contract PDF.
    Summary {
        /// Path to the contract PDF file.
        pdf: PathBuf,
    },
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();
    let analyzer = Analyzer::new();

    match cli.command {
        Command::Analyze { pdf, threshold } => {
            match analyzer.analyze(&pdf) {
                Ok(analysis) => {
                    let min_risk = threshold.map(|t| t.to_risk_level());

                    // Publish bus event
                    if let Ok(bus) = EventBus::open_default() {
                        let score = analysis.overall_score;
                        let count = analysis.all_clause_refs.len();
                        let pdf_path = pdf.display().to_string();
                        let risk = if score > 50 { "red" } else if score > 20 { "yellow" } else { "green" };
                        bus.publish("clauseguard", "clauseguard.clause_analyzed", &serde_json::json!({
                            "pdf": pdf_path, "risk_level": risk, "score": score, "clauses_found": count
                        }).to_string());
                    }

                    if cli.json {
                        match to_json_analysis(&analysis) {
                            Ok(j) => println!("{j}"),
                            Err(e) => {
                                eprintln!("JSON error: {e}");
                                std::process::exit(1);
                            }
                        }
                    } else {
                        print_analysis(&analysis, min_risk.as_ref());
                        // Exit code reflects overall risk: 0=green, 1=yellow, 2=red
                        let code = match analysis.overall_risk {
                            RiskLevel::Green => 0,
                            RiskLevel::Yellow => 1,
                            RiskLevel::Red => 2,
                        };
                        std::process::exit(code);
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(3);
                }
            }
        }

        Command::Batch { dir, threshold } => {
            if !dir.exists() || !dir.is_dir() {
                eprintln!("Error: directory not found: {}", dir.display());
                std::process::exit(1);
            }

            // Collect all PDF files in the directory
            let pdf_files: Vec<_> = std::fs::read_dir(&dir)
                .map(|rd| {
                    rd.filter_map(|e| e.ok())
                        .map(|e| e.path())
                        .filter(|p| {
                            p.extension()
                                .and_then(|e| e.to_str())
                                .map(|e| e.eq_ignore_ascii_case("pdf"))
                                .unwrap_or(false)
                        })
                        .collect()
                })
                .unwrap_or_default();

            if pdf_files.is_empty() {
                eprintln!("No PDF files found in {}", dir.display());
                std::process::exit(1);
            }

            // Analyze all PDFs in parallel
            let analyses: Vec<_> = pdf_files
                .par_iter()
                .filter_map(|pdf_path| {
                    analyzer.analyze(pdf_path).ok().map(|analysis| {
                        (pdf_path.clone(), analysis)
                    })
                })
                .collect();

            // Sort by risk score (highest first)
            let mut sorted = analyses;
            sorted.sort_by(|a, b| b.1.overall_score.partial_cmp(&a.1.overall_score).unwrap_or(std::cmp::Ordering::Equal));

            // Print summary of riskiest documents
            println!("Batch Analysis Summary: {} PDFs analyzed\n", sorted.len());
            println!("{:<50} {:<10} {:<6}", "Document", "Risk Level", "Score");
            println!("{}", "─".repeat(70));

            // Helper to compare risk levels: Red=2, Yellow=1, Green=0
            fn risk_score(level: &RiskLevel) -> u8 {
                match level {
                    RiskLevel::Red => 2,
                    RiskLevel::Yellow => 1,
                    RiskLevel::Green => 0,
                }
            }

            let min_risk_level = threshold.map(|t| t.to_risk_level());
            for (pdf_path, analysis) in &sorted {
                if let Some(ref min_risk) = min_risk_level {
                    if risk_score(&analysis.overall_risk) < risk_score(min_risk) {
                        continue;
                    }
                }

                let risk_label = match analysis.overall_risk {
                    RiskLevel::Red => "RED",
                    RiskLevel::Yellow => "YELLOW",
                    RiskLevel::Green => "GREEN",
                };

                let file_name = pdf_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");
                println!("{:<50} {:<10} {:<6}", file_name, risk_label, analysis.overall_score);
            }

            if cli.json {
                // Output as JSON if requested
                let json_results: Vec<_> = sorted.iter().map(|(path, analysis)| {
                    serde_json::json!({
                        "pdf": path.display().to_string(),
                        "risk_level": match analysis.overall_risk {
                            RiskLevel::Red => "red",
                            RiskLevel::Yellow => "yellow",
                            RiskLevel::Green => "green",
                        },
                        "score": analysis.overall_score,
                        "clauses_found": analysis.all_clause_refs.len(),
                    })
                }).collect();
                println!("\n{}", serde_json::to_string_pretty(&json_results).unwrap_or_default());
            }

            std::process::exit(0);
        }

        Command::Check { pdf, clause } => {
            match analyzer.check_clause(&pdf, &clause) {
                Ok(result) => {
                    if cli.json {
                        match to_json_clause_check(&result) {
                            Ok(j) => println!("{j}"),
                            Err(e) => {
                                eprintln!("JSON error: {e}");
                                std::process::exit(1);
                            }
                        }
                    } else {
                        print_clause_check(&result);
                        std::process::exit(if result.found { 0 } else { 1 });
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(3);
                }
            }
        }

        Command::Compare { pdf1, pdf2 } => {
            match analyzer.compare(&pdf1, &pdf2) {
                Ok(diff) => {
                    if cli.json {
                        match to_json_diff(&diff) {
                            Ok(j) => println!("{j}"),
                            Err(e) => {
                                eprintln!("JSON error: {e}");
                                std::process::exit(1);
                            }
                        }
                    } else {
                        print_diff(&diff);
                        // Exit code: 0 = same or safer, 1 = riskier
                        std::process::exit(if diff.risk_delta > 0 { 1 } else { 0 });
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(3);
                }
            }
        }

        Command::Summary { pdf } => {
            match analyzer.analyze(&pdf) {
                Ok(analysis) => {
                    if cli.json {
                        match to_json_summary(&analysis) {
                            Ok(j) => println!("{j}"),
                            Err(e) => {
                                eprintln!("JSON error: {e}");
                                std::process::exit(1);
                            }
                        }
                    } else {
                        print_summary(&analysis);
                        let code = match analysis.overall_risk {
                            RiskLevel::Green => 0,
                            RiskLevel::Yellow => 1,
                            RiskLevel::Red => 2,
                        };
                        std::process::exit(code);
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(3);
                }
            }
        }
    }
}
