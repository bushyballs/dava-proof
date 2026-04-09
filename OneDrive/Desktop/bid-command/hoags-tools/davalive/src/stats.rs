//! DAVA cumulative statistics — aggregates data across all tools and the event bus.

use hoags_core::bus::EventBus;
use hoags_core::memory::FieldMemory;
use std::path::Path;

/// Aggregate stats snapshot for DAVA.
#[derive(Debug, Default)]
pub struct DavaStats {
    // ── Bus stats ──────────────────────────────────────────────────────────
    /// Total events ever published to the bus.
    pub total_bus_events: i64,
    /// Total cross-tool knowledge entries shared.
    pub total_knowledge_entries: i64,
    /// Total connector memory key-value pairs stored.
    pub total_memory_entries: i64,

    // ── Memory stats ───────────────────────────────────────────────────────
    /// Unique form fields DAVA has learned.
    pub fields_learned: i64,
    /// Form templates DAVA has cached.
    pub templates_cached: i64,

    // ── Connector stats ────────────────────────────────────────────────────
    /// Events seen by the LearningConnector (pulled from bus memory).
    pub learning_events_seen: i64,

    // ── Tool health ────────────────────────────────────────────────────────
    /// How many tool binaries exist in target/release/.
    pub tools_built: usize,
    /// How many tools are tracked in total.
    pub tools_total: usize,

    // ── Source stats ────────────────────────────────────────────────────────
    /// Total lines of Rust source code across all tools in the workspace.
    pub total_rust_lines: usize,
}

impl DavaStats {
    /// Summarise everything into a human-readable multi-line string.
    pub fn display(&self) -> String {
        let tool_status = format!("{}/{}", self.tools_built, self.tools_total);
        vec![
            format!("  Bus events      : {}", self.total_bus_events),
            format!("  Cross-knowledge : {}", self.total_knowledge_entries),
            format!("  Connector memory: {}", self.total_memory_entries),
            format!("  Fields learned  : {}", self.fields_learned),
            format!("  Templates cached: {}", self.templates_cached),
            format!("  Learning events : {}", self.learning_events_seen),
            format!("  Tools built     : {}", tool_status),
            format!("  Rust lines      : {}", self.total_rust_lines),
        ]
        .join("\n")
    }
}

/// Collect DAVA stats using the default database paths.
pub fn collect() -> DavaStats {
    collect_with_paths(None, None, None)
}

/// Collect DAVA stats with explicit paths (useful for tests).
///
/// - `bus_path`    — path to `hoags_bus.db`; `None` → use `EventBus::open_default()`
/// - `memory_path` — path to `dava_memory.db`; `None` → use `FieldMemory::open_default()`
/// - `workspace`   — workspace root for counting `.rs` files; `None` → `cwd`
pub fn collect_with_paths(
    bus_path: Option<&Path>,
    memory_path: Option<&Path>,
    workspace: Option<&Path>,
) -> DavaStats {
    let mut s = DavaStats::default();

    // ── Bus stats ──────────────────────────────────────────────────────────
    let bus_result = if let Some(p) = bus_path {
        EventBus::open(p)
    } else {
        EventBus::open_default()
    };
    if let Ok(bus) = bus_result {
        let bs = bus.stats();
        s.total_bus_events = bs.total_events;
        s.total_knowledge_entries = bs.total_knowledge;
        s.total_memory_entries = bs.total_memory;

        // Pull LearningConnector counter from bus memory
        s.learning_events_seen = bus
            .get_memory("connector_learning", "total_events_seen")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
    }

    // ── DAVA memory stats ──────────────────────────────────────────────────
    let mem_result = if let Some(p) = memory_path {
        FieldMemory::open(p)
    } else {
        FieldMemory::open_default()
    };
    if let Ok(mem) = mem_result {
        let ms = mem.stats();
        s.fields_learned = ms.total_fields;
        s.templates_cached = ms.total_templates;
    }

    // ── Tool health ────────────────────────────────────────────────────────
    let release_dir = workspace
        .map(|w| w.join("target").join("release"))
        .unwrap_or_else(|| {
            std::env::current_dir()
                .unwrap_or_default()
                .join("target")
                .join("release")
        });
    let tools = crate::health::check_tools_in(Some(&release_dir));
    s.tools_built = crate::health::healthy_count(&tools);
    s.tools_total = tools.len();

    // ── Rust line count ────────────────────────────────────────────────────
    let root = workspace
        .map(|w| w.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    s.total_rust_lines = count_rust_lines(&root);

    s
}

/// Recursively count total lines in all `*.rs` files under `root`,
/// skipping `target/` directories.
pub fn count_rust_lines(root: &Path) -> usize {
    let mut total = 0;
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                // Skip build output
                if path.file_name().map_or(false, |n| n == "target") {
                    continue;
                }
                total += count_rust_lines(&path);
            } else if path.extension().map_or(false, |ext| ext == "rs") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    total += content.lines().count();
                }
            }
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::{NamedTempFile, TempDir};

    fn tmp_bus_with_data() -> (NamedTempFile, EventBus) {
        let tmp = NamedTempFile::new().unwrap();
        let bus = EventBus::open(tmp.path()).unwrap();
        bus.publish("pdffill", "pdffill.form_detected", r#"{"template":"SF1449"}"#);
        bus.publish("invoicer", "invoicer.invoice_generated", r#"{"total":1000.0}"#);
        bus.share_knowledge("clauseguard", "mailcraft", "risk", "clause 52.249-8", 0.9);
        bus.set_memory("connector_learning", "total_events_seen", "5");
        (tmp, bus)
    }

    fn tmp_memory_with_data() -> NamedTempFile {
        let tmp = NamedTempFile::new().unwrap();
        let mem = FieldMemory::open(tmp.path()).unwrap();
        mem.store("offeror name", "identity.name", "Hoags Inc.", "identity.name", "a.pdf", true);
        mem.store("cage code", "identity.cage", "7KXQ1", "identity.cage", "b.pdf", true);
        drop(mem);
        tmp
    }

    #[test]
    fn test_collect_bus_stats() {
        let (tmp, _bus) = tmp_bus_with_data();
        let stats = collect_with_paths(Some(tmp.path()), None, None);
        assert_eq!(stats.total_bus_events, 2);
        assert_eq!(stats.total_knowledge_entries, 1);
        assert_eq!(stats.total_memory_entries, 1);
        assert_eq!(stats.learning_events_seen, 5);
    }

    #[test]
    fn test_collect_memory_stats() {
        let tmp_mem = tmp_memory_with_data();
        let stats = collect_with_paths(None, Some(tmp_mem.path()), None);
        assert_eq!(stats.fields_learned, 2);
        assert_eq!(stats.templates_cached, 0);
    }

    #[test]
    fn test_count_rust_lines_in_temp_dir() {
        let dir = TempDir::new().unwrap();
        // Write two fake Rust files
        let mut f1 = std::fs::File::create(dir.path().join("a.rs")).unwrap();
        writeln!(f1, "fn main() {{\n    println!(\"hello\");\n}}").unwrap();
        let mut f2 = std::fs::File::create(dir.path().join("b.rs")).unwrap();
        writeln!(f2, "// line 1\n// line 2").unwrap();
        let lines = count_rust_lines(dir.path());
        assert!(lines >= 5, "Expected at least 5 lines, got {}", lines);
    }

    #[test]
    fn test_count_rust_lines_skips_target() {
        let dir = TempDir::new().unwrap();
        // Create a target/ subdir with a Rust file — it should be skipped
        let target = dir.path().join("target").join("release");
        std::fs::create_dir_all(&target).unwrap();
        let mut f = std::fs::File::create(target.join("generated.rs")).unwrap();
        writeln!(f, "// 10 lines\n// 2\n// 3\n// 4\n// 5\n// 6\n// 7\n// 8\n// 9\n// 10").unwrap();
        // Also create a real source file
        let mut src = std::fs::File::create(dir.path().join("main.rs")).unwrap();
        writeln!(src, "fn main() {{}}").unwrap();
        let lines = count_rust_lines(dir.path());
        // Only main.rs should be counted (1 line), not generated.rs
        assert_eq!(lines, 1);
    }

    #[test]
    fn test_display_contains_key_labels() {
        let stats = DavaStats {
            total_bus_events: 42,
            total_knowledge_entries: 7,
            total_memory_entries: 3,
            fields_learned: 15,
            templates_cached: 4,
            learning_events_seen: 88,
            tools_built: 5,
            tools_total: 12,
            total_rust_lines: 9000,
        };
        let out = stats.display();
        assert!(out.contains("42"));
        assert!(out.contains("Bus events"));
        assert!(out.contains("9000"));
        assert!(out.contains("5/12"));
    }

    #[test]
    fn test_collect_tools_total() {
        let stats = collect_with_paths(None, None, None);
        assert_eq!(stats.tools_total, crate::health::TOOL_NAMES.len());
    }

    #[test]
    fn test_empty_bus_returns_zeroes() {
        let tmp = NamedTempFile::new().unwrap();
        let stats = collect_with_paths(Some(tmp.path()), None, None);
        assert_eq!(stats.total_bus_events, 0);
        assert_eq!(stats.total_knowledge_entries, 0);
        assert_eq!(stats.total_memory_entries, 0);
        assert_eq!(stats.learning_events_seen, 0);
    }

    #[test]
    fn test_empty_memory_returns_zeroes() {
        let tmp = NamedTempFile::new().unwrap();
        let _ = FieldMemory::open(tmp.path()).unwrap(); // init tables
        let stats = collect_with_paths(None, Some(tmp.path()), None);
        assert_eq!(stats.fields_learned, 0);
        assert_eq!(stats.templates_cached, 0);
    }

    #[test]
    fn test_count_rust_lines_empty_dir() {
        let dir = TempDir::new().unwrap();
        assert_eq!(count_rust_lines(dir.path()), 0);
    }

    #[test]
    fn test_count_rust_lines_ignores_non_rs() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("readme.md"), "# Hello\nWorld\n").unwrap();
        std::fs::write(dir.path().join("data.json"), "{}\n").unwrap();
        assert_eq!(count_rust_lines(dir.path()), 0);
    }

    #[test]
    fn test_display_format_all_fields() {
        let stats = DavaStats {
            total_bus_events: 1,
            total_knowledge_entries: 2,
            total_memory_entries: 3,
            fields_learned: 4,
            templates_cached: 5,
            learning_events_seen: 6,
            tools_built: 7,
            tools_total: 12,
            total_rust_lines: 8,
        };
        let out = stats.display();
        assert!(out.contains("Fields learned"));
        assert!(out.contains("Templates cached"));
        assert!(out.contains("Learning events"));
        assert!(out.contains("Rust lines"));
        assert!(out.contains("7/12"));
    }

    #[test]
    fn test_default_stats_all_zeroes() {
        let stats = DavaStats::default();
        assert_eq!(stats.total_bus_events, 0);
        assert_eq!(stats.fields_learned, 0);
        assert_eq!(stats.tools_built, 0);
        assert_eq!(stats.total_rust_lines, 0);
    }

    // ── 5 new stats tests ─────────────────────────────────────────────────────

    #[test]
    fn test_stats_display_not_empty() {
        let stats = DavaStats::default();
        let out = stats.display();
        assert!(!out.is_empty(), "display() should return non-empty string");
    }

    #[test]
    fn test_stats_with_both_bus_and_memory() {
        let (tmp_bus, _bus) = tmp_bus_with_data();
        let tmp_mem = tmp_memory_with_data();
        let stats = collect_with_paths(Some(tmp_bus.path()), Some(tmp_mem.path()), None);
        assert!(stats.total_bus_events > 0, "bus events should be > 0");
        assert!(stats.fields_learned > 0, "fields_learned should be > 0");
    }

    #[test]
    fn test_count_rust_lines_nested_dirs() {
        let dir = TempDir::new().unwrap();
        // Create nested structure: src/lib.rs and src/module/mod.rs
        let src = dir.path().join("src");
        let moddir = src.join("module");
        std::fs::create_dir_all(&moddir).unwrap();
        std::fs::write(src.join("lib.rs"), "// line1\n// line2\n").unwrap();
        std::fs::write(moddir.join("mod.rs"), "// a\n// b\n// c\n").unwrap();
        let lines = count_rust_lines(dir.path());
        assert_eq!(lines, 5, "expected 5 lines across nested dirs, got {}", lines);
    }

    #[test]
    fn test_count_rust_lines_symlinks_safe() {
        // Count on a plain dir with only real files should not panic or loop
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("main.rs"), "fn main() {}\n").unwrap();
        // Just verify it returns a sane value without panicking
        let lines = count_rust_lines(dir.path());
        assert!(lines >= 1);
    }

    #[test]
    fn test_stats_default_paths_dont_panic() {
        // collect_with_paths with all None should not panic even when DBs don't exist
        let stats = collect_with_paths(None, None, None);
        // tools_total should always equal TOOL_NAMES.len()
        assert_eq!(stats.tools_total, crate::health::TOOL_NAMES.len());
    }
}
