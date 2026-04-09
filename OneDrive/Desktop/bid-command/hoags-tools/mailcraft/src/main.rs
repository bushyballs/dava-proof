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

    /// Override company name (default: HOAGS_COMPANY env or "Hoags Inc.")
    #[arg(long)]
    company: Option<String>,

    /// Override signer name (default: HOAGS_SIGNER env or "Collin Hoag")
    #[arg(long)]
    signer: Option<String>,

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

        /// Contracting officer email address (.gov or .mil)
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

        /// Contracting officer email address (.gov or .mil, optional)
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

        /// Contracting officer email address (.gov or .mil, optional)
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

        /// Contracting officer email address (.gov or .mil, optional)
        #[arg(long, default_value = "")]
        co_email: String,
    },

    /// Draft a monthly contract status-update email
    StatusUpdate {
        /// Contract number (e.g. 12444626P0028)
        #[arg(long)]
        contract: String,

        /// Current status text (e.g. "On track", "Minor delay — see note")
        #[arg(long)]
        status: String,

        /// Contracting officer email address (.gov or .mil, optional)
        #[arg(long, default_value = "")]
        co_email: String,
    },

    /// Draft a question to the contracting officer
    Question {
        /// Solicitation number
        #[arg(long)]
        sol: String,

        /// The question to ask
        #[arg(long)]
        question: String,

        /// Contracting officer email address (.gov or .mil, optional)
        #[arg(long, default_value = "")]
        co_email: String,
    },

    /// Draft an invoice submission email
    InvoiceSubmit {
        /// Invoice identifier (e.g. HOAGS-INV-001)
        #[arg(long)]
        invoice_number: String,

        /// Invoice total amount in USD (e.g. 10645.63)
        #[arg(long)]
        amount: f64,

        /// Contract number to reference (optional)
        #[arg(long)]
        contract: Option<String>,

        /// Finance office / CO email address (.gov or .mil, optional)
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

    let company = cli.company.as_deref();
    let signer = cli.signer.as_deref();

    let (draft, identifier, email_type) = match &cli.command {
        Commands::QuoteSubmit { sol, co, co_email, attachments } => {
            validate_email_or_exit(co_email);
            let refs: Vec<&str> = attachments.iter().map(String::as_str).collect();
            let d = templates::quote_submit(sol, co, co_email, &refs, company, signer);
            (d, sol.clone(), "quote-submit".to_string())
        }

        Commands::AmendmentAck { sol, amendment, co_email } => {
            validate_email_or_exit(co_email);
            let d = templates::amendment_ack(sol, *amendment, co_email, company, signer);
            (d, sol.clone(), format!("amendment-ack-{}", amendment))
        }

        Commands::DebriefRequest { sol, co, co_email } => {
            validate_email_or_exit(co_email);
            let d = templates::debrief_request(sol, co, co_email, company, signer);
            (d, sol.clone(), "debrief-request".to_string())
        }

        Commands::AwardResponse { contract, co, co_email } => {
            validate_email_or_exit(co_email);
            let d = templates::award_response(contract, co, co_email, company, signer);
            (d, contract.clone(), "award-response".to_string())
        }

        Commands::StatusUpdate { contract, status, co_email } => {
            validate_email_or_exit(co_email);
            let d = templates::status_update(contract, status, co_email, company, signer);
            (d, contract.clone(), "status-update".to_string())
        }

        Commands::Question { sol, question, co_email } => {
            validate_email_or_exit(co_email);
            let d = templates::question(sol, question, co_email, company, signer);
            (d, sol.clone(), "question".to_string())
        }

        Commands::InvoiceSubmit { invoice_number, amount, contract, co_email } => {
            validate_email_or_exit(co_email);
            let d = templates::invoice_submit(
                invoice_number,
                *amount,
                contract.as_deref(),
                co_email,
                company,
                signer,
            );
            let id = contract
                .as_deref()
                .map(|c| format!("{}_{}", c, invoice_number))
                .unwrap_or_else(|| invoice_number.clone());
            (d, id, "invoice-submit".to_string())
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

/// Validate a CO email and exit with an error if it fails.
/// Empty strings pass (optional email fields).
fn validate_email_or_exit(email: &str) {
    if let Err(msg) = templates::validate_gov_email(email) {
        eprintln!("error: {}", msg);
        std::process::exit(1);
    }
}
