/// sigstamp — PDF digital signature stamper for Hoags Inc.
///
/// Usage:
///   sigstamp sign <pdf> --signer "Collin Hoag" [--title "President"] [--initials] [--pages 1-3]
///   sigstamp sign <pdf> --signer "Collin Hoag" --page 1 --x 200 --y 700
///   sigstamp date <pdf>
///   sigstamp batch <dir> --signer "Collin Hoag" [--title "President"]
///   sigstamp verify <pdf>

mod detect;
mod sign;

use clap::{Parser, Subcommand};
use hoags_core::bus::EventBus;
use std::path::PathBuf;

use detect::detect_sig_locations;
use sign::{sign_batch, sign_pdf, stamp_date_only, verify_pdf_has_signature, StampParams};

#[derive(Parser)]
#[command(
    name = "sigstamp",
    version = env!("CARGO_PKG_VERSION"),
    about = "Stamp PDF signature fields with /s/ signatures — Hoags Inc."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Parse a page range string like "1-3" or "2" into an inclusive (start, end) tuple (1-based).
fn parse_page_range(s: &str) -> Result<(usize, usize), String> {
    let s = s.trim();
    if let Some((a, b)) = s.split_once('-') {
        let start: usize = a.trim().parse().map_err(|_| format!("Invalid page range start: '{}'", a))?;
        let end: usize = b.trim().parse().map_err(|_| format!("Invalid page range end: '{}'", b))?;
        if start == 0 || end == 0 {
            return Err("Page numbers are 1-based; 0 is not valid".to_string());
        }
        if start > end {
            return Err(format!("Page range start ({start}) must be <= end ({end})"));
        }
        Ok((start, end))
    } else {
        let n: usize = s.parse().map_err(|_| format!("Invalid page number: '{s}'"))?;
        if n == 0 {
            return Err("Page numbers are 1-based; 0 is not valid".to_string());
        }
        Ok((n, n))
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Place /s/ signature on a PDF.
    Sign {
        /// Path to the input PDF.
        pdf: PathBuf,

        /// Signer full name (e.g. "Collin Hoag").
        #[arg(long)]
        signer: String,

        /// Signer title (e.g. "President"). Placed below the signature line.
        #[arg(long)]
        title: Option<String>,

        /// 0-based page index. Defaults to the auto-detected signature page.
        #[arg(long, default_value_t = 0)]
        page: usize,

        /// Explicit X coordinate (PDF user-space, origin = lower-left).
        /// If omitted, auto-detection is used.
        #[arg(long)]
        x: Option<f64>,

        /// Explicit Y coordinate.
        #[arg(long)]
        y: Option<f64>,

        /// Output directory. Defaults to `<pdf_stem>_signed/` beside the input.
        #[arg(long)]
        output: Option<PathBuf>,

        /// Place initials (first letter of each name word) instead of full signature.
        #[arg(long)]
        initials: bool,

        /// Only sign specific pages (e.g. "1-3" or "2"). Pages are 1-based.
        #[arg(long)]
        pages: Option<String>,
    },

    /// Stamp today's date on the date field of a PDF.
    Date {
        /// Path to the input PDF.
        pdf: PathBuf,

        /// Output directory. Defaults to `<pdf_stem>_dated/` beside the input.
        #[arg(long)]
        output: Option<PathBuf>,
    },

    /// Sign all PDFs in a directory.
    Batch {
        /// Directory containing PDFs to sign.
        dir: PathBuf,

        /// Signer full name.
        #[arg(long)]
        signer: String,

        /// Signer title.
        #[arg(long)]
        title: Option<String>,

        /// Output directory. Defaults to `<dir>_signed/`.
        #[arg(long)]
        output: Option<PathBuf>,
    },

    /// Check whether a PDF already has a /s/ signature stamped in it.
    Verify {
        /// Path to the PDF to check.
        pdf: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Sign {
            pdf,
            signer,
            title,
            page,
            x,
            y,
            output,
            initials,
            pages,
        } => {
            if !pdf.exists() {
                eprintln!("Error: PDF not found: {}", pdf.display());
                std::process::exit(1);
            }

            // Parse --pages range if provided
            let page_range = match pages {
                Some(ref s) => match parse_page_range(s) {
                    Ok(r) => Some(r),
                    Err(e) => {
                        eprintln!("Error: invalid --pages value '{}': {e}", s);
                        std::process::exit(1);
                    }
                },
                None => None,
            };

            let output_dir = output.unwrap_or_else(|| {
                let stem = pdf
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("output");
                pdf.parent()
                    .unwrap_or_else(|| std::path::Path::new("."))
                    .join(format!("{}_signed", stem))
            });

            let params = StampParams {
                signer,
                title,
                page,
                x,
                y,
                initials,
                page_range,
            };

            // Auto-detect signature locations unless explicit coords supplied
            let locs = if params.x.is_none() || params.y.is_none() {
                let detected = detect_sig_locations(&pdf);
                if detected.is_empty() {
                    println!("No signature fields auto-detected — using default placement.");
                } else {
                    println!(
                        "Detected {} signature location(s).",
                        detected.len()
                    );
                }
                detected
            } else {
                Vec::new()
            };

            match sign_pdf(&pdf, &output_dir, &params, &locs) {
                Ok(out) => println!("Signed: {}", out.display()),
                Err(e) => {
                    eprintln!("Error signing PDF: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Date { pdf, output } => {
            if !pdf.exists() {
                eprintln!("Error: PDF not found: {}", pdf.display());
                std::process::exit(1);
            }

            let output_dir = output.unwrap_or_else(|| {
                let stem = pdf
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("output");
                pdf.parent()
                    .unwrap_or_else(|| std::path::Path::new("."))
                    .join(format!("{}_dated", stem))
            });

            match stamp_date_only(&pdf, &output_dir) {
                Ok(out) => println!("Dated: {}", out.display()),
                Err(e) => {
                    eprintln!("Error stamping date: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Batch {
            dir,
            signer,
            title,
            output,
        } => {
            if !dir.exists() || !dir.is_dir() {
                eprintln!("Error: directory not found: {}", dir.display());
                std::process::exit(1);
            }

            let output_dir = output.unwrap_or_else(|| {
                let stem = dir
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("batch");
                dir.parent()
                    .unwrap_or_else(|| std::path::Path::new("."))
                    .join(format!("{}_signed", stem))
            });

            let params = StampParams {
                signer,
                title,
                page: 0,
                x: None,
                y: None,
                initials: false,
                page_range: None,
            };

            let results = sign_batch(&dir, &output_dir, &params);

            let mut ok = 0usize;
            let mut fail = 0usize;
            for (src, res) in &results {
                match res {
                    Ok(out) => {
                        println!("  OK  {} -> {}", src.display(), out.display());
                        ok += 1;
                    }
                    Err(e) => {
                        eprintln!("  ERR {} : {}", src.display(), e);
                        fail += 1;
                    }
                }
            }
            println!("\nBatch complete: {} signed, {} failed.", ok, fail);
            if fail > 0 {
                std::process::exit(1);
            }
        }

        Commands::Verify { pdf } => {
            if !pdf.exists() {
                eprintln!("Error: PDF not found: {}", pdf.display());
                std::process::exit(1);
            }

            match verify_pdf_has_signature(&pdf) {
                Ok(true) => {
                    println!("SIGNED: /s/ signature found in '{}'.", pdf.display());
                }
                Ok(false) => {
                    println!("UNSIGNED: No /s/ signature found in '{}'.", pdf.display());
                    std::process::exit(2);
                }
                Err(e) => {
                    eprintln!("Error reading PDF: {e}");
                    std::process::exit(1);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_page_range_single() {
        assert_eq!(parse_page_range("3").unwrap(), (3, 3));
    }

    #[test]
    fn test_parse_page_range_range() {
        assert_eq!(parse_page_range("1-3").unwrap(), (1, 3));
        assert_eq!(parse_page_range("2-5").unwrap(), (2, 5));
    }

    #[test]
    fn test_parse_page_range_invalid() {
        assert!(parse_page_range("0").is_err());
        assert!(parse_page_range("3-1").is_err());
        assert!(parse_page_range("abc").is_err());
    }

    #[test]
    fn test_parse_page_range_whitespace() {
        assert_eq!(parse_page_range(" 2 - 4 ").unwrap(), (2, 4));
    }
}
