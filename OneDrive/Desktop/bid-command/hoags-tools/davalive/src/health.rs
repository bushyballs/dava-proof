//! Health checks for all DAVA tools.
//!
//! Checks whether each tool binary exists in the workspace `target/release/`
//! directory. On Windows, binaries have a `.exe` extension.

use std::path::Path;

/// Health report for a single tool binary.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ToolHealth {
    pub name: String,
    pub binary_exists: bool,
    pub version: Option<String>,
    /// Absolute path checked.
    pub binary_path: String,
}

impl ToolHealth {
    /// Returns a short status string: "OK" or "MISSING".
    pub fn status_str(&self) -> &'static str {
        if self.binary_exists {
            "OK"
        } else {
            "MISSING"
        }
    }
}

/// All tool names tracked by DAVA.
pub const TOOL_NAMES: &[&str] = &[
    "pdffill",
    "invoicer",
    "sigstamp",
    "clauseguard",
    "sheetwise",
    "mailcraft",
    "docconv",
    "actionminer",
    "receipts",
    "propbuilder",
    "screenreader",
    "mousecontrol",
];

/// Check all tools and return health results.
///
/// The release binary directory is resolved relative to the current working
/// directory (the workspace root when running `cargo run -p davalive`).
pub fn check_all_tools() -> Vec<ToolHealth> {
    check_tools_in(None)
}

/// Check tools in a specific directory. Pass `None` to use the default
/// `target/release/` path relative to `cwd`.
pub fn check_tools_in(release_dir: Option<&Path>) -> Vec<ToolHealth> {
    let default_dir;
    let dir: &Path = if let Some(d) = release_dir {
        d
    } else {
        default_dir = default_release_dir();
        Path::new(default_dir.as_str())
    };

    TOOL_NAMES
        .iter()
        .map(|name| {
            let exe_name = if cfg!(target_os = "windows") {
                format!("{}.exe", name)
            } else {
                name.to_string()
            };
            let binary_path = dir.join(&exe_name);
            let exists = binary_path.exists();
            ToolHealth {
                name: name.to_string(),
                binary_exists: exists,
                version: if exists { Some("0.1.0".into()) } else { None },
                binary_path: binary_path.to_string_lossy().into_owned(),
            }
        })
        .collect()
}

fn default_release_dir() -> String {
    let cwd = std::env::current_dir().unwrap_or_default();
    cwd.join("target").join("release").to_string_lossy().into_owned()
}

/// Count how many tools have their binaries present.
pub fn healthy_count(tools: &[ToolHealth]) -> usize {
    tools.iter().filter(|t| t.binary_exists).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_tool_dir(tools: &[&str]) -> TempDir {
        let dir = TempDir::new().unwrap();
        for name in tools {
            let exe = if cfg!(target_os = "windows") {
                format!("{}.exe", name)
            } else {
                name.to_string()
            };
            std::fs::write(dir.path().join(exe), b"fake binary").unwrap();
        }
        dir
    }

    #[test]
    fn test_all_tools_missing_when_empty_dir() {
        let dir = TempDir::new().unwrap();
        let results = check_tools_in(Some(dir.path()));
        assert_eq!(results.len(), TOOL_NAMES.len());
        assert!(results.iter().all(|t| !t.binary_exists));
        assert_eq!(healthy_count(&results), 0);
    }

    #[test]
    fn test_detects_existing_binaries() {
        let dir = make_tool_dir(&["pdffill", "invoicer"]);
        let results = check_tools_in(Some(dir.path()));
        let pdffill = results.iter().find(|t| t.name == "pdffill").unwrap();
        let invoicer = results.iter().find(|t| t.name == "invoicer").unwrap();
        let sigstamp = results.iter().find(|t| t.name == "sigstamp").unwrap();
        assert!(pdffill.binary_exists);
        assert!(invoicer.binary_exists);
        assert!(!sigstamp.binary_exists);
    }

    #[test]
    fn test_healthy_count() {
        let dir = make_tool_dir(&["pdffill", "invoicer", "clauseguard"]);
        let results = check_tools_in(Some(dir.path()));
        assert_eq!(healthy_count(&results), 3);
    }

    #[test]
    fn test_version_only_when_binary_exists() {
        let dir = make_tool_dir(&["pdffill"]);
        let results = check_tools_in(Some(dir.path()));
        let pdffill = results.iter().find(|t| t.name == "pdffill").unwrap();
        let missing = results.iter().find(|t| t.name == "invoicer").unwrap();
        assert_eq!(pdffill.version.as_deref(), Some("0.1.0"));
        assert!(missing.version.is_none());
    }

    #[test]
    fn test_status_str() {
        let ok = ToolHealth {
            name: "pdffill".into(),
            binary_exists: true,
            version: Some("0.1.0".into()),
            binary_path: "/fake/path".into(),
        };
        let missing = ToolHealth {
            name: "invoicer".into(),
            binary_exists: false,
            version: None,
            binary_path: "/fake/path".into(),
        };
        assert_eq!(ok.status_str(), "OK");
        assert_eq!(missing.status_str(), "MISSING");
    }

    #[test]
    fn test_tool_names_count() {
        assert_eq!(TOOL_NAMES.len(), 12);
    }

    #[test]
    fn test_all_tools_present_when_dir_has_all() {
        let dir = make_tool_dir(TOOL_NAMES);
        let results = check_tools_in(Some(dir.path()));
        assert_eq!(healthy_count(&results), TOOL_NAMES.len());
        assert!(results.iter().all(|t| t.binary_exists));
    }

    #[test]
    fn test_partial_tools() {
        let dir = make_tool_dir(&["pdffill", "sigstamp", "clauseguard"]);
        let results = check_tools_in(Some(dir.path()));
        assert_eq!(healthy_count(&results), 3);
        assert_eq!(results.len(), TOOL_NAMES.len());
    }

    #[test]
    fn test_tool_names_are_unique() {
        let mut names: Vec<&str> = TOOL_NAMES.to_vec();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), TOOL_NAMES.len(), "Duplicate tool names found");
    }

    #[test]
    fn test_tool_names_alphabetical_sanity() {
        // All tool names should be lowercase alphanumeric
        for name in TOOL_NAMES {
            assert!(name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()),
                "Tool name '{}' should be lowercase alphanumeric", name);
        }
    }

    #[test]
    fn test_binary_path_contains_tool_name() {
        let dir = TempDir::new().unwrap();
        let results = check_tools_in(Some(dir.path()));
        for t in &results {
            assert!(t.binary_path.contains(&t.name),
                "Path '{}' should contain tool name '{}'", t.binary_path, t.name);
        }
    }
}
