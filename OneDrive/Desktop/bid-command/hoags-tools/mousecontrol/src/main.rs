mod actions;
mod platform_win;

use clap::{Parser, Subcommand};

/// MOUSECONTROL — DAVA's hands.
///
/// Controls mouse and keyboard on Windows.
/// All commands are DRY RUN by default.
/// Pass --live to actually execute the action.
#[derive(Parser, Debug)]
#[command(name = "mousecontrol", about = "DAVA's hands — mouse/keyboard control")]
#[command(version = "0.1.0")]
struct Cli {
    /// Execute for real (default is dry-run — prints what would happen)
    #[arg(long, global = true)]
    live: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Click at a screen position
    Click {
        #[arg(long)]
        x: i32,
        #[arg(long)]
        y: i32,
        /// "left" or "right"
        #[arg(long, default_value = "left")]
        button: String,
    },
    /// Move the mouse cursor to a position
    Move {
        #[arg(long)]
        x: i32,
        #[arg(long)]
        y: i32,
    },
    /// Type text at the current cursor focus
    Type {
        #[arg(long)]
        text: String,
    },
    /// Send a key combination (e.g. ctrl+s, alt+tab, enter)
    Key {
        #[arg(long)]
        combo: String,
    },
    /// Report the current mouse cursor position
    Position,
    /// Run a sequence of actions from a JSON script file
    Script {
        #[arg(long)]
        file: String,
    },
}

fn main() {
    let cli = Cli::parse();
    let live = cli.live;

    if !live {
        eprintln!("[DRY RUN MODE] Pass --live to execute for real.");
    }

    let result = match cli.command {
        Commands::Click { x, y, button } => {
            let action = actions::Action::Click { x, y, button };
            actions::execute(&action, live)
        }
        Commands::Move { x, y } => {
            let action = actions::Action::Move { x, y };
            actions::execute(&action, live)
        }
        Commands::Type { text } => {
            let action = actions::Action::Type { text };
            actions::execute(&action, live)
        }
        Commands::Key { combo } => {
            let action = actions::Action::Key { combo };
            actions::execute(&action, live)
        }
        Commands::Position => {
            if live {
                let (x, y) = platform_win::get_cursor_position();
                Ok(format!("Mouse position: ({}, {})", x, y))
            } else {
                Ok("[DRY RUN] Would report current mouse position".to_string())
            }
        }
        Commands::Script { file } => run_script(&file, live),
    };

    match result {
        Ok(msg) => println!("{}", msg),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn run_script(file: &str, live: bool) -> Result<String, String> {
    let script_actions = actions::load_script(file)?;
    let total = script_actions.len();
    let mut results = Vec::with_capacity(total);

    for (i, action) in script_actions.iter().enumerate() {
        let msg = actions::execute(action, live)?;
        println!("[{}/{}] {}", i + 1, total, msg);
        results.push(msg);
    }

    Ok(format!("Script complete: {} action(s) executed", total))
}
