//! Screen text extraction: window titles + focused element via PowerShell/UIAutomation.

use std::process::Command;

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ScreenText {
    /// Titles of all visible top-level windows.
    pub window_titles: Vec<String>,
    /// Text of the currently focused UI element (best-effort).
    pub focused_element: Option<String>,
    /// Title of the window that currently has keyboard focus.
    pub focused_window: Option<String>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Capture all visible text from the current screen state:
/// - titles of every window with a non-empty `MainWindowTitle`
/// - the text/name of the focused UIAutomation element
/// - the title of the foreground window
pub fn read_screen() -> ScreenText {
    let window_titles = collect_window_titles();
    let (focused_window, focused_element) = collect_focused_element();

    ScreenText {
        window_titles,
        focused_element,
        focused_window,
    }
}

/// Print a human-readable report of the screen text to stdout.
pub fn print_screen_text(st: &ScreenText) {
    println!("=== Visible Windows ===");
    if st.window_titles.is_empty() {
        println!("  (none found)");
    } else {
        for title in &st.window_titles {
            println!("  - {title}");
        }
    }

    println!();
    println!("=== Focused Window ===");
    match &st.focused_window {
        Some(t) => println!("  {t}"),
        None => println!("  (unknown)"),
    }

    println!();
    println!("=== Focused Element ===");
    match &st.focused_element {
        Some(t) => println!("  {t}"),
        None => println!("  (unknown)"),
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn collect_window_titles() -> Vec<String> {
    let script = r#"Get-Process | Where-Object { $_.MainWindowTitle -ne '' } | ForEach-Object { $_.MainWindowTitle }"#;

    let out = Command::new("powershell")
        .args(["-NoProfile", "-Command", script])
        .output()
        .ok();

    match out {
        Some(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect(),
        _ => Vec::new(),
    }
}

/// Returns (focused_window_title, focused_element_text).
///
/// Uses UIAutomation via PowerShell to get the focused element's name/value.
/// This is best-effort — many apps use non-standard accessibility trees.
fn collect_focused_element() -> (Option<String>, Option<String>) {
    // Script returns two lines:
    //   Line 1: foreground window title
    //   Line 2: focused element AutomationId + Name
    let script = r#"Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @'
using System;
using System.Windows.Automation;
public class FocusHelper {
    public static string FocusedName() {
        try {
            var el = AutomationElement.FocusedElement;
            if (el == null) return "";
            var name = el.GetCurrentPropertyValue(AutomationElement.NameProperty) as string;
            var val  = el.GetCurrentPropertyValue(ValuePattern.ValueProperty) as string;
            if (!string.IsNullOrEmpty(val))  return val;
            if (!string.IsNullOrEmpty(name)) return name;
            return "";
        } catch { return ""; }
    }
    public static string ForegroundTitle() {
        try {
            var el = AutomationElement.FocusedElement;
            var win = TreeWalker.ControlViewWalker.GetParent(el);
            while (win != null) {
                if ((int)win.GetCurrentPropertyValue(AutomationElement.ControlTypeProperty) == 50032) {
                    return (string)win.GetCurrentPropertyValue(AutomationElement.NameProperty);
                }
                win = TreeWalker.ControlViewWalker.GetParent(win);
            }
            return "";
        } catch { return ""; }
    }
}
'@ -ErrorAction SilentlyContinue
[FocusHelper]::ForegroundTitle()
[FocusHelper]::FocusedName()"#;

    let out = Command::new("powershell")
        .args(["-NoProfile", "-Command", script])
        .output()
        .ok();

    match out {
        Some(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout);
            let mut lines = text.lines().map(str::trim).filter(|s| !s.is_empty());
            let win = lines.next().map(String::from);
            let elem = lines.next().map(String::from);
            (win, elem)
        }
        _ => (None, None),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// read_screen() must not panic regardless of environment.
    #[test]
    fn read_screen_does_not_panic() {
        let st = read_screen();
        // In headless CI there may be no windows.
        let _ = st.window_titles.len();
    }

    #[test]
    fn print_screen_text_empty() {
        let st = ScreenText {
            window_titles: Vec::new(),
            focused_element: None,
            focused_window: None,
        };
        // Should not panic.
        print_screen_text(&st);
    }

    #[test]
    fn print_screen_text_populated() {
        let st = ScreenText {
            window_titles: vec!["Notepad".into(), "VS Code".into()],
            focused_element: Some("Hello, world!".into()),
            focused_window: Some("Notepad".into()),
        };
        print_screen_text(&st);
    }
}
