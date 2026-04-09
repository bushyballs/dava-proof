mod expense;
mod report;
mod tracker;

use clap::{Parser, Subcommand};
use expense::Expense;
use hoags_core::bus::EventBus;

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
    long_about = "Hoags Inc expense and receipt tracker for federal contracting.\n\
                  Manages expenses by category, vendor, and contract number. Supports\n\
                  budget tracking, tax-year reporting, and CSV bulk import/export.\n\n\
                  Usage examples:\n\
                  receipts add --amount 45.99 --vendor 'Office Depot' --category supplies\n\
                  receipts import --csv expenses.csv\n\
                  receipts list --contract 12444626P0025\n\
                  receipts summary\n\
                  receipts report --tax_year 2026\n\
                  receipts budget --contract 12444626P0025 --limit 50000\n\
                  receipts export --format csv --contract 12444626P0025\n\n\
                  Categories: supplies, fuel, labor, equipment, travel, office, other\n\
                  Database location: $RECEIPTS_DB env var or ~/.hoags/receipts.db",
    version = env!("CARGO_PKG_VERSION")
)]
struct Cli {
    /// Path to the SQLite database file (overrides $RECEIPTS_DB env var)
    #[arg(long)]
    db: Option<String>,

    /// Output as JSON
    #[arg(long, global = true)]
    json: bool,

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

    /// Delete an expense by ID
    Delete {
        /// Expense ID to remove
        id: i64,
    },

    /// Bulk import expenses from a CSV file
    ///
    /// CSV must have headers: amount,vendor,category,date,contract_number,description
    /// (contract_number and description may be empty)
    Import {
        /// Path to the CSV file
        #[arg(long)]
        csv: String,
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

    /// Show totals by category, month, and contract (with monthly trend)
    Summary,

    /// Generate a tax-year expense report with deductibility classification
    Report {
        /// Tax year (e.g. 2026)
        #[arg(long)]
        tax_year: u32,
    },

    /// Set a budget limit for a contract; warns when approaching
    Budget {
        /// Contract number
        #[arg(long)]
        contract: String,

        /// Budget limit in USD (e.g. 50000)
        #[arg(long)]
        limit: f64,
    },

    /// Export expenses
    Export {
        /// Output format: json or csv (default: json)
        #[arg(long, default_value = "json")]
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
    let json_output = cli.json;

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
                    .as_deref()
                    .map(|c| format!(" (contract: {})", c))
                    .unwrap_or_default()
            );

            // Publish bus event
            if let Ok(bus) = EventBus::open_default() {
                bus.publish("receipts", "receipts.expense_added", &serde_json::json!({
                    "amount": amount, "vendor": vendor, "category": category, "contract": contract
                }).to_string());
            }

            // Budget check after add
            if let Some(ref cnum) = contract {
                check_budget_warning(&conn, cnum);
            }
        }

        Commands::Delete { id } => {
            let n = tracker::delete(&conn, id).unwrap_or_else(|e| {
                eprintln!("Error deleting expense: {}", e);
                std::process::exit(1);
            });
            if n == 0 {
                eprintln!("No expense found with ID {}.", id);
                std::process::exit(1);
            } else {
                println!("Deleted expense #{}.", id);
            }
        }

        Commands::Import { csv } => {
            import_csv(&conn, &csv);
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

            if json_output {
                let json_value = serde_json::json!({
                    "expenses": expenses,
                    "total": expenses.iter().map(|e| e.amount).sum::<f64>(),
                    "count": expenses.len()
                });
                println!("{}", serde_json::to_string_pretty(&json_value).unwrap());
            } else {
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
        }

        Commands::Summary => {
            report::print_summary(&conn);
        }

        Commands::Report { tax_year } => {
            report::print_tax_year_report(&conn, tax_year);
        }

        Commands::Budget { contract, limit } => {
            tracker::upsert_budget(&conn, &contract, limit).unwrap_or_else(|e| {
                eprintln!("Error setting budget: {}", e);
                std::process::exit(1);
            });
            println!(
                "Budget set for contract {}: ${:.2}",
                contract, limit
            );
            // Show current spend vs budget
            check_budget_warning(&conn, &contract);
        }

        Commands::Export {
            format,
            month,
            contract,
        } => {
            let expenses = match (&month, &contract) {
                (Some(m), None) => tracker::list_by_month(&conn, m),
                (None, Some(c)) => tracker::list_by_contract(&conn, c),
                _ => tracker::list_all(&conn),
            }
            .unwrap_or_else(|e| {
                eprintln!("Query error: {}", e);
                std::process::exit(1);
            });

            match format.as_str() {
                "json" => {
                    let json_value = serde_json::json!({
                        "expenses": expenses,
                        "total": expenses.iter().map(|e| e.amount).sum::<f64>(),
                        "count": expenses.len()
                    });
                    println!("{}", serde_json::to_string_pretty(&json_value).unwrap());
                }
                "csv" => {
                    report::export_csv(&expenses);
                }
                other => {
                    eprintln!("Unsupported format '{}'. Supported: json, csv", other);
                    std::process::exit(1);
                }
            }
        }
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Check if a contract is approaching or over its budget limit and print a warning.
fn check_budget_warning(conn: &rusqlite::Connection, contract: &str) {
    if let Ok(Some(limit)) = tracker::get_budget(conn, contract) {
        if let Ok(spent) = tracker::total_for_contract(conn, contract) {
            let pct = if limit > 0.0 { spent / limit * 100.0 } else { 0.0 };
            if spent >= limit {
                eprintln!(
                    "WARNING: Contract {} is OVER BUDGET — spent ${:.2} of ${:.2} limit ({:.1}%)",
                    contract, spent, limit, pct
                );
            } else if pct >= 80.0 {
                eprintln!(
                    "WARNING: Contract {} is at {:.1}% of budget — spent ${:.2} of ${:.2}",
                    contract, pct, spent, limit
                );
            } else {
                println!(
                    "Budget status for {}: ${:.2} spent / ${:.2} limit ({:.1}%)",
                    contract, spent, limit, pct
                );
            }
        }
    }
}

/// Bulk-import expenses from a CSV file.
///
/// Expected header: amount,vendor,category,date,contract_number,description
/// Categories: supplies, fuel, labor, equipment, travel, office, other
fn import_csv(conn: &rusqlite::Connection, csv_path: &str) {
    let mut rdr = csv::Reader::from_path(csv_path).unwrap_or_else(|e| {
        eprintln!("ERROR: Cannot open CSV file '{}': {}", csv_path, e);
        eprintln!("\nExpected CSV format:");
        eprintln!("  amount,vendor,category,date,contract_number,description");
        eprintln!("\nExample:");
        eprintln!("  45.99,Office Depot,supplies,2026-03-15,12444626P0025,Printer paper");
        eprintln!("\nCategories: supplies, fuel, labor, equipment, travel, office, other");
        eprintln!("\nDate format: YYYY-MM-DD (defaults to today if empty)");
        std::process::exit(1);
    });

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let mut imported = 0u32;
    let mut errors = 0u32;

    for (line_num, result) in rdr.records().enumerate() {
        let record = match result {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Line {}: CSV parse error: {}", line_num + 2, e);
                errors += 1;
                continue;
            }
        };

        // Columns: amount, vendor, category, date, contract_number, description
        let amount_str = record.get(0).unwrap_or("").trim();
        let vendor = record.get(1).unwrap_or("").trim();
        let category_raw = record.get(2).unwrap_or("other").trim();
        let date = {
            let d = record.get(3).unwrap_or("").trim();
            if d.is_empty() { today.as_str() } else { d }
        };
        let contract_number = {
            let c = record.get(4).unwrap_or("").trim();
            if c.is_empty() { None } else { Some(c) }
        };
        let description = {
            let d = record.get(5).unwrap_or("").trim();
            if d.is_empty() { None } else { Some(d) }
        };

        let amount: f64 = match amount_str.parse() {
            Ok(v) => v,
            Err(_) => {
                eprintln!("Line {}: invalid amount '{}' — expected a number (e.g. 45.99)", line_num + 2, amount_str);
                errors += 1;
                continue;
            }
        };

        if vendor.is_empty() {
            eprintln!("Line {}: vendor is empty — vendor name is required (column 2)", line_num + 2);
            errors += 1;
            continue;
        }

        let category = Expense::normalize_category(category_raw);

        match tracker::insert(conn, amount, vendor, &category, date, contract_number, description) {
            Ok(_) => imported += 1,
            Err(e) => {
                eprintln!("Line {}: DB error: {}", line_num + 2, e);
                errors += 1;
            }
        }
    }

    println!(
        "Import complete: {} imported, {} error{}.",
        imported,
        errors,
        if errors == 1 { "" } else { "s" }
    );
}
