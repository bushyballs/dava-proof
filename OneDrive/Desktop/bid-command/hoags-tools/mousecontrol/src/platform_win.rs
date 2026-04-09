/// Windows implementation — all real actions are executed via PowerShell
/// so the binary itself needs no unsafe code and no C FFI dependencies.

pub fn execute_click(x: i32, y: i32, button: &str) -> Result<String, String> {
    let (down_flag, up_flag) = match button.to_lowercase().as_str() {
        "right" => ("0x0008", "0x0010"), // RIGHTDOWN / RIGHTUP
        _ => ("0x0002", "0x0004"),        // LEFTDOWN  / LEFTUP
    };

    let script = format!(
        r#"Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class HoagsMouse {{
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int X, int Y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint dwFlags, int dx, int dy, uint dwData, int dwExtraInfo);
}}
"@
[HoagsMouse]::SetCursorPos({x}, {y}) | Out-Null
[HoagsMouse]::mouse_event({down}, 0, 0, 0, 0)
[HoagsMouse]::mouse_event({up}, 0, 0, 0, 0)"#,
        x = x,
        y = y,
        down = down_flag,
        up = up_flag
    );

    run_powershell(&script)?;
    Ok(format!("Clicked {} at ({}, {})", button, x, y))
}

pub fn execute_move(x: i32, y: i32) -> Result<String, String> {
    let script = format!(
        r#"Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class HoagsMouseMove {{
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int X, int Y);
}}
"@
[HoagsMouseMove]::SetCursorPos({x}, {y}) | Out-Null"#,
        x = x,
        y = y
    );

    run_powershell(&script)?;
    Ok(format!("Moved to ({}, {})", x, y))
}

pub fn execute_type(text: &str) -> Result<String, String> {
    // Escape single-quotes for SendKeys and handle special chars
    let escaped = text
        .replace('\'', "''")
        // SendKeys treats these chars specially — wrap each in braces
        .replace('+', "{+}")
        .replace('^', "{^}")
        .replace('%', "{%}")
        .replace('~', "{~}")
        .replace('(', "{(}")
        .replace(')', "{)}")
        .replace('[', "{[}")
        .replace(']', "{]}")
        .replace('{', "{{}")
        .replace('}', "{}}");

    let script = format!(
        r#"Add-Type -AssemblyName System.Windows.Forms
[System.Windows.Forms.SendKeys]::SendWait('{}')"#,
        escaped
    );

    run_powershell(&script)?;
    Ok(format!("Typed: {}", text))
}

pub fn execute_key(combo: &str) -> Result<String, String> {
    // Convert human-readable combos to SendKeys notation
    // e.g. "ctrl+s" -> "^s",  "alt+tab" -> "%{TAB}",  "enter" -> "{ENTER}"
    let sendkeys = combo_to_sendkeys(combo)?;

    let script = format!(
        r#"Add-Type -AssemblyName System.Windows.Forms
[System.Windows.Forms.SendKeys]::SendWait('{}')"#,
        sendkeys
    );

    run_powershell(&script)?;
    Ok(format!("Sent key combo: {} -> {}", combo, sendkeys))
}

pub fn execute_screenshot(output: &str) -> Result<String, String> {
    // Uses .NET to capture the primary screen and save as PNG
    let script = format!(
        r#"Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$bmp = New-Object System.Drawing.Bitmap($bounds.Width, $bounds.Height)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size)
$g.Dispose()
$bmp.Save('{output}')
$bmp.Dispose()
Write-Output "saved""#,
        output = output.replace('\'', "''")
    );

    run_powershell(&script)?;
    Ok(format!("Screenshot saved to: {}", output))
}

pub fn get_cursor_position() -> (i32, i32) {
    let script = r#"Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public struct POINT { public int X; public int Y; }
public class HoagsCursor {
    [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT lpPoint);
}
"@
$pt = New-Object POINT
[HoagsCursor]::GetCursorPos([ref]$pt) | Out-Null
"$($pt.X),$($pt.Y)""#;

    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .ok();

    match output {
        Some(out) if out.status.success() => {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let parts: Vec<&str> = s.split(',').collect();
            if parts.len() == 2 {
                let x: i32 = parts[0].parse().unwrap_or(0);
                let y: i32 = parts[1].parse().unwrap_or(0);
                return (x, y);
            }
            (0, 0)
        }
        _ => (0, 0),
    }
}

// ── Internals ────────────────────────────────────────────────────────────────

fn run_powershell(script: &str) -> Result<(), String> {
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .map_err(|e| format!("Failed to launch PowerShell: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Err(format!(
            "PowerShell error (exit {:?}):\nstderr: {}\nstdout: {}",
            output.status.code(),
            stderr.trim(),
            stdout.trim()
        ))
    }
}

/// Convert human-readable key combos to Windows SendKeys notation.
///
/// Rules:
///   ctrl  -> ^
///   alt   -> %
///   shift -> +
///   win   -> ^%  (approximate — no direct SendKeys equivalent; use ^ for now)
///   Named keys (tab, enter, esc, …) wrapped in {KEYNAME}
///   Single char keys passed as-is after modifier
fn combo_to_sendkeys(combo: &str) -> Result<String, String> {
    let lower = combo.to_lowercase();
    let parts: Vec<&str> = lower.split('+').collect();
    if parts.is_empty() {
        return Err("Empty key combo".to_string());
    }

    let mut modifiers = String::new();
    let key = parts.last().unwrap();

    for &part in &parts[..parts.len() - 1] {
        match part.trim() {
            "ctrl" | "control" => modifiers.push('^'),
            "alt" => modifiers.push('%'),
            "shift" => modifiers.push('+'),
            "win" | "windows" => modifiers.push_str("^%"), // best approximation
            other => return Err(format!("Unknown modifier: '{}'", other)),
        }
    }

    let key_str = named_key(key.trim());
    Ok(format!("{}{}", modifiers, key_str))
}

fn named_key(k: &str) -> String {
    match k {
        "tab" => "{TAB}".into(),
        "enter" | "return" => "{ENTER}".into(),
        "esc" | "escape" => "{ESC}".into(),
        "space" => " ".into(),
        "backspace" | "bs" => "{BACKSPACE}".into(),
        "delete" | "del" => "{DELETE}".into(),
        "home" => "{HOME}".into(),
        "end" => "{END}".into(),
        "pageup" | "pgup" => "{PGUP}".into(),
        "pagedown" | "pgdn" => "{PGDN}".into(),
        "up" => "{UP}".into(),
        "down" => "{DOWN}".into(),
        "left" => "{LEFT}".into(),
        "right" => "{RIGHT}".into(),
        "f1" => "{F1}".into(),
        "f2" => "{F2}".into(),
        "f3" => "{F3}".into(),
        "f4" => "{F4}".into(),
        "f5" => "{F5}".into(),
        "f6" => "{F6}".into(),
        "f7" => "{F7}".into(),
        "f8" => "{F8}".into(),
        "f9" => "{F9}".into(),
        "f10" => "{F10}".into(),
        "f11" => "{F11}".into(),
        "f12" => "{F12}".into(),
        other => other.to_string(), // single character — pass through
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combo_ctrl_s_converts_correctly() {
        assert_eq!(combo_to_sendkeys("ctrl+s").unwrap(), "^s");
    }

    #[test]
    fn combo_alt_tab_converts_correctly() {
        assert_eq!(combo_to_sendkeys("alt+tab").unwrap(), "%{TAB}");
    }

    #[test]
    fn combo_enter_alone_converts_correctly() {
        assert_eq!(combo_to_sendkeys("enter").unwrap(), "{ENTER}");
    }

    #[test]
    fn combo_ctrl_shift_s_converts_correctly() {
        assert_eq!(combo_to_sendkeys("ctrl+shift+s").unwrap(), "^+s");
    }

    #[test]
    fn combo_unknown_modifier_returns_err() {
        let result = combo_to_sendkeys("super+s");
        assert!(result.is_err());
    }

    #[test]
    fn named_key_f5_converts() {
        assert_eq!(named_key("f5"), "{F5}");
    }

    #[test]
    fn named_key_unknown_passthrough() {
        assert_eq!(named_key("a"), "a");
    }
}
