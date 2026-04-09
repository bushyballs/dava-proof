mod filter;
mod output;
mod pivot;
mod reader;
mod stats;

use anyhow::Result;
use clap::{Parser, Subcommand};
use filter::{Filter, FilterOp, apply_filter};
use output::{print_csv, print_describe, print_info, print_json, print_pivot, print_stats, print_table};
use pivot::pivot as do_pivot;
use reader::{read_sheet, Sheet};
use stats::{compute_describe, compute_stats};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "sheetwise",
    version = env!("CARGO_PKG_VERSION"),
    about = "Spreadsheet / CSV intelligence CLI",
    long_about = "Spreadsheet and CSV analysis tool for exploring and manipulating tabular data.\n\n\
                  Subcommands:\n\
                  - info: Detect file type, row/column count, column names, data types\n\
                  - stats: Min/max/avg/sum for numeric columns, missing/distinct counts\n\
                  - filter: Filter rows by column value (eq, ne, gt, lt, gte, lte, contains, starts_with)\n\
                  - sort: Sort rows by a column, ascending or descending\n\
                  - convert: Convert file to another format (json, csv, table)\n\
                  - pivot: Group by one column and sum another (create pivot table)\n\
                  - merge: Merge two CSV files by appending rows (columns matched by name)\n\
                  - unique: Show unique values in a column\n\
                  - sample: Show a random sample of N rows\n\
                  - describe: Show statistical summary (count, mean, std, min, quartiles, max)\n\n\
                  Usage examples:\n\
                  sheetwise info data.csv\n\
                  sheetwise filter data.csv --column amount --gt 100 --format json\n\
                  sheetwise sort data.csv --by date --desc\n\
                  sheetwise pivot data.csv --group_by category --sum amount\n\
                  sheetwise merge sales.csv totals.csv --format table\n\
                  sheetwise describe data.csv"
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

    /// Generate a pivot table: group by one column and sum another.
    Pivot {
        file: PathBuf,

        /// Column to group by.
        #[arg(long)]
        group_by: String,

        /// Numeric column to sum.
        #[arg(long)]
        sum: String,
    },

    /// Merge two CSV files by appending rows (columns matched by name).
    Merge {
        file1: PathBuf,
        file2: PathBuf,

        /// Output format: table (default), json, csv.
        #[arg(long, default_value = "table")]
        format: String,
    },

    /// Show unique values in a column.
    Unique {
        file: PathBuf,

        /// Column name to inspect.
        #[arg(long)]
        column: String,
    },

    /// Show a random sample of N rows.
    Sample {
        file: PathBuf,

        /// Number of rows to sample.
        #[arg(long, default_value = "10")]
        n: usize,

        /// Output format: table (default), json, csv.
        #[arg(long, default_value = "table")]
        format: String,
    },

    /// Describe numeric columns: count, mean, std, min, 25%, 50%, 75%, max.
    Describe {
        file: PathBuf,
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

        Commands::Pivot { file, group_by, sum } => {
            let sheet = read_sheet(&file)?;
            let result = do_pivot(&sheet, &group_by, &sum)?;
            print_pivot(&result);
        }

        Commands::Merge { file1, file2, format } => {
            let sheet1 = read_sheet(&file1)?;
            let sheet2 = read_sheet(&file2)?;

            // Build unified column list (union of both, preserving sheet1 order)
            let mut col_names: Vec<String> = sheet1.columns.iter().map(|c| c.name.clone()).collect();
            for col in &sheet2.columns {
                if !col_names.iter().any(|n| n.eq_ignore_ascii_case(&col.name)) {
                    col_names.push(col.name.clone());
                }
            }

            // Map sheet column names to unified index
            let align = |sheet: &Sheet| -> Vec<Vec<String>> {
                let idx_map: Vec<Option<usize>> = col_names
                    .iter()
                    .map(|cname| {
                        sheet
                            .columns
                            .iter()
                            .position(|sc| sc.name.eq_ignore_ascii_case(cname))
                    })
                    .collect();
                sheet
                    .rows
                    .iter()
                    .map(|row| {
                        idx_map
                            .iter()
                            .map(|opt_i| {
                                opt_i.map(|i| row[i].clone()).unwrap_or_default()
                            })
                            .collect()
                    })
                    .collect()
            };

            let mut merged: Vec<Vec<String>> = align(&sheet1);
            merged.extend(align(&sheet2));

            println!("Merged {} + {} = {} rows", sheet1.row_count(), sheet2.row_count(), merged.len());
            render_output(&format, &col_names, &merged);
        }

        Commands::Unique { file, column } => {
            let sheet = read_sheet(&file)?;
            let col_idx = sheet
                .columns
                .iter()
                .position(|c| c.name.eq_ignore_ascii_case(&column))
                .ok_or_else(|| anyhow::anyhow!("Column not found: {column}"))?;

            let mut seen: HashSet<String> = HashSet::new();
            let mut unique_vals: Vec<String> = Vec::new();
            for row in &sheet.rows {
                let v = row[col_idx].clone();
                if seen.insert(v.clone()) {
                    unique_vals.push(v);
                }
            }
            unique_vals.sort();

            println!("Unique values in '{}' ({} total):", column, unique_vals.len());
            for v in &unique_vals {
                println!("  {v}");
            }
        }

        Commands::Sample { file, n, format } => {
            let sheet = read_sheet(&file)?;
            let total = sheet.rows.len();
            let col_names: Vec<String> = sheet.columns.iter().map(|c| c.name.clone()).collect();

            let sampled: Vec<Vec<String>> = if n >= total {
                sheet.rows.clone()
            } else {
                // Reservoir sampling (deterministic stride for reproducibility without rand dep)
                let step = total / n;
                (0..n).map(|i| sheet.rows[i * step].clone()).collect()
            };

            println!("Sample of {} / {} rows:", sampled.len(), total);
            render_output(&format, &col_names, &sampled);
        }

        Commands::Describe { file } => {
            let sheet = read_sheet(&file)?;
            let stats = compute_stats(&sheet);
            let describe_stats = compute_describe(&sheet, &stats);
            print_describe(&describe_stats);
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
