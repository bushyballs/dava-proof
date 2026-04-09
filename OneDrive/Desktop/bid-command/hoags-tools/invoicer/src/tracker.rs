//! tracker.rs — SQLite-backed invoice tracking for invoicer.
//!
//! Tables:
//!   invoices      — one row per invoice
//!   invoice_lines — one row per CLIN on an invoice

use rusqlite::{Connection, Result, params};
use serde::{Deserialize, Serialize};

// ── Status enum ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InvoiceStatus {
    Draft,
    Submitted,
    Paid,
}

impl InvoiceStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            InvoiceStatus::Draft => "draft",
            InvoiceStatus::Submitted => "submitted",
            InvoiceStatus::Paid => "paid",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "submitted" => InvoiceStatus::Submitted,
            "paid" => InvoiceStatus::Paid,
            _ => InvoiceStatus::Draft,
        }
    }
}

impl std::fmt::Display for InvoiceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ── Row types ────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct InvoiceRow {
    pub id: i64,
    pub contract_number: String,
    pub invoice_number: String,
    pub period: String,
    pub total_amount: f64,
    pub status: InvoiceStatus,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InvoiceLine {
    pub id: i64,
    pub invoice_id: i64,
    pub clin: String,
    pub description: String,
    pub qty: f64,
    pub unit_price: f64,
    pub amount: f64,
}

// ── Tracker ──────────────────────────────────────────────────────────────────

pub struct Tracker {
    conn: Connection,
}

impl Tracker {
    /// Open (or create) a tracker database at `db_path`.
    pub fn open(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        let tracker = Tracker { conn };
        tracker.init()?;
        Ok(tracker)
    }

    /// Create an in-memory tracker (used in tests).
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let tracker = Tracker { conn };
        tracker.init()?;
        Ok(tracker)
    }

    fn init(&self) -> Result<()> {
        self.conn.execute_batch("
            CREATE TABLE IF NOT EXISTS invoices (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                contract_number TEXT NOT NULL,
                invoice_number  TEXT NOT NULL UNIQUE,
                period          TEXT NOT NULL,
                total_amount    REAL NOT NULL,
                status          TEXT NOT NULL DEFAULT 'draft',
                created_at      TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS invoice_lines (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                invoice_id  INTEGER NOT NULL REFERENCES invoices(id),
                clin        TEXT NOT NULL,
                description TEXT NOT NULL,
                qty         REAL NOT NULL,
                unit_price  REAL NOT NULL,
                amount      REAL NOT NULL
            );
        ")
    }

    /// Next sequence number for a contract+period, used to build invoice_number.
    /// `period` is "YYYY-MM" (e.g. "2026-04").
    pub fn next_sequence(&self, contract_number: &str, period: &str) -> Result<u32> {
        // Match any invoice whose period starts with "2026-04" (the YYYY-MM prefix)
        let count: u32 = self.conn.query_row(
            "SELECT COUNT(*) FROM invoices WHERE contract_number = ?1 AND period LIKE ?2",
            params![contract_number, format!("{}%", period)],
            |row| row.get(0),
        )?;
        Ok(count + 1)
    }

    /// Insert a new invoice header + lines. Returns the assigned row id.
    pub fn insert_invoice(
        &self,
        contract_number: &str,
        invoice_number: &str,
        period: &str,
        total_amount: f64,
        lines: &[(String, String, f64, f64, f64)], // (clin, desc, qty, unit_price, amount)
    ) -> Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO invoices (contract_number, invoice_number, period, total_amount, status, created_at)
             VALUES (?1, ?2, ?3, ?4, 'draft', ?5)",
            params![contract_number, invoice_number, period, total_amount, now],
        )?;
        let invoice_id = self.conn.last_insert_rowid();

        for (clin, desc, qty, unit_price, amount) in lines {
            self.conn.execute(
                "INSERT INTO invoice_lines (invoice_id, clin, description, qty, unit_price, amount)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![invoice_id, clin, desc, qty, unit_price, amount],
            )?;
        }

        Ok(invoice_id)
    }

    /// Update status of an invoice by invoice_number.
    pub fn update_status(&self, invoice_number: &str, status: InvoiceStatus) -> Result<usize> {
        self.conn.execute(
            "UPDATE invoices SET status = ?1 WHERE invoice_number = ?2",
            params![status.as_str(), invoice_number],
        )
    }

    /// List all invoices, newest first.
    pub fn list_all(&self) -> Result<Vec<InvoiceRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, contract_number, invoice_number, period, total_amount, status, created_at
             FROM invoices ORDER BY created_at DESC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(InvoiceRow {
                id: row.get(0)?,
                contract_number: row.get(1)?,
                invoice_number: row.get(2)?,
                period: row.get(3)?,
                total_amount: row.get(4)?,
                status: InvoiceStatus::from_str(&row.get::<_, String>(5)?),
                created_at: row.get(6)?,
            })
        })?;
        rows.collect()
    }

    /// All invoices for a specific contract.
    pub fn list_for_contract(&self, contract_number: &str) -> Result<Vec<InvoiceRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, contract_number, invoice_number, period, total_amount, status, created_at
             FROM invoices WHERE contract_number = ?1 ORDER BY created_at DESC"
        )?;
        let rows = stmt.query_map(params![contract_number], |row| {
            Ok(InvoiceRow {
                id: row.get(0)?,
                contract_number: row.get(1)?,
                invoice_number: row.get(2)?,
                period: row.get(3)?,
                total_amount: row.get(4)?,
                status: InvoiceStatus::from_str(&row.get::<_, String>(5)?),
                created_at: row.get(6)?,
            })
        })?;
        rows.collect()
    }

    /// Sum of all invoice totals for a contract (submitted + paid).
    pub fn total_invoiced(&self, contract_number: &str) -> Result<f64> {
        let total: f64 = self.conn.query_row(
            "SELECT COALESCE(SUM(total_amount), 0.0) FROM invoices
             WHERE contract_number = ?1 AND status IN ('submitted', 'paid')",
            params![contract_number],
            |row| row.get(0),
        )?;
        Ok(total)
    }

    /// Total amount already invoiced for each CLIN of a contract (submitted + paid).
    /// Returns a map of clin_number → total_invoiced.
    pub fn total_invoiced_per_clin(
        &self,
        contract_number: &str,
    ) -> Result<std::collections::HashMap<String, f64>> {
        let mut stmt = self.conn.prepare(
            "SELECT il.clin, COALESCE(SUM(il.amount), 0.0)
             FROM invoice_lines il
             JOIN invoices i ON i.id = il.invoice_id
             WHERE i.contract_number = ?1
               AND i.status IN ('submitted', 'paid')
             GROUP BY il.clin"
        )?;
        let rows = stmt.query_map(params![contract_number], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
        })?;
        let mut map = std::collections::HashMap::new();
        for row in rows {
            let (clin, amount) = row?;
            map.insert(clin, amount);
        }
        Ok(map)
    }

    /// Lines for a specific invoice.
    pub fn lines_for_invoice(&self, invoice_id: i64) -> Result<Vec<InvoiceLine>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, invoice_id, clin, description, qty, unit_price, amount
             FROM invoice_lines WHERE invoice_id = ?1"
        )?;
        let rows = stmt.query_map(params![invoice_id], |row| {
            Ok(InvoiceLine {
                id: row.get(0)?,
                invoice_id: row.get(1)?,
                clin: row.get(2)?,
                description: row.get(3)?,
                qty: row.get(4)?,
                unit_price: row.get(5)?,
                amount: row.get(6)?,
            })
        })?;
        rows.collect()
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tracker() -> Tracker {
        Tracker::open_in_memory().expect("in-memory tracker")
    }

    #[test]
    fn test_init_creates_tables() {
        let t = make_tracker();
        let rows = t.list_all().expect("list_all");
        assert!(rows.is_empty());
    }

    #[test]
    fn test_insert_and_list() {
        let t = make_tracker();
        let lines = vec![
            ("0001".to_string(), "Daily Service".to_string(), 30.0, 91.19, 2735.70),
        ];
        let id = t.insert_invoice("W9127S26QA030", "HOAGS-INV-202604-001", "2026-04", 2735.70, &lines)
            .expect("insert");
        assert_eq!(id, 1);

        let all = t.list_all().expect("list");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].invoice_number, "HOAGS-INV-202604-001");
        assert_eq!(all[0].status, InvoiceStatus::Draft);
    }

    #[test]
    fn test_update_status() {
        let t = make_tracker();
        let lines: Vec<(String, String, f64, f64, f64)> = vec![];
        t.insert_invoice("W9127S26QA030", "HOAGS-INV-202604-002", "2026-04", 0.0, &lines)
            .expect("insert");
        t.update_status("HOAGS-INV-202604-002", InvoiceStatus::Submitted)
            .expect("update");

        let all = t.list_for_contract("W9127S26QA030").expect("list");
        assert_eq!(all[0].status, InvoiceStatus::Submitted);
    }

    #[test]
    fn test_total_invoiced_only_counts_submitted_paid() {
        let t = make_tracker();
        let empty: Vec<(String, String, f64, f64, f64)> = vec![];
        t.insert_invoice("CONTRACT-1", "HOAGS-INV-202604-010", "2026-04", 100.0, &empty).unwrap();
        t.insert_invoice("CONTRACT-1", "HOAGS-INV-202604-011", "2026-04", 200.0, &empty).unwrap();
        t.update_status("HOAGS-INV-202604-010", InvoiceStatus::Submitted).unwrap();
        t.update_status("HOAGS-INV-202604-011", InvoiceStatus::Paid).unwrap();
        t.insert_invoice("CONTRACT-1", "HOAGS-INV-202604-012", "2026-04", 50.0, &empty).unwrap();
        // draft is not counted
        let total = t.total_invoiced("CONTRACT-1").unwrap();
        assert!((total - 300.0).abs() < 0.01);
    }

    #[test]
    fn test_next_sequence() {
        let t = make_tracker();
        let seq = t.next_sequence("W9127S26QA030", "2026-04").unwrap();
        assert_eq!(seq, 1);
        let empty: Vec<(String, String, f64, f64, f64)> = vec![];
        t.insert_invoice("W9127S26QA030", "HOAGS-INV-202604-001", "2026-04", 0.0, &empty).unwrap();
        let seq2 = t.next_sequence("W9127S26QA030", "2026-04").unwrap();
        assert_eq!(seq2, 2);
    }

    #[test]
    fn test_lines_stored_and_retrieved() {
        let t = make_tracker();
        let lines = vec![
            ("0001".to_string(), "Daily".to_string(), 10.0, 91.19, 911.90),
            ("0002".to_string(), "Semi-Annual".to_string(), 1.0, 307.61, 307.61),
        ];
        let inv_id = t.insert_invoice("CTR-1", "HOAGS-INV-202604-020", "2026-04", 1219.51, &lines).unwrap();
        let fetched = t.lines_for_invoice(inv_id).unwrap();
        assert_eq!(fetched.len(), 2);
        assert_eq!(fetched[0].clin, "0001");
        assert!((fetched[1].unit_price - 307.61).abs() < 0.001);
    }
}
