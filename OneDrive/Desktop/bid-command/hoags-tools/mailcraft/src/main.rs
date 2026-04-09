//! mailcraft — government contractor email composer for Hoags Inc.
//!
//! Drafts professional federal contracting emails and saves them as plain-text
//! files.  This tool NEVER sends email; it only outputs drafts to disk and
//! stdout per Hoags standing rules.

mod draft;
mod templates;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// mailcraft: Government contractor email composer.
///
/// Drafts professional emails for federal contracting scenarios.
/// All output is plain-text only — this tool NEVER sends email.
#[derive(Parser, Debug)]
#[command(
    name = "mailcraft",
    version = "0.1.0",
    author = "Hoags Inc.",
    about = "Government contractor email drafter — output only, never sends"
)]
struct Cli {
    /// Directory where draft files are saved (default: current directory)
    #[arg(short, long, default_value = ".")]
    output_dir: PathBuf,

    /// Print the draft to stdout in addition to saving the file
    #[arg(short, long, default_value_t = true)]
    print: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Draft a quote submission email
    QuoteSubmit {
        /// Solicitation number (e.g. W9127S26QA030)
        #[arg(long)]
        sol: String,

        /// Contracting officer full name
        #[arg(long)]
        co: String,

        /// Contracting officer email address
        #[arg(long)]
        co_email: String,

        /// Attachment file names to list in the email (repeatable)
        #[arg(long = "attach", value_name = "FILE")]
        attachments: Vec<String>,
    },

    /// Draft an amendment acknowledgment email
    AmendmentAck {
        /// Solicitation number
        #[arg(long)]
        sol: String,

        /// Amendment number
        #[arg(long)]
        amendment: u32,

        /// Contracting officer email address (optional)
        #[arg(long, default_value = "")]
        co_email: String,
    },

    /// Draft a post-award debrief request (FAR 15.506)
    DebriefRequest {
        /// Solicitation number
        #[arg(long)]
        sol: String,

        /// Contracting officer full name
        #[arg(long)]
        co: String,

        /// Contracting officer email address
        #[arg(long, default_value = "")]
        co_email: String,
    },

    /// Draft a response to an award notification
    AwardResponse {
        /// Contract number (e.g. 12444626P0028)
        #[arg(long)]
        contract: String,

        /// Contracting officer full name (optional)
        #[arg(long, default_value = "")]
        co: String,

        /// Contracting officer email address (optional)
        #[arg(long, default_value = "")]
        co_email: String,
    },
}

fn main() {
    let cli = Cli::parse();

    // Ensure output directory exists.
    if let Err(e) = std::fs::create_dir_all(&cli.output_dir) {
        eprintln!("error: cannot create output directory '{}': {}", cli.output_dir.display(), e);
        std::process::exit(1);
    }

    let (draft, identifier, email_type) = match &cli.command {
        Commands::QuoteSubmit { sol, co, co_email, attachments } => {
            let refs: Vec<&str> = attachments.iter().map(String::as_str).collect();
            let d = templates::quote_submit(sol, co, co_email, &refs);
            (d, sol.clone(), "quote-submit".to_string())
        }

        Commands::AmendmentAck { sol, amendment, co_email } => {
            let d = templates::amendment_ack(sol, *amendment, co_email);
            (d, sol.clone(), format!("amendment-ack-{}", amendment))
        }

        Commands::DebriefRequest { sol, co, co_email } => {
            let d = templates::debrief_request(sol, co, co_email);
            (d, sol.clone(), "debrief-request".to_string())
        }

        Commands::AwardResponse { contract, co, co_email } => {
            let d = templates::award_response(contract, co, co_email);
            (d, contract.clone(), "award-response".to_string())
        }
    };

    // Always save to file.
    match draft::save_draft(&draft, &identifier, &email_type, &cli.output_dir) {
        Ok(path) => {
            if cli.print {
                print!("{}", draft::compose(&draft));
                println!();
            }
            println!(
                "Draft saved: {}",
                path.display()
            );
            println!("NOTE: This draft has NOT been sent. Review before emailing.");
        }
        Err(e) => {
            eprintln!("error: failed to save draft: {}", e);
            std::process::exit(1);
        }
    }
}
