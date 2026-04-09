/// docconv — universal document format converter.
///
/// Commands:
///   convert <input> --to <format>   Convert a file to the target format
///   info <file>                     Show file type, page count, text preview
///   extract-text <pdf>              Extract all text from a PDF
///   extract-tables <pdf>            Extract table-like rows from a PDF as CSV

mod convert;
mod extract;
mod formats;

use anyhow::Context;
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

use crate::formats::{detect_format, Format};

// ─── CLI definition ──────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "docconv",
    version = "0.1.0",
    about = "Universal document format converter — PDF / CSV / JSON / Text"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert a document to the target format.
    Convert {
        /// Input file path.
        input: PathBuf,
        /// Target format: pdf | txt | json | csv
        #[arg(long)]
        to: String,
        /// Output file path (default: <input-stem>.<target-ext>)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Show file type, page count (for PDFs), and a short text preview.
    Info {
        /// File to inspect.
        file: PathBuf,
    },
    /// Extract all text from a PDF and print to stdout.
    ExtractText {
        /// PDF file path.
        pdf: PathBuf,
        /// Output file (default: print to stdout).
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Extract table-like rows from a PDF and emit CSV to stdout.
    ExtractTables {
        /// PDF file path.
        pdf: PathBuf,
        /// Output file (default: print to stdout).
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

// ─── Entry point ─────────────────────────────────────────────────────────────

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Convert { input, to, output } => cmd_convert(&input, &to, output.as_deref()),
        Commands::Info { file } => cmd_info(&file),
        Commands::ExtractText { pdf, output } => cmd_extract_text(&pdf, output.as_deref()),
        Commands::ExtractTables { pdf, output } => cmd_extract_tables(&pdf, output.as_deref()),
    }
}

// ─── Command handlers ────────────────────────────────────────────────────────

fn cmd_convert(input: &Path, to: &str, output: Option<&Path>) -> anyhow::Result<()> {
    let target = Format::from_str(to);

    let result = convert::convert(input, &target)
        .with_context(|| format!("Converting {}", input.display()))?;

    let out_path = output
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| {
            let stem = input.file_stem().unwrap_or_default();
            let mut p = input.parent().unwrap_or(Path::new(".")).to_path_buf();
            p.push(format!(
                "{}.{}",
                stem.to_string_lossy(),
                result.format.extension()
            ));
            p
        });

    std::fs::write(&out_path, result.output.as_bytes())
        .with_context(|| format!("Writing output to {}", out_path.display()))?;

    println!("Converted → {}", out_path.display());
    Ok(())
}

fn cmd_info(file: &Path) -> anyhow::Result<()> {
    if !file.exists() {
        anyhow::bail!("File not found: {}", file.display());
    }

    let format = detect_format(file);
    println!("File   : {}", file.display());
    println!("Format : {format}");

    if format == Format::Pdf {
        let pages = extract::extract_text_pages(file)
            .with_context(|| format!("Reading PDF {}", file.display()))?;
        println!("Pages  : {}", pages.len());
        if let Some((_, first_text)) = pages.first() {
            let preview: String = first_text.chars().take(300).collect();
            let preview = preview.trim();
            if !preview.is_empty() {
                println!("Preview:");
                println!("{preview}");
            }
        }
    } else {
        let meta = std::fs::metadata(file)?;
        println!("Size   : {} bytes", meta.len());
        // Show first 300 chars for text-like files.
        if matches!(format, Format::Text | Format::Csv | Format::Json) {
            let raw = std::fs::read_to_string(file).unwrap_or_default();
            let preview: String = raw.chars().take(300).collect();
            let preview = preview.trim();
            if !preview.is_empty() {
                println!("Preview:");
                println!("{preview}");
            }
        }
    }

    Ok(())
}

fn cmd_extract_text(pdf: &Path, output: Option<&Path>) -> anyhow::Result<()> {
    let text = extract::extract_all_text(pdf)
        .with_context(|| format!("Extracting text from {}", pdf.display()))?;

    emit(&text, output)
}

fn cmd_extract_tables(pdf: &Path, output: Option<&Path>) -> anyhow::Result<()> {
    let rows = extract::extract_tables(pdf)
        .with_context(|| format!("Extracting tables from {}", pdf.display()))?;

    let mut csv = String::new();
    for row in rows {
        csv.push_str(&row.join(","));
        csv.push('\n');
    }

    emit(&csv, output)
}

/// Print `content` to stdout or write to `output` if provided.
fn emit(content: &str, output: Option<&Path>) -> anyhow::Result<()> {
    if let Some(path) = output {
        std::fs::write(path, content.as_bytes())
            .with_context(|| format!("Writing to {}", path.display()))?;
        println!("Wrote → {}", path.display());
    } else {
        print!("{content}");
    }
    Ok(())
}
