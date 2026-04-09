//! invoicer — Government invoice generator for Hoags Inc.
//!
//! Commands:
//!   generate  --contract <json_path> --period <YYYY-MM>
//!   status    --contract <json_path>
//!   list

mod generate;
mod invoice;
mod tracker;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

use generate::generate_pdf;
use invoice::{build_invoice, parse_contract};
use tracker::Tracker;

// ── CLI definition ────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "invoicer",
    about = "Hoags Inc. government invoice generator",
    version = "0.1.0"
)]
struct Cli {
    /// Path to SQLite tracker database (default: invoices.db)
    #[arg(long, default_value = "invoices.db", global = true)]
    db: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a new invoice for a billing period
    Generate {
        /// Path to contract JSON file (or inline JSON string)
        #[arg(long)]
        contract: String,

        /// Billing period as YYYY-MM (e.g. 2026-04)
        #[arg(long)]
        period: String,

        /// Output PDF path (default: <invoice_number>.pdf)
        #[arg(long)]
        out: Option<String>,

        /// Emit invoice JSON to stdout instead of writing a PDF
        #[arg(long)]
        json: bool,
    },

    /// Show invoiced vs. outstanding amounts for a contract
    Status {
        /// Path to contract JSON file (or inline JSON string)
        #[arg(long)]
        contract: String,
    },

    /// List all generated invoices
    List {
        /// Filter by contract number
        #[arg(long)]
        contract: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Load contract JSON — either read from a file or treat the arg as raw JSON.
fn load_contract_json(path_or_json: &str) -> Result<String, String> {
    let p = PathBuf::from(path_or_json);
    if p.exists() {
        std::fs::read_to_string(&p)
            .map_err(|e| format!("Cannot read '{}': {e}", p.display()))
    } else {
        // Treat as inline JSON
        Ok(path_or_json.to_string())
    }
}

// ── Command implementations ───────────────────────────────────────────────────

fn cmd_generate(
    db_path: &str,
    contract_arg: &str,
    period: &str,
    out: Option<&str>,
    emit_json: bool,
) -> Result<(), String> {
    let json = load_contract_json(contract_arg)?;
    let contract = parse_contract(&json)?;

    let tracker = Tracker::open(db_path).map_err(|e| format!("DB error: {e}"))?;

    // Sequence is counted by YYYY-MM period prefix
    let seq = tracker
        .next_sequence(&contract.contract_number, period)
        .map_err(|e| format!("Sequence error: {e}"))?;

    let inv = build_invoice(&contract, period, seq)?;

    if emit_json {
        println!("{}", serde_json::to_string_pretty(&inv).unwrap());
    } else {
        // Determine output path
        let pdf_path = match out {
            Some(o) => o.to_string(),
            None => format!("{}.pdf", inv.invoice_number),
        };

        generate_pdf(&inv, &pdf_path)?;
        println!("PDF written: {pdf_path}");
    }

    // Persist to tracker
    let lines: Vec<(String, String, f64, f64, f64)> = inv
        .lines
        .iter()
        .map(|l| {
            (
                l.clin.clone(),
                l.description.clone(),
                l.qty,
                l.unit_price,
                l.amount,
            )
        })
        .collect();

    tracker
        .insert_invoice(
            &contract.contract_number,
            &inv.invoice_number,
            &inv.billing_period,
            inv.total,
            &lines,
        )
        .map_err(|e| format!("Tracker insert error: {e}"))?;

    println!("Invoice {} recorded in tracker (status: draft).", inv.invoice_number);
    println!("Total: ${:.2}", inv.total);

    Ok(())
}

fn cmd_status(db_path: &str, contract_arg: &str) -> Result<(), String> {
    let json = load_contract_json(contract_arg)?;
    let contract = parse_contract(&json)?;

    let tracker = Tracker::open(db_path).map_err(|e| format!("DB error: {e}"))?;

    let invoices = tracker
        .list_for_contract(&contract.contract_number)
        .map_err(|e| format!("DB error: {e}"))?;

    // Total contract value (sum of all CLINs × quantity × unit_price)
    let contract_value: f64 = contract
        .clins
        .iter()
        .map(|c| c.quantity * c.unit_price)
        .sum();

    let total_invoiced = tracker
        .total_invoiced(&contract.contract_number)
        .map_err(|e| format!("DB error: {e}"))?;

    let outstanding = (contract_value - total_invoiced).max(0.0);

    println!("Contract:       {}", contract.contract_number);
    println!("Period:         {} to {}", contract.period.start, contract.period.end);
    println!("Contract Value: ${:.2}", contract_value);
    println!("Invoiced:       ${:.2}  (submitted + paid)", total_invoiced);
    println!("Outstanding:    ${:.2}", outstanding);
    println!();

    if invoices.is_empty() {
        println!("No invoices on record.");
    } else {
        println!("{:<28}  {:>10}  {:>12}  {}", "Invoice", "Period", "Total", "Status");
        println!("{}", "-".repeat(68));
        for inv in &invoices {
            println!(
                "{:<28}  {:>10}  ${:>11.2}  {}",
                inv.invoice_number, inv.period, inv.total_amount, inv.status
            );
        }
    }

    Ok(())
}

fn cmd_list(db_path: &str, contract_filter: Option<&str>, emit_json: bool) -> Result<(), String> {
    let tracker = Tracker::open(db_path).map_err(|e| format!("DB error: {e}"))?;

    let invoices = match contract_filter {
        Some(c) => tracker
            .list_for_contract(c)
            .map_err(|e| format!("DB error: {e}"))?,
        None => tracker.list_all().map_err(|e| format!("DB error: {e}"))?,
    };

    if emit_json {
        println!("{}", serde_json::to_string_pretty(&invoices).unwrap());
        return Ok(());
    }

    if invoices.is_empty() {
        println!("No invoices found.");
        return Ok(());
    }

    println!(
        "{:<28}  {:<20}  {:>10}  {:>12}  {}",
        "Invoice", "Contract", "Period", "Total", "Status"
    );
    println!("{}", "-".repeat(88));
    for inv in &invoices {
        println!(
            "{:<28}  {:<20}  {:>10}  ${:>11.2}  {}",
            inv.invoice_number,
            inv.contract_number,
            inv.period,
            inv.total_amount,
            inv.status
        );
    }

    Ok(())
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();

    let result = match &cli.command {
        Commands::Generate { contract, period, out, json } => {
            cmd_generate(&cli.db, contract, period, out.as_deref(), *json)
        }
        Commands::Status { contract } => cmd_status(&cli.db, contract),
        Commands::List { contract, json } => cmd_list(&cli.db, contract.as_deref(), *json),
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
