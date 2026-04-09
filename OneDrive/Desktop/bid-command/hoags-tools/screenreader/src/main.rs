//! SCREENREADER — DAVA's eyes.
//!
//! CLI for screen capture, text extraction, window enumeration, and change
//! detection.  All heavy lifting is delegated to PowerShell + UIAutomation
//! so no native compilation against Win32 is required.

mod capture;
mod read;
mod watch;
mod windows;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

// ---------------------------------------------------------------------------
// CLI definition
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(
    name = "screenreader",
    version = env!("CARGO_PKG_VERSION"),
    about = "DAVA's eyes — screen capture, text extraction, window listing, change detection",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Capture the screen (full or a region) to a PNG file.
    Capture {
        /// Output path for the screenshot PNG.
        #[arg(long, default_value = "screenshot.png")]
        output: PathBuf,

        /// Capture a rectangular region: x,y,width,height (e.g. 100,200,800,600).
        #[arg(long)]
        region: Option<String>,
    },

    /// Capture the screen and extract all visible text (window titles, focused element).
    Read,

    /// Continuously capture the screen every N seconds and report changes.
    Watch {
        /// Capture interval in seconds.
        #[arg(long, default_value_t = 5)]
        interval: u64,

        /// Directory to save changed frames (omit to discard frames).
        #[arg(long)]
        output_dir: Option<PathBuf>,
    },

    /// List all visible windows with titles and process names.
    Windows,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Capture { output, region } => cmd_capture(output, region),
        Commands::Read => cmd_read(),
        Commands::Watch { interval, output_dir } => cmd_watch(interval, output_dir),
        Commands::Windows => cmd_windows(),
    }
}

// ---------------------------------------------------------------------------
// Command implementations
// ---------------------------------------------------------------------------

fn cmd_capture(output: PathBuf, region: Option<String>) {
    match region {
        None => {
            println!("Capturing full screen -> {}", output.display());
            match capture::capture_screen(&output) {
                Ok(()) => println!("Saved: {}", output.display()),
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some(spec) => {
            let (x, y, w, h) = parse_region(&spec).unwrap_or_else(|e| {
                eprintln!("Invalid --region '{spec}': {e}");
                std::process::exit(1);
            });
            println!("Capturing region {x},{y},{w},{h} -> {}", output.display());
            match capture::capture_region(&output, x, y, w, h) {
                Ok(()) => println!("Saved: {}", output.display()),
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }
    }

    // Print screen size as a courtesy.
    let (sw, sh) = capture::screen_size();
    println!("Screen resolution: {sw}x{sh}");
}

fn cmd_read() {
    println!("Reading screen text...");
    let st = read::read_screen();
    read::print_screen_text(&st);
}

fn cmd_watch(interval: u64, output_dir: Option<PathBuf>) {
    watch::watch(interval, output_dir.as_deref(), None);
}

fn cmd_windows() {
    let wins = windows::list_windows();
    if wins.is_empty() {
        println!("No visible windows found.");
        return;
    }
    println!("{:<30} {}", "PROCESS", "TITLE");
    println!("{}", "-".repeat(72));
    for w in &wins {
        println!("{:<30} {}", w.process_name, w.title);
    }
    println!("\n{} window(s) listed.", wins.len());
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse a region spec "x,y,w,h" into four i32 values.
fn parse_region(s: &str) -> Result<(i32, i32, i32, i32), String> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 4 {
        return Err("expected exactly 4 comma-separated integers".into());
    }
    let nums: Result<Vec<i32>, _> = parts.iter().map(|p| p.trim().parse::<i32>()).collect();
    match nums {
        Ok(v) => {
            if v[2] <= 0 || v[3] <= 0 {
                Err("width and height must be positive".into())
            } else {
                Ok((v[0], v[1], v[2], v[3]))
            }
        }
        Err(e) => Err(format!("parse error: {e}")),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_region_valid() {
        let (x, y, w, h) = parse_region("100,200,800,600").unwrap();
        assert_eq!((x, y, w, h), (100, 200, 800, 600));
    }

    #[test]
    fn parse_region_with_spaces() {
        let (x, y, w, h) = parse_region(" 0 , 0 , 1920 , 1080 ").unwrap();
        assert_eq!((x, y, w, h), (0, 0, 1920, 1080));
    }

    #[test]
    fn parse_region_too_few_parts() {
        assert!(parse_region("100,200,800").is_err());
    }

    #[test]
    fn parse_region_non_numeric() {
        assert!(parse_region("a,b,c,d").is_err());
    }

    #[test]
    fn parse_region_zero_dimensions() {
        assert!(parse_region("0,0,0,0").is_err());
    }

    #[test]
    fn parse_region_negative_offset_ok() {
        // Negative x/y are allowed (for multi-monitor setups).
        let (x, y, w, h) = parse_region("-100,-200,400,300").unwrap();
        assert_eq!((x, y, w, h), (-100, -200, 400, 300));
    }

    #[test]
    fn parse_region_too_many_parts() {
        assert!(parse_region("0,0,100,100,50").is_err());
    }

    #[test]
    fn parse_region_empty_string() {
        assert!(parse_region("").is_err());
    }

    #[test]
    fn parse_region_negative_width_fails() {
        assert!(parse_region("0,0,-100,100").is_err());
    }

    #[test]
    fn parse_region_negative_height_fails() {
        assert!(parse_region("0,0,100,-1").is_err());
    }

    #[test]
    fn parse_region_float_values_fail() {
        assert!(parse_region("0.5,0,100,100").is_err());
    }

    #[test]
    fn parse_region_positive_x_y() {
        let (x, y, w, h) = parse_region("10,20,640,480").unwrap();
        assert_eq!(x, 10);
        assert_eq!(y, 20);
        assert_eq!(w, 640);
        assert_eq!(h, 480);
    }

    #[test]
    fn parse_region_large_values() {
        let (x, y, w, h) = parse_region("0,0,3840,2160").unwrap();
        assert_eq!((x, y, w, h), (0, 0, 3840, 2160));
    }

    #[test]
    fn parse_region_minimal_1x1() {
        let (x, y, w, h) = parse_region("0,0,1,1").unwrap();
        assert_eq!((x, y, w, h), (0, 0, 1, 1));
    }

    #[test]
    fn parse_region_whitespace_only_fails() {
        // Whitespace between commas yields non-numeric tokens after trim
        // strip happens inside the parse_region trim — this should error.
        assert!(parse_region("  ,  ,  ,  ").is_err());
    }

    #[test]
    fn parse_region_zero_width_fails() {
        assert!(parse_region("0,0,0,100").is_err());
    }
}
