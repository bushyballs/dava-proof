//! davalive — DAVA's heartbeat daemon and system orchestrator.
//!
//! # Commands
//!
//! | Command  | Description                                                 |
//! |----------|-------------------------------------------------------------|
//! | `start`  | Start the daemon (runs bus connectors every 10s forever)    |
//! | `status` | Show system status: tools, bus stats, memory stats          |
//! | `pulse`  | Single heartbeat: run all connectors once, report results   |
//! | `health` | Check health of all tools (do binaries exist?)              |
//! | `stats`  | Show DAVA's cumulative stats across all tools               |

mod daemon;
mod health;
mod stats;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "davalive",
    about = "DAVA's heartbeat daemon — orchestrates all Hoags tools",
    version = "0.1.0"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the daemon (runs bus connectors in a loop forever)
    Start,
    /// Show system status — tools, bus stats, memory stats
    Status,
    /// Single heartbeat: run all connectors once and report results
    Pulse,
    /// Check health of all tools (do binaries exist? do they respond?)
    Health,
    /// Show DAVA's cumulative stats across all tools
    Stats,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start => cmd_start(),
        Commands::Status => cmd_status(),
        Commands::Pulse => cmd_pulse(),
        Commands::Health => cmd_health(),
        Commands::Stats => cmd_stats(),
    }
}

// ── Command implementations ───────────────────────────────────────────────────

fn cmd_start() {
    daemon::start_daemon();
}

fn cmd_pulse() {
    println!("DAVA LIVE — Running single heartbeat pass...");
    let summaries = daemon::run_once();

    let mut total_events = 0usize;
    let mut total_knowledge = 0usize;
    let mut total_memory = 0usize;

    for s in &summaries {
        println!(
            "  [{:<25}] events={} knowledge={} memory={}",
            s.name, s.events_processed, s.knowledge_shared, s.memory_updates
        );
        total_events += s.events_processed;
        total_knowledge += s.knowledge_shared;
        total_memory += s.memory_updates;
    }

    println!();
    println!(
        "Heartbeat complete — {} events, {} knowledge entries, {} memory updates",
        total_events, total_knowledge, total_memory
    );
}

fn cmd_health() {
    println!("DAVA LIVE — Tool health check");
    println!("{:-<50}", "");

    let tools = health::check_all_tools();
    let healthy = health::healthy_count(&tools);

    for t in &tools {
        println!(
            "  {:<16} [{}]  {}",
            t.name,
            t.status_str(),
            if t.binary_exists {
                t.binary_path.clone()
            } else {
                format!("not found: {}", t.binary_path)
            }
        );
    }

    println!("{:-<50}", "");
    println!("{}/{} tools built", healthy, tools.len());
}

fn cmd_stats() {
    println!("DAVA LIVE — Cumulative system stats");
    println!("{:-<50}", "");
    let s = stats::collect();
    println!("{}", s.display());
    println!("{:-<50}", "");
}

fn cmd_status() {
    println!("DAVA LIVE — System Status");
    println!("{:=<50}", "");

    // 1. Tool health
    println!("\n[ TOOL HEALTH ]");
    let tools = health::check_all_tools();
    let healthy = health::healthy_count(&tools);
    println!("  {}/{} binaries present", healthy, tools.len());
    for t in tools.iter().filter(|t| t.binary_exists) {
        println!("    {} — OK", t.name);
    }
    let missing: Vec<_> = tools.iter().filter(|t| !t.binary_exists).collect();
    if !missing.is_empty() {
        println!("  Missing binaries:");
        for t in &missing {
            println!("    {} — MISSING", t.name);
        }
    }

    // 2. Bus + memory stats
    println!("\n[ DAVA STATS ]");
    let s = stats::collect();
    println!("{}", s.display());

    println!("\n{:=<50}", "");
    println!("DAVA is {}.", if healthy > 0 { "ALIVE" } else { "offline (no tools built)" });
}
