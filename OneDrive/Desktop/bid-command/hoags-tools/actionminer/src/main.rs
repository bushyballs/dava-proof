mod deadline;
mod extract;
mod models;
mod tracker;

use std::io::Read;

use clap::{Parser, Subcommand};
use hoags_core::bus::EventBus;

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

    /// Show open action items sorted by deadline (earliest first).
    Priorities,

    /// Assign or reassign an action item to a person.
    Assign {
        /// The numeric ID of the action item.
        id: i64,

        /// Person to assign the item to (e.g. "Collin" or "@alice").
        #[arg(long)]
        to: String,
    },

    /// Show open items whose deadline has passed.
    Overdue,

    /// Extract action items from a file and immediately save them (extract + insert).
    Import {
        /// Path to the meeting-notes file to import.
        #[arg(long)]
        file: String,
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
        Commands::Priorities => cmd_priorities(),
        Commands::Assign { id, to } => cmd_assign(id, to),
        Commands::Overdue => cmd_overdue(),
        Commands::Import { file } => cmd_import(file),
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

    // Publish bus event
    if let Ok(bus) = EventBus::open_default() {
        bus.publish("actionminer", "actionminer.action_extracted", &serde_json::json!({
            "count": saved.len(), "source": source
        }).to_string());
    }
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

fn cmd_priorities() {
    let conn = tracker::open().unwrap_or_else(|e| { eprintln!("DB error: {e}"); std::process::exit(1); });
    let items = tracker::list_by_priority(&conn)
        .unwrap_or_else(|e| { eprintln!("Query error: {e}"); std::process::exit(1); });

    if items.is_empty() {
        println!("No open action items with deadlines.");
        return;
    }

    println!("Action items by priority (earliest deadline first):");
    print_items(&items);
}

fn cmd_assign(id: i64, to: String) {
    let conn = tracker::open().unwrap_or_else(|e| { eprintln!("DB error: {e}"); std::process::exit(1); });
    match tracker::assign(&conn, id, &to) {
        Ok(true)  => println!("Action item #{id} assigned to '{to}'."),
        Ok(false) => { eprintln!("No action item with id {id}."); std::process::exit(1); }
        Err(e)    => { eprintln!("DB error: {e}"); std::process::exit(1); }
    }
}

fn cmd_overdue() {
    let conn = tracker::open().unwrap_or_else(|e| { eprintln!("DB error: {e}"); std::process::exit(1); });
    let items = tracker::list_overdue(&conn)
        .unwrap_or_else(|e| { eprintln!("Query error: {e}"); std::process::exit(1); });

    if items.is_empty() {
        println!("No overdue action items.");
        return;
    }

    println!("Overdue action items ({} total):", items.len());
    print_items(&items);
}

fn cmd_import(file: String) {
    let content = std::fs::read_to_string(&file)
        .unwrap_or_else(|e| { eprintln!("Error reading {file}: {e}"); std::process::exit(1); });

    let items = extract::extract(&content, &file);

    if items.is_empty() {
        println!("No action items found in '{file}'.");
        return;
    }

    let conn = tracker::open().unwrap_or_else(|e| { eprintln!("DB error: {e}"); std::process::exit(1); });
    let saved = tracker::insert_many(&conn, &items)
        .unwrap_or_else(|e| { eprintln!("Insert error: {e}"); std::process::exit(1); });

    println!("Imported and saved {} action item(s) from '{file}':", saved.len());
    print_items(&saved);
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
