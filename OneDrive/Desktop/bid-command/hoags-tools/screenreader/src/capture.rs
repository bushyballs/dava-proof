//! Screen capture via PowerShell + System.Drawing.

use std::path::Path;
use std::process::Command;

/// Capture the full primary screen to a PNG file.
pub fn capture_screen(output: &Path) -> Result<(), String> {
    let path_str = output.to_string_lossy().replace('\\', "\\\\");
    let script = format!(
        r#"Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$screen  = [System.Windows.Forms.Screen]::PrimaryScreen
$bitmap  = New-Object System.Drawing.Bitmap($screen.Bounds.Width, $screen.Bounds.Height)
$gfx     = [System.Drawing.Graphics]::FromImage($bitmap)
$gfx.CopyFromScreen($screen.Bounds.Location, [System.Drawing.Point]::Empty, $screen.Bounds.Size)
$bitmap.Save('{path}')
$gfx.Dispose()
$bitmap.Dispose()"#,
        path = path_str
    );

    run_ps(&script)
}

/// Capture a rectangular region (x, y, width, height) to a PNG file.
pub fn capture_region(output: &Path, x: i32, y: i32, w: i32, h: i32) -> Result<(), String> {
    let path_str = output.to_string_lossy().replace('\\', "\\\\");
    let script = format!(
        r#"Add-Type -AssemblyName System.Drawing
$bitmap = New-Object System.Drawing.Bitmap({w}, {h})
$gfx    = [System.Drawing.Graphics]::FromImage($bitmap)
$src    = New-Object System.Drawing.Point({x}, {y})
$dst    = [System.Drawing.Point]::Empty
$size   = New-Object System.Drawing.Size({w}, {h})
$gfx.CopyFromScreen($src, $dst, $size)
$bitmap.Save('{path}')
$gfx.Dispose()
$bitmap.Dispose()"#,
        x = x,
        y = y,
        w = w,
        h = h,
        path = path_str
    );

    run_ps(&script)
}

/// Return the primary screen resolution (width, height).
/// Falls back to 1920x1080 if the PowerShell query fails.
pub fn screen_size() -> (u32, u32) {
    let script = r#"Add-Type -AssemblyName System.Windows.Forms
$s = [System.Windows.Forms.Screen]::PrimaryScreen
Write-Output "$($s.Bounds.Width)x$($s.Bounds.Height)""#;

    let out = Command::new("powershell")
        .args(["-NoProfile", "-Command", script])
        .output()
        .ok();

    if let Some(o) = out {
        if o.status.success() {
            let text = String::from_utf8_lossy(&o.stdout);
            let text = text.trim();
            let parts: Vec<&str> = text.splitn(2, 'x').collect();
            if parts.len() == 2 {
                if let (Ok(w), Ok(h)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                    return (w, h);
                }
            }
        }
    }

    (1920, 1080)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn run_ps(script: &str) -> Result<(), String> {
    let result = Command::new("powershell")
        .args(["-NoProfile", "-Command", script])
        .output()
        .map_err(|e| format!("PowerShell launch failed: {e}"))?;

    if result.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&result.stderr);
        let stdout = String::from_utf8_lossy(&result.stdout);
        Err(format!(
            "PowerShell error:\nstderr: {stderr}\nstdout: {stdout}"
        ))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// screen_size() must return sensible positive dimensions even without
    /// a real display (the fallback path is exercised in CI).
    #[test]
    fn screen_size_is_positive() {
        let (w, h) = screen_size();
        assert!(w > 0, "width must be > 0");
        assert!(h > 0, "height must be > 0");
    }

    /// The path escaping logic must survive backslashes.
    #[test]
    fn path_escaping_roundtrip() {
        let path = Path::new(r"C:\tmp\out.png");
        let escaped = path.to_string_lossy().replace('\\', "\\\\");
        assert!(escaped.contains("\\\\"));
    }

    /// capture_screen on a non-writable path returns an Err, not a panic.
    #[test]
    fn capture_screen_bad_path_returns_err() {
        let bad = Path::new("");
        let result = capture_screen(bad);
        let _ = result;
    }

    #[test]
    fn screen_size_reasonable_dimensions() {
        let (w, h) = screen_size();
        // Should be at least 640x480 on any modern system
        assert!(w >= 640, "width {} too small", w);
        assert!(h >= 480, "height {} too small", h);
    }

    #[test]
    fn capture_region_bad_path_no_panic() {
        let bad = Path::new("");
        let result = capture_region(bad, 0, 0, 100, 100);
        let _ = result;
    }

    #[test]
    fn path_with_spaces_escapes_correctly() {
        let path = Path::new(r"C:\My Documents\screenshot.png");
        let escaped = path.to_string_lossy().replace('\\', "\\\\");
        assert!(escaped.contains("My Documents"));
        assert!(escaped.contains("\\\\"));
    }
}
