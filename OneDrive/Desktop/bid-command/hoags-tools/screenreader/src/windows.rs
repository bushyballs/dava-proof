//! Window enumeration via PowerShell Get-Process.

use std::process::Command;

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub title: String,
    pub process_name: String,
    /// Left edge in screen coordinates (0 when not retrieved).
    pub x: i32,
    /// Top edge in screen coordinates (0 when not retrieved).
    pub y: i32,
    /// Window client width (0 when not retrieved).
    pub width: i32,
    /// Window client height (0 when not retrieved).
    pub height: i32,
    pub is_visible: bool,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Return all visible windows that have a non-empty title.
///
/// Uses `Get-Process` which is available on every modern Windows installation
/// without requiring elevated permissions.  Position/size information is
/// currently left at zero; a future version can pinvoke `GetWindowRect`.
pub fn list_windows() -> Vec<WindowInfo> {
    // Output format: ProcessName|PID|MainWindowTitle
    let script = r#"Get-Process | Where-Object { $_.MainWindowTitle -ne '' } | ForEach-Object {
    "$($_.ProcessName)|$($_.Id)|$($_.MainWindowTitle)"
}"#;

    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", script])
        .output()
        .ok();

    match output {
        Some(out) if out.status.success() => {
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter_map(parse_window_line)
                .collect()
        }
        Some(out) => {
            let err = String::from_utf8_lossy(&out.stderr);
            eprintln!("screenreader: window list error: {err}");
            Vec::new()
        }
        None => {
            eprintln!("screenreader: failed to launch PowerShell");
            Vec::new()
        }
    }
}

/// Parse a single `ProcessName|PID|Title` line.
/// Returns `None` for malformed or empty lines.
pub fn parse_window_line(line: &str) -> Option<WindowInfo> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    // Split into at most 3 parts so titles containing '|' are preserved.
    let parts: Vec<&str> = line.splitn(3, '|').collect();
    if parts.len() < 3 {
        return None;
    }
    let title = parts[2].trim().to_string();
    if title.is_empty() {
        return None;
    }
    Some(WindowInfo {
        process_name: parts[0].trim().to_string(),
        // PID is captured but not exposed in the struct yet; kept for future use.
        title,
        x: 0,
        y: 0,
        width: 0,
        height: 0,
        is_visible: true,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_line() {
        let w = parse_window_line("chrome|1234|Google Chrome - New Tab").unwrap();
        assert_eq!(w.process_name, "chrome");
        assert_eq!(w.title, "Google Chrome - New Tab");
        assert!(w.is_visible);
    }

    #[test]
    fn parse_line_with_pipe_in_title() {
        let w = parse_window_line("notepad|999|foo | bar | baz").unwrap();
        assert_eq!(w.title, "foo | bar | baz");
    }

    #[test]
    fn parse_empty_line_returns_none() {
        assert!(parse_window_line("").is_none());
        assert!(parse_window_line("   ").is_none());
    }

    #[test]
    fn parse_missing_fields_returns_none() {
        // Only one field — no separators.
        assert!(parse_window_line("chrome").is_none());
        // Two fields — title missing.
        assert!(parse_window_line("chrome|1234").is_none());
    }

    #[test]
    fn parse_empty_title_returns_none() {
        assert!(parse_window_line("chrome|1234|").is_none());
        assert!(parse_window_line("chrome|1234|   ").is_none());
    }

    /// list_windows() should not panic regardless of environment.
    #[test]
    fn list_windows_does_not_panic() {
        let wins = list_windows();
        // On a headless CI box there may be zero windows — that is fine.
        let _ = wins.len();
    }
}
