use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod context;
mod proposal;
mod templates;

use context::{load_context, load_context_with_sol};
use proposal::{
    build_cover_letter, build_past_performance, build_price_schedule,
    build_technical_approach, extract_sol_meta, generate_full_package,
};

// ─── CLI definition ───────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "propbuilder",
    version = "0.1.0",
    about = "Proposal package generator for Hoags Inc. federal bids",
    long_about = "Generates complete proposal packages from a context JSON and optional\n\
                  solicitation PDF. Outputs cover letter, technical approach, past\n\
                  performance, and price schedule as individual PDF files."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a complete proposal package (all 4 documents).
    Generate {
        /// Path to the solicitation PDF (optional; used to extract sol number, CO, due date).
        #[arg(long, value_name = "PDF")]
        solicitation: Option<PathBuf>,

        /// Path to the company context JSON file.
        #[arg(long, value_name = "JSON")]
        context: PathBuf,

        /// Output directory for the proposal files.
        #[arg(long, value_name = "DIR", default_value = "proposal_output")]
        output: PathBuf,
    },

    /// Generate only the cover letter.
    CoverLetter {
        /// Path to the company context JSON file.
        #[arg(long, value_name = "JSON")]
        context: PathBuf,

        /// Contracting Officer name (overrides value in context JSON).
        #[arg(long, value_name = "NAME")]
        co_name: Option<String>,

        /// Output file path.
        #[arg(long, value_name = "FILE", default_value = "cover_letter.pdf")]
        output: PathBuf,
    },

    /// Generate only the past performance volume.
    PastPerformance {
        /// Path to the company context JSON file.
        #[arg(long, value_name = "JSON")]
        context: PathBuf,

        /// Output file path.
        #[arg(long, value_name = "FILE", default_value = "past_performance.pdf")]
        output: PathBuf,
    },

    /// Generate only the price schedule.
    PriceSchedule {
        /// Path to the company context JSON file.
        #[arg(long, value_name = "JSON")]
        context: PathBuf,

        /// Output file path.
        #[arg(long, value_name = "FILE", default_value = "price_schedule.pdf")]
        output: PathBuf,
    },

    /// Generate only the technical approach volume.
    TechnicalApproach {
        /// Path to the company context JSON file.
        #[arg(long, value_name = "JSON")]
        context: PathBuf,

        /// Output file path.
        #[arg(long, value_name = "FILE", default_value = "technical_approach.pdf")]
        output: PathBuf,
    },
}

// ─── Entry point ─────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Generate { solicitation, context, output } => {
            println!("[propbuilder] Generating full proposal package...");
            println!("  context : {}", context.display());
            println!("  output  : {}", output.display());

            // Extract solicitation metadata from PDF if provided
            let sol_meta = solicitation.as_ref().map(|p| {
                println!("  solicitation PDF: {}", p.display());
                let meta = extract_sol_meta(p);
                if !meta.number.is_empty() {
                    println!("  -> Sol number : {}", meta.number);
                }
                if !meta.co_name.is_empty() {
                    println!("  -> CO name    : {}", meta.co_name);
                }
                meta
            });

            let ctx = load_context_with_sol(&context, sol_meta).unwrap_or_else(|e| {
                eprintln!("ERROR: Failed to load context: {}", e);
                std::process::exit(1);
            });

            match generate_full_package(&ctx, &output) {
                Ok(files) => {
                    println!("[propbuilder] Package complete — {} files written:", files.len());
                    for f in &files {
                        println!("  -> {}", f.display());
                    }
                }
                Err(e) => {
                    eprintln!("ERROR: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::CoverLetter { context, co_name, output } => {
            println!("[propbuilder] Building cover letter...");
            let ctx = load_context(&context).unwrap_or_else(|e| {
                eprintln!("ERROR: {}", e);
                std::process::exit(1);
            });
            let co = co_name.as_deref();
            let mut doc = build_cover_letter(&ctx, co);
            doc.save(&output).unwrap_or_else(|e| {
                eprintln!("ERROR saving PDF: {}", e);
                std::process::exit(1);
            });
            println!("  -> {}", output.display());
        }

        Commands::PastPerformance { context, output } => {
            println!("[propbuilder] Building past performance volume...");
            let ctx = load_context(&context).unwrap_or_else(|e| {
                eprintln!("ERROR: {}", e);
                std::process::exit(1);
            });
            let mut doc = build_past_performance(&ctx);
            doc.save(&output).unwrap_or_else(|e| {
                eprintln!("ERROR saving PDF: {}", e);
                std::process::exit(1);
            });
            println!("  -> {}", output.display());
        }

        Commands::PriceSchedule { context, output } => {
            println!("[propbuilder] Building price schedule...");
            let ctx = load_context(&context).unwrap_or_else(|e| {
                eprintln!("ERROR: {}", e);
                std::process::exit(1);
            });
            let mut doc = build_price_schedule(&ctx);
            doc.save(&output).unwrap_or_else(|e| {
                eprintln!("ERROR saving PDF: {}", e);
                std::process::exit(1);
            });
            println!("  -> {}", output.display());
        }

        Commands::TechnicalApproach { context, output } => {
            println!("[propbuilder] Building technical approach volume...");
            let ctx = load_context(&context).unwrap_or_else(|e| {
                eprintln!("ERROR: {}", e);
                std::process::exit(1);
            });
            let mut doc = build_technical_approach(&ctx);
            doc.save(&output).unwrap_or_else(|e| {
                eprintln!("ERROR saving PDF: {}", e);
                std::process::exit(1);
            });
            println!("  -> {}", output.display());
        }
    }
}
