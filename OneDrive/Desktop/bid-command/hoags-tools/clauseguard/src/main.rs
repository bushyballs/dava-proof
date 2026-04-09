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
