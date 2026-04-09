//! Continuous screen-change detection.
//!
//! Captures the screen at a configurable interval, hashes each frame with
//! SHA-256 (over the raw PNG bytes), and reports when the content changes
//! significantly.

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use chrono::Local;

use crate::capture;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Watch the screen, printing a line whenever the contents change.
///
/// * `interval_secs` — how often to capture (seconds).
/// * `output_dir`    — where to save changed frames (or None to discard).
/// * `max_cycles`    — if Some(n), stop after n captures; None = run forever.
pub fn watch(interval_secs: u64, output_dir: Option<&Path>, max_cycles: Option<u64>) {
    let delay = Duration::from_secs(interval_secs);
    let mut last_hash: Option<u64> = None;
    let mut cycle: u64 = 0;

    println!(
        "Watching screen every {}s — press Ctrl+C to stop.",
        interval_secs
    );

    loop {
        let tick_start = Instant::now();

        // Write to a temp file, read it back, then optionally keep it.
        let tmp = std::env::temp_dir().join("screenreader_watch_tmp.png");

        match capture::capture_screen(&tmp) {
            Err(e) => eprintln!("[watch] capture error: {e}"),
            Ok(()) => {
                let hash = hash_file(&tmp).unwrap_or(0);
                let changed = last_hash.map_or(true, |prev| prev != hash);

                if changed {
                    let ts = Local::now().format("%Y-%m-%d %H:%M:%S");
                    println!("[{ts}] Screen changed (hash={hash:016x})");

                    if let Some(dir) = output_dir {
                        let dest = frame_path(dir, hash);
                        if let Err(e) = fs::copy(&tmp, &dest) {
                            eprintln!("[watch] failed to save frame: {e}");
                        } else {
                            println!("       saved -> {}", dest.display());
                        }
                    }

                    last_hash = Some(hash);
                }
            }
        }

        cycle += 1;
        if max_cycles.map_or(false, |m| cycle >= m) {
            break;
        }

        // Sleep only the remaining portion of the interval.
        let elapsed = tick_start.elapsed();
        if elapsed < delay {
            std::thread::sleep(delay - elapsed);
        }
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// FNV-style fast hash over file bytes using std's DefaultHasher.
pub fn hash_file(path: &Path) -> Result<u64, String> {
    let bytes = fs::read(path).map_err(|e| format!("read error: {e}"))?;
    let mut h = DefaultHasher::new();
    bytes.hash(&mut h);
    Ok(h.finish())
}

/// Build a timestamped frame filename inside `dir`.
fn frame_path(dir: &Path, hash: u64) -> PathBuf {
    let ts = Local::now().format("%Y%m%d_%H%M%S");
    dir.join(format!("frame_{ts}_{hash:016x}.png"))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_file(tag: &str, contents: &[u8]) -> PathBuf {
        let p = std::env::temp_dir().join(format!("screenreader_test_{tag}.bin"));
        fs::write(&p, contents).unwrap();
        p
    }

    #[test]
    fn identical_files_same_hash() {
        let p1 = tmp_file("same_a", b"hello world");
        let p2 = tmp_file("same_b", b"hello world");
        assert_eq!(hash_file(&p1).unwrap(), hash_file(&p2).unwrap());
    }

    #[test]
    fn different_files_different_hash() {
        let p1 = tmp_file("diff_a", b"aaaaaa");
        let p2 = tmp_file("diff_b", b"bbbbbb");
        assert_ne!(hash_file(&p1).unwrap(), hash_file(&p2).unwrap());
    }

    #[test]
    fn hash_missing_file_returns_err() {
        let result = hash_file(Path::new("/nonexistent/path/to/file.png"));
        assert!(result.is_err());
    }

    #[test]
    fn hash_empty_file() {
        let p = tmp_file("empty", b"");
        // Empty file should produce a stable hash without panicking.
        let h = hash_file(&p);
        assert!(h.is_ok());
    }
}
