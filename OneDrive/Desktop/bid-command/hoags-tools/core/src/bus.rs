//! Message bus for cross-tool communication.
//! Tools publish events, connectors listen and integrate knowledge.
//!
//! Architecture:
//!   Tool → Event → Bus (SQLite) → Connector agents read + act
//!
//! Events are typed messages that flow between tools:
//!   - pdffill detects a new form template → clauseguard can pre-analyze it
//!   - invoicer generates invoice → receipts tracks the expected payment
//!   - clauseguard finds risky clause → mailcraft can draft a question to CO
//!   - sigstamp signs a doc → propbuilder knows it's ready for submission

use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusEvent {
    pub id: i64,
    pub source_tool: String,
    pub event_type: String,
    pub payload: String, // JSON
    pub created_at: String,
    pub consumed_by: String, // comma-separated tool names that processed this
}

pub struct EventBus {
    conn: Connection,
}

impl EventBus {
    pub fn open(db_path: &Path) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(db_path)?;
        let bus = Self { conn };
        bus.init()?;
        Ok(bus)
    }

    pub fn open_default() -> Result<Self, rusqlite::Error> {
        let path = default_bus_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        Self::open(&path)
    }

    fn init(&self) -> Result<(), rusqlite::Error> {
        self.conn.execute_batch("
            CREATE TABLE IF NOT EXISTS bus_events (
                id INTEGER PRIMARY KEY,
                source_tool TEXT NOT NULL,
                event_type TEXT NOT NULL,
                payload TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL,
                consumed_by TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_events_type ON bus_events(event_type);
            CREATE INDEX IF NOT EXISTS idx_events_source ON bus_events(source_tool);

            CREATE TABLE IF NOT EXISTS connector_memory (
                id INTEGER PRIMARY KEY,
                connector_name TEXT NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                UNIQUE(connector_name, key)
            );

            CREATE TABLE IF NOT EXISTS cross_knowledge (
                id INTEGER PRIMARY KEY,
                from_tool TEXT NOT NULL,
                to_tool TEXT NOT NULL,
                knowledge_type TEXT NOT NULL,
                content TEXT NOT NULL,
                confidence REAL NOT NULL DEFAULT 0.5,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_knowledge_to ON cross_knowledge(to_tool);
        ")?;
        Ok(())
    }

    /// Publish an event to the bus.
    pub fn publish(&self, source_tool: &str, event_type: &str, payload: &str) -> i64 {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO bus_events (source_tool, event_type, payload, created_at) VALUES (?,?,?,?)",
            params![source_tool, event_type, payload, now],
        ).ok();
        self.conn.last_insert_rowid()
    }

    /// Read unconsumed events for a specific tool.
    pub fn poll(&self, consumer_tool: &str, event_type: Option<&str>) -> Vec<BusEvent> {
        let query = if let Some(et) = event_type {
            format!(
                "SELECT id, source_tool, event_type, payload, created_at, consumed_by FROM bus_events WHERE event_type = '{}' AND consumed_by NOT LIKE '%{}%' ORDER BY id",
                et, consumer_tool
            )
        } else {
            format!(
                "SELECT id, source_tool, event_type, payload, created_at, consumed_by FROM bus_events WHERE consumed_by NOT LIKE '%{}%' ORDER BY id",
                consumer_tool
            )
        };

        let mut stmt = match self.conn.prepare(&query) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        stmt.query_map([], |row| {
            Ok(BusEvent {
                id: row.get(0)?,
                source_tool: row.get(1)?,
                event_type: row.get(2)?,
                payload: row.get(3)?,
                created_at: row.get(4)?,
                consumed_by: row.get(5)?,
            })
        }).ok()
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
    }

    /// Mark an event as consumed by a tool.
    pub fn ack(&self, event_id: i64, consumer_tool: &str) {
        // Append consumer to the consumed_by list
        let current: String = self.conn.query_row(
            "SELECT consumed_by FROM bus_events WHERE id = ?",
            params![event_id],
            |row| row.get(0),
        ).unwrap_or_default();

        let updated = if current.is_empty() {
            consumer_tool.to_string()
        } else {
            format!("{},{}", current, consumer_tool)
        };

        self.conn.execute(
            "UPDATE bus_events SET consumed_by = ? WHERE id = ?",
            params![updated, event_id],
        ).ok();
    }

    /// Store connector-specific memory (key-value).
    pub fn set_memory(&self, connector: &str, key: &str, value: &str) {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT OR REPLACE INTO connector_memory (connector_name, key, value, updated_at) VALUES (?,?,?,?)",
            params![connector, key, value, now],
        ).ok();
    }

    /// Read connector memory.
    pub fn get_memory(&self, connector: &str, key: &str) -> Option<String> {
        self.conn.query_row(
            "SELECT value FROM connector_memory WHERE connector_name = ? AND key = ?",
            params![connector, key],
            |row| row.get(0),
        ).ok()
    }

    /// Share knowledge between tools.
    pub fn share_knowledge(&self, from: &str, to: &str, knowledge_type: &str, content: &str, confidence: f64) {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO cross_knowledge (from_tool, to_tool, knowledge_type, content, confidence, created_at) VALUES (?,?,?,?,?,?)",
            params![from, to, knowledge_type, content, confidence, now],
        ).ok();
    }

    /// Get knowledge shared TO a specific tool.
    pub fn get_knowledge(&self, to_tool: &str) -> Vec<(String, String, String, f64)> {
        let mut stmt = self.conn.prepare(
            "SELECT from_tool, knowledge_type, content, confidence FROM cross_knowledge WHERE to_tool = ? ORDER BY confidence DESC"
        ).unwrap();

        stmt.query_map(params![to_tool], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        }).ok()
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
    }

    /// Get bus stats.
    pub fn stats(&self) -> BusStats {
        let total_events: i64 = self.conn.query_row("SELECT COUNT(*) FROM bus_events", [], |r| r.get(0)).unwrap_or(0);
        let total_knowledge: i64 = self.conn.query_row("SELECT COUNT(*) FROM cross_knowledge", [], |r| r.get(0)).unwrap_or(0);
        let total_memory: i64 = self.conn.query_row("SELECT COUNT(*) FROM connector_memory", [], |r| r.get(0)).unwrap_or(0);
        BusStats { total_events, total_knowledge, total_memory }
    }

    /// Delete all events older than `days` days. Returns the number of rows deleted.
    pub fn purge_old(&self, days: i64) -> usize {
        let cutoff = Utc::now()
            .checked_sub_signed(chrono::Duration::days(days))
            .unwrap_or_else(Utc::now)
            .to_rfc3339();
        self.conn
            .execute("DELETE FROM bus_events WHERE created_at < ?", params![cutoff])
            .unwrap_or(0) as usize
    }

    /// Return a count of events grouped by source_tool.
    pub fn event_count_by_tool(&self) -> std::collections::HashMap<String, usize> {
        let mut stmt = match self.conn.prepare(
            "SELECT source_tool, COUNT(*) FROM bus_events GROUP BY source_tool",
        ) {
            Ok(s) => s,
            Err(_) => return std::collections::HashMap::new(),
        };
        stmt.query_map([], |row| {
            let tool: String = row.get(0)?;
            let count: i64 = row.get(1)?;
            Ok((tool, count as usize))
        })
        .ok()
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
    }

    /// Count unconsumed (pending) events for a specific consumer tool.
    pub fn pending_for(&self, tool: &str) -> usize {
        // events where tool has NOT yet consumed them (consumed_by doesn't contain tool)
        let query = format!(
            "SELECT COUNT(*) FROM bus_events WHERE consumed_by NOT LIKE '%{}%'",
            tool
        );
        self.conn
            .query_row(&query, [], |r| r.get::<_, i64>(0))
            .unwrap_or(0) as usize
    }

    /// Dump all events, cross_knowledge, and connector_memory as a JSON string.
    pub fn export_json(&self) -> String {
        let events: Vec<BusEvent> = {
            let mut stmt = self.conn.prepare(
                "SELECT id, source_tool, event_type, payload, created_at, consumed_by FROM bus_events ORDER BY id",
            ).unwrap();
            stmt.query_map([], |row| {
                Ok(BusEvent {
                    id: row.get(0)?,
                    source_tool: row.get(1)?,
                    event_type: row.get(2)?,
                    payload: row.get(3)?,
                    created_at: row.get(4)?,
                    consumed_by: row.get(5)?,
                })
            })
            .ok()
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
            .unwrap_or_default()
        };

        let knowledge: Vec<serde_json::Value> = {
            let mut stmt = self.conn.prepare(
                "SELECT from_tool, to_tool, knowledge_type, content, confidence, created_at FROM cross_knowledge ORDER BY id",
            ).unwrap();
            stmt.query_map([], |row| {
                let from_tool: String = row.get(0)?;
                let to_tool: String = row.get(1)?;
                let knowledge_type: String = row.get(2)?;
                let content: String = row.get(3)?;
                let confidence: f64 = row.get(4)?;
                let created_at: String = row.get(5)?;
                Ok(serde_json::json!({
                    "from_tool": from_tool,
                    "to_tool": to_tool,
                    "knowledge_type": knowledge_type,
                    "content": content,
                    "confidence": confidence,
                    "created_at": created_at,
                }))
            })
            .ok()
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
            .unwrap_or_default()
        };

        let memory: Vec<serde_json::Value> = {
            let mut stmt = self.conn.prepare(
                "SELECT connector_name, key, value, updated_at FROM connector_memory ORDER BY id",
            ).unwrap();
            stmt.query_map([], |row| {
                let connector_name: String = row.get(0)?;
                let key: String = row.get(1)?;
                let value: String = row.get(2)?;
                let updated_at: String = row.get(3)?;
                Ok(serde_json::json!({
                    "connector": connector_name,
                    "key": key,
                    "value": value,
                    "updated_at": updated_at,
                }))
            })
            .ok()
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
            .unwrap_or_default()
        };

        let root = serde_json::json!({
            "events": events,
            "knowledge": knowledge,
            "memory": memory,
        });
        serde_json::to_string_pretty(&root).unwrap_or_else(|_| "{}".to_string())
    }
}

#[derive(Debug)]
pub struct BusStats {
    pub total_events: i64,
    pub total_knowledge: i64,
    pub total_memory: i64,
}

// ─── BusRunner ───────────────────────────────────────────────────────────────

use crate::connectors::run_all_connectors;

/// Runs all 5 connectors against the bus in a loop with a configurable interval.
/// Call `BusRunner::new(bus).run_loop()` to start. The loop runs forever until
/// the process is killed, which is fine for background daemon use.
pub struct BusRunner {
    pub bus: EventBus,
    /// How long to sleep between connector passes.
    pub interval: std::time::Duration,
}

impl BusRunner {
    pub fn new(bus: EventBus) -> Self {
        Self {
            bus,
            interval: std::time::Duration::from_secs(30),
        }
    }

    pub fn with_interval(mut self, interval: std::time::Duration) -> Self {
        self.interval = interval;
        self
    }

    /// Run one pass of all connectors immediately, returning the results.
    pub fn run_once(&self) -> Vec<(String, crate::connectors::ConnectorResult)> {
        run_all_connectors(&self.bus)
    }

    /// Block and run connectors in a loop. Each iteration sleeps `self.interval`.
    /// In tests prefer `run_once`; use this for long-running daemons.
    pub fn run_loop(&self) -> ! {
        loop {
            let results = self.run_once();
            let total_processed: usize = results.iter().map(|(_, r)| r.events_processed).sum();
            if total_processed > 0 {
                eprintln!(
                    "[BusRunner] pass complete — {} events processed across {} connectors",
                    total_processed,
                    results.len()
                );
            }
            std::thread::sleep(self.interval);
        }
    }
}

fn default_bus_path() -> PathBuf {
    let mut path = std::env::current_dir().unwrap_or_default();
    path.push("data");
    path.push("hoags_bus.db");
    path
}

// ── Event Types ─────────────────────────────────────────────────────
// Standard event types that tools publish:

pub mod events {
    // pdffill
    pub const FORM_DETECTED: &str = "pdffill.form_detected";
    pub const FORM_FILLED: &str = "pdffill.form_filled";
    pub const TEMPLATE_LEARNED: &str = "pdffill.template_learned";

    // invoicer
    pub const INVOICE_GENERATED: &str = "invoicer.invoice_generated";
    pub const INVOICE_SUBMITTED: &str = "invoicer.invoice_submitted";
    pub const PAYMENT_RECEIVED: &str = "invoicer.payment_received";

    // clauseguard
    pub const RISK_FOUND: &str = "clauseguard.risk_found";
    pub const CLAUSE_ANALYZED: &str = "clauseguard.clause_analyzed";

    // sigstamp
    pub const DOCUMENT_SIGNED: &str = "sigstamp.document_signed";

    // propbuilder
    pub const PROPOSAL_GENERATED: &str = "propbuilder.proposal_generated";

    // mailcraft
    pub const EMAIL_DRAFTED: &str = "mailcraft.email_drafted";

    // receipts
    pub const EXPENSE_ADDED: &str = "receipts.expense_added";

    // docconv
    pub const DOCUMENT_CONVERTED: &str = "docconv.document_converted";

    // sheetwise
    pub const DATA_ANALYZED: &str = "sheetwise.data_analyzed";

    // actionminer
    pub const ACTION_EXTRACTED: &str = "actionminer.action_extracted";
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn tmp_bus() -> EventBus {
        let tmp = NamedTempFile::new().unwrap();
        EventBus::open(tmp.path()).unwrap()
    }

    #[test]
    fn test_publish_and_poll() {
        let bus = tmp_bus();
        bus.publish("pdffill", "pdffill.form_detected", r#"{"template":"SF1449"}"#);
        let events = bus.poll("clauseguard", None);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].source_tool, "pdffill");
    }

    #[test]
    fn test_ack_prevents_repoll() {
        let bus = tmp_bus();
        let id = bus.publish("pdffill", "pdffill.form_filled", "{}");
        bus.ack(id, "invoicer");
        let events = bus.poll("invoicer", None);
        assert!(events.is_empty());
    }

    #[test]
    fn test_connector_memory() {
        let bus = tmp_bus();
        bus.set_memory("connector_bid", "last_scan", "2026-04-08");
        assert_eq!(bus.get_memory("connector_bid", "last_scan"), Some("2026-04-08".into()));
        assert_eq!(bus.get_memory("connector_bid", "missing"), None);
    }

    #[test]
    fn test_cross_knowledge() {
        let bus = tmp_bus();
        bus.share_knowledge("clauseguard", "mailcraft", "risky_clause", "52.249-8 Default termination", 0.9);
        let knowledge = bus.get_knowledge("mailcraft");
        assert_eq!(knowledge.len(), 1);
        assert_eq!(knowledge[0].2, "52.249-8 Default termination");
    }

    #[test]
    fn test_stats() {
        let bus = tmp_bus();
        bus.publish("pdffill", "test", "{}");
        bus.publish("invoicer", "test", "{}");
        bus.set_memory("conn1", "k", "v");
        bus.share_knowledge("a", "b", "t", "c", 0.5);
        let stats = bus.stats();
        assert_eq!(stats.total_events, 2);
        assert_eq!(stats.total_memory, 1);
        assert_eq!(stats.total_knowledge, 1);
    }

    #[test]
    fn test_purge_old_keeps_recent_events() {
        let bus = tmp_bus();
        bus.publish("pdffill", "pdffill.form_detected", "{}");
        bus.publish("invoicer", "invoicer.invoice_generated", "{}");
        // Purge events older than 30 days — none should be deleted since both are fresh
        let deleted = bus.purge_old(30);
        assert_eq!(deleted, 0);
        let stats = bus.stats();
        assert_eq!(stats.total_events, 2);
    }

    #[test]
    fn test_purge_old_negative_days_removes_all() {
        let bus = tmp_bus();
        bus.publish("pdffill", "pdffill.form_detected", "{}");
        bus.publish("invoicer", "invoicer.invoice_generated", "{}");
        // days = -1 means cutoff is in the future → all events are "old"
        let deleted = bus.purge_old(-1);
        assert_eq!(deleted, 2);
        let stats = bus.stats();
        assert_eq!(stats.total_events, 0);
    }

    #[test]
    fn test_event_count_by_tool() {
        let bus = tmp_bus();
        bus.publish("pdffill", "pdffill.form_detected", "{}");
        bus.publish("pdffill", "pdffill.form_filled", "{}");
        bus.publish("invoicer", "invoicer.invoice_generated", "{}");
        let counts = bus.event_count_by_tool();
        assert_eq!(counts.get("pdffill"), Some(&2));
        assert_eq!(counts.get("invoicer"), Some(&1));
        assert!(counts.get("clauseguard").is_none());
    }

    #[test]
    fn test_pending_for_tool() {
        let bus = tmp_bus();
        bus.publish("pdffill", "pdffill.form_detected", "{}");
        bus.publish("pdffill", "pdffill.form_filled", "{}");
        // Ack one event for clauseguard
        let events = bus.poll("clauseguard", None);
        bus.ack(events[0].id, "clauseguard");
        // clauseguard has consumed 1 of 2, but pending_for counts events NOT yet consumed
        assert_eq!(bus.pending_for("clauseguard"), 1);
        // A fresh tool hasn't consumed any
        assert_eq!(bus.pending_for("invoicer"), 2);
    }

    #[test]
    fn test_export_json_structure() {
        let bus = tmp_bus();
        bus.publish("pdffill", "pdffill.form_detected", r#"{"template":"SF1449"}"#);
        bus.set_memory("connector_bid", "last_scan", "2026-04-08");
        bus.share_knowledge("clauseguard", "mailcraft", "risky_clause", "52.249-8", 0.9);
        let json_str = bus.export_json();
        let val: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert!(val["events"].is_array());
        assert_eq!(val["events"].as_array().unwrap().len(), 1);
        assert!(val["knowledge"].is_array());
        assert_eq!(val["knowledge"].as_array().unwrap().len(), 1);
        assert!(val["memory"].is_array());
        assert_eq!(val["memory"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_bus_runner_run_once() {
        let tmp = NamedTempFile::new().unwrap();
        let bus = EventBus::open(tmp.path()).unwrap();
        bus.publish("pdffill", "pdffill.form_detected", r#"{"template":"W9"}"#);
        let runner = BusRunner::new(bus);
        let results = runner.run_once();
        assert_eq!(results.len(), 5);
        let total: usize = results.iter().map(|(_, r)| r.events_processed).sum();
        assert!(total >= 2);
    }
}
