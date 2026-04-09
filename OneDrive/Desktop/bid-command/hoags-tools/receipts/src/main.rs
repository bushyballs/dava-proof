mod expense;
mod report;
mod tracker;

use clap::{Parser, Subcommand};
use expense::Expense;

/// Default database location: $RECEIPTS_DB or ~/.hoags/receipts.db
fn default_db_path() -> String {
    if let Ok(path) = std::env::var("RECEIPTS_DB") {
        return path;
    }
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    format!("{}/.hoags/receipts.db", home)
}

fn open_db(db: &str) -> rusqlite::Connection {
    // Ensure parent directory exists
    if let Some(parent) = std::path::Path::new(db).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).ok();
        }
    }
    tracker::open(db).unwrap_or_else(|e| {
        eprintln!("Failed to open database '{}': {}", db, e);
        std::process::exit(1);
    })
}

// ── CLI definition ────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "receipts",
    about = "Hoags Inc — expense & receipt tracker for federal contracting",
    version
)]
struct Cli {
    /// Path to the SQLite database file (overrides $RECEIPTS_DB env var)
    #[arg(long)]
    db: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new expense
    Add {
        /// Dollar amount (e.g. 45.99)
        #[arg(long)]
        amount: f64,

        /// Vendor / store name
        #[arg(long)]
        vendor: String,

        /// Category: supplies, fuel, labor, equipment, travel, office, other
        #[arg(long, default_value = "other")]
        category: String,

        /// Date in YYYY-MM-DD format (defaults to today)
        #[arg(long)]
        date: Option<String>,

        /// Federal contract number to link this expense to
        #[arg(long)]
        contract: Option<String>,

        /// Optional short description
        #[arg(long)]
        description: Option<String>,
    },

    /// List expenses (optionally filtered)
    List {
        /// Filter by month: YYYY-MM
        #[arg(long)]
        month: Option<String>,

        /// Filter by contract number
        #[arg(long)]
        contract: Option<String>,

        /// Filter by category
        #[arg(long)]
        category: Option<String>,
    },

    /// Show totals by category, month, and contract
    Summary,

    /// Export expenses
    Export {
        /// Output format: csv (default)
        #[arg(long, default_value = "csv")]
        format: String,

        /// Filter by month: YYYY-MM
        #[arg(long)]
        month: Option<String>,

        /// Filter by contract number
        #[arg(long)]
        contract: Option<String>,
    },
}

// ── entry point ───────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();
    let db_path = cli.db.unwrap_or_else(default_db_path);
    let conn = open_db(&db_path);

    match cli.command {
        Commands::Add {
            amount,
            vendor,
            category,
            date,
            contract,
            description,
        } => {
            let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
            let date = date.unwrap_or(today);
            let category = Expense::normalize_category(&category);

            let id = tracker::insert(
                &conn,
                amount,
                &vendor,
                &category,
                &date,
                contract.as_deref(),
                description.as_deref(),
            )
            .unwrap_or_else(|e| {
                eprintln!("Error saving expense: {}", e);
                std::process::exit(1);
            });

            println!(
                "Added expense #{}: ${:.2} at {} [{}] on {}{}",
                id,
                amount,
                vendor,
                category,
                date,
                contract
                    .map(|c| format!(" (contract: {})", c))
                    .unwrap_or_default()
            );
        }

        Commands::List {
            month,
            contract,
            category,
        } => {
            let expenses = match (&month, &contract, &category) {
                (Some(m), None, None) => tracker::list_by_month(&conn, m),
                (None, Some(c), None) => tracker::list_by_contract(&conn, c),
                (None, None, Some(cat)) => tracker::list_by_category(&conn, cat),
                (None, None, None) => tracker::list_all(&conn),
                _ => {
                    eprintln!(
                        "Specify at most one filter: --month, --contract, or --category"
                    );
                    std::process::exit(1);
                }
            }
            .unwrap_or_else(|e| {
                eprintln!("Query error: {}", e);
                std::process::exit(1);
            });

            if expenses.is_empty() {
                println!("No expenses found.");
            } else {
                println!("{}", Expense::header());
                println!("{}", "-".repeat(80));
                for e in &expenses {
                    println!("{}", e.to_row());
                }
                println!("{}", "-".repeat(80));
                let total: f64 = expenses.iter().map(|e| e.amount).sum();
                println!(
                    "Total: ${:.2}  ({} expense{})",
                    total,
                    expenses.len(),
                    if expenses.len() == 1 { "" } else { "s" }
                );
            }
        }

        Commands::Summary => {
            report::print_summary(&conn);
        }

        Commands::Export {
            format,
            month,
            contract,
        } => {
            if format != "csv" {
                eprintln!("Unsupported format '{}'. Currently supported: csv", format);
                std::process::exit(1);
            }
            let expenses = match (&month, &contract) {
                (Some(m), None) => tracker::list_by_month(&conn, m),
                (None, Some(c)) => tracker::list_by_contract(&conn, c),
                _ => tracker::list_all(&conn),
            }
            .unwrap_or_else(|e| {
                eprintln!("Query error: {}", e);
                std::process::exit(1);
            });
            report::export_csv(&expenses);
        }
    }
}
