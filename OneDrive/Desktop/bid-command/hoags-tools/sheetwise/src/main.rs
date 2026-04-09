mod filter;
mod output;
mod reader;
mod stats;

use anyhow::Result;
use clap::{Parser, Subcommand};
use filter::{Filter, FilterOp, apply_filter};
use output::{print_csv, print_info, print_json, print_stats, print_table};
use reader::read_sheet;
use stats::compute_stats;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "sheetwise",
    version = "0.1.0",
    about = "Spreadsheet / CSV intelligence CLI"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Detect file type, show row/col count, column names, and data types.
    Info {
        file: PathBuf,
    },
    /// Show min/max/avg/sum for numeric columns plus missing/distinct counts.
    Stats {
        file: PathBuf,
    },
    /// Filter rows by a column value.
    Filter {
        file: PathBuf,

        /// Column name to filter on.
        #[arg(long)]
        column: String,

        /// Filter operator: eq, ne, gt, lt, gte, lte, contains, starts_with.
        #[arg(long, default_value = "gt")]
        op: String,

        /// Shorthand: equivalent to --op gt --value <VALUE>
        #[arg(long)]
        gt: Option<String>,

        /// Shorthand: equivalent to --op lt --value <VALUE>
        #[arg(long)]
        lt: Option<String>,

        /// Shorthand: equivalent to --op eq --value <VALUE>
        #[arg(long)]
        eq: Option<String>,

        /// Shorthand: equivalent to --op contains --value <VALUE>
        #[arg(long)]
        contains: Option<String>,

        /// Value to compare against (used with --op).
        #[arg(long)]
        value: Option<String>,

        /// Output format: table (default), json, csv.
        #[arg(long, default_value = "table")]
        format: String,
    },
    /// Sort rows by a column.
    Sort {
        file: PathBuf,

        /// Column name to sort by.
        #[arg(long)]
        by: String,

        /// Sort descending.
        #[arg(long)]
        desc: bool,

        /// Output format: table (default), json, csv.
        #[arg(long, default_value = "table")]
        format: String,
    },
    /// Convert the file to another format.
    Convert {
        file: PathBuf,

        /// Target format: json, csv, table.
        #[arg(long)]
        to: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Info { file } => {
            let sheet = read_sheet(&file)?;
            print_info(&sheet);
        }

        Commands::Stats { file } => {
            let sheet = read_sheet(&file)?;
            let stats = compute_stats(&sheet);
            print_stats(&stats);
        }

        Commands::Filter {
            file,
            column,
            op,
            gt,
            lt,
            eq,
            contains,
            value,
            format,
        } => {
            let sheet = read_sheet(&file)?;

            // Resolve operator + value from shorthand flags or explicit --op/--value
            let (resolved_op, resolved_value) = if let Some(v) = gt {
                (FilterOp::Gt, v)
            } else if let Some(v) = lt {
                (FilterOp::Lt, v)
            } else if let Some(v) = eq {
                (FilterOp::Eq, v)
            } else if let Some(v) = contains {
                (FilterOp::Contains, v)
            } else {
                let op_enum = FilterOp::from_str(&op)
                    .ok_or_else(|| anyhow::anyhow!("Unknown operator: {op}"))?;
                let val = value
                    .ok_or_else(|| anyhow::anyhow!("--value is required when using --op"))?;
                (op_enum, val)
            };

            let filter = Filter {
                column,
                op: resolved_op,
                value: resolved_value,
            };

            let rows: Vec<Vec<String>> = apply_filter(&sheet, &filter)
                .into_iter()
                .cloned()
                .collect();

            let col_names: Vec<String> = sheet.columns.iter().map(|c| c.name.clone()).collect();
            render_output(&format, &col_names, &rows);
        }

        Commands::Sort {
            file,
            by,
            desc,
            format,
        } => {
            let sheet = read_sheet(&file)?;
            let col_idx = sheet
                .columns
                .iter()
                .position(|c| c.name.eq_ignore_ascii_case(&by))
                .ok_or_else(|| anyhow::anyhow!("Column not found: {by}"))?;

            let mut rows = sheet.rows.clone();
            rows.sort_by(|a, b| {
                let av = &a[col_idx];
                let bv = &b[col_idx];
                // Try numeric sort first
                let cmp = match (parse_num(av), parse_num(bv)) {
                    (Some(an), Some(bn)) => an.partial_cmp(&bn).unwrap_or(std::cmp::Ordering::Equal),
                    _ => av.cmp(bv),
                };
                if desc { cmp.reverse() } else { cmp }
            });

            let col_names: Vec<String> = sheet.columns.iter().map(|c| c.name.clone()).collect();
            render_output(&format, &col_names, &rows);
        }

        Commands::Convert { file, to } => {
            let sheet = read_sheet(&file)?;
            let col_names: Vec<String> = sheet.columns.iter().map(|c| c.name.clone()).collect();
            render_output(&to, &col_names, &sheet.rows);
        }
    }

    Ok(())
}

fn render_output(format: &str, col_names: &[String], rows: &[Vec<String>]) {
    match format.to_ascii_lowercase().as_str() {
        "json" => print_json(col_names, rows),
        "csv" => print_csv(col_names, rows),
        _ => print_table(col_names, rows),
    }
}

fn parse_num(s: &str) -> Option<f64> {
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
        .collect();
    cleaned.parse::<f64>().ok()
}
