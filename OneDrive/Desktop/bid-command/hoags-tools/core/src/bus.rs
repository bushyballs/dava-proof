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
}

#[derive(Debug)]
pub struct BusStats {
    pub total_events: i64,
    pub total_knowledge: i64,
    pub total_memory: i64,
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
}
