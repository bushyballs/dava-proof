mod extract;
mod models;
mod tracker;

use std::io::Read;

use clap::{Parser, Subcommand};

// ---------------------------------------------------------------------------
// CLI definition
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(
    name = "actionminer",
    version = "0.1.0",
    about = "Extract and track action items from meeting notes",
    long_about = None,
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Extract action items from a text/markdown file (or stdin).
    Extract {
        /// Path to the meeting-notes file.
        file: Option<String>,

        /// Read from stdin instead of a file.
        #[arg(long)]
        stdin: bool,

        /// Print extracted items but do NOT save them to the database.
        #[arg(long)]
        dry_run: bool,
    },

    /// List tracked action items.
    List {
        /// Filter by status: open | done
        #[arg(long)]
        status: Option<String>,

        /// Filter by assignee (e.g. "@alice")
        #[arg(long)]
        assignee: Option<String>,
    },

    /// Mark an action item as done.
    Complete {
        /// The numeric ID shown by `actionminer list`.
        id: i64,
    },

    /// Export all action items.
    Export {
        /// Output format: json (default)
        #[arg(long, default_value = "json")]
        format: String,
    },
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Extract { file, stdin, dry_run } => cmd_extract(file, stdin, dry_run),
        Commands::List { status, assignee } => cmd_list(status, assignee),
        Commands::Complete { id } => cmd_complete(id),
        Commands::Export { format } => cmd_export(format),
    }
}

// ---------------------------------------------------------------------------
// Command implementations
// ---------------------------------------------------------------------------

fn cmd_extract(file: Option<String>, use_stdin: bool, dry_run: bool) {
    let (text, source) = if use_stdin || file.is_none() {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .expect("Failed to read stdin");
        (buf, "<stdin>".to_string())
    } else {
        let path = file.unwrap();
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| { eprintln!("Error reading {path}: {e}"); std::process::exit(1); });
        (content, path)
    };

    let items = extract::extract(&text, &source);

    if items.is_empty() {
        println!("No action items found.");
        return;
    }

    if dry_run {
        println!("Found {} action item(s) (dry run — not saved):", items.len());
        print_items(&items);
        return;
    }

    let conn = tracker::open().unwrap_or_else(|e| { eprintln!("DB error: {e}"); std::process::exit(1); });
    let saved = tracker::insert_many(&conn, &items)
        .unwrap_or_else(|e| { eprintln!("Insert error: {e}"); std::process::exit(1); });

    println!("Extracted and saved {} action item(s):", saved.len());
    print_items(&saved);
}

fn cmd_list(status: Option<String>, assignee: Option<String>) {
    let conn = tracker::open().unwrap_or_else(|e| { eprintln!("DB error: {e}"); std::process::exit(1); });

    let items = if let Some(ref who) = assignee {
        tracker::list_by_assignee(&conn, who)
            .unwrap_or_else(|e| { eprintln!("Query error: {e}"); std::process::exit(1); })
    } else {
        tracker::list(&conn, status.as_deref())
            .unwrap_or_else(|e| { eprintln!("Query error: {e}"); std::process::exit(1); })
    };

    if items.is_empty() {
        println!("No action items found.");
        return;
    }

    print_items(&items);
}

fn cmd_complete(id: i64) {
    let conn = tracker::open().unwrap_or_else(|e| { eprintln!("DB error: {e}"); std::process::exit(1); });
    match tracker::complete(&conn, id) {
        Ok(true)  => println!("Action item #{id} marked as done."),
        Ok(false) => { eprintln!("No action item with id {id}."); std::process::exit(1); }
        Err(e)    => { eprintln!("DB error: {e}"); std::process::exit(1); }
    }
}

fn cmd_export(format: String) {
    let conn = tracker::open().unwrap_or_else(|e| { eprintln!("DB error: {e}"); std::process::exit(1); });

    match format.as_str() {
        "json" => {
            let json = tracker::export_json(&conn)
                .unwrap_or_else(|e| { eprintln!("Export error: {e}"); std::process::exit(1); });
            println!("{json}");
        }
        other => {
            eprintln!("Unsupported format '{other}'. Supported: json");
            std::process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// Display helper
// ---------------------------------------------------------------------------

fn print_items(items: &[models::ActionItem]) {
    for item in items {
        let assignee = item.assignee.as_deref().unwrap_or("-");
        let deadline = item.deadline.as_deref().unwrap_or("-");
        let status_tag = if item.status == "done" { "[DONE]" } else { "[OPEN]" };
        println!(
            "#{:<4} {status_tag}  {}\n       assignee={assignee}  deadline={deadline}  source={}",
            item.id, item.description, item.source_file
        );
    }
}
