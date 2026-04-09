use rusqlite::{Connection, Result, params};
use chrono::Utc;
use crate::expense::Expense;

/// Open (or create) the SQLite database at the given path and ensure the
/// `expenses` table exists.
pub fn open(path: &str) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS expenses (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            amount          REAL    NOT NULL,
            vendor          TEXT    NOT NULL,
            category        TEXT    NOT NULL DEFAULT 'other',
            date            TEXT    NOT NULL,
            contract_number TEXT,
            description     TEXT,
            created_at      TEXT    NOT NULL
        );",
    )?;
    Ok(conn)
}

/// Insert a new expense and return the auto-assigned id.
pub fn insert(
    conn: &Connection,
    amount: f64,
    vendor: &str,
    category: &str,
    date: &str,
    contract_number: Option<&str>,
    description: Option<&str>,
) -> Result<i64> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO expenses (amount, vendor, category, date, contract_number, description, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![amount, vendor, category, date, contract_number, description, now],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Return all expenses, newest first.
pub fn list_all(conn: &Connection) -> Result<Vec<Expense>> {
    let mut stmt = conn.prepare(
        "SELECT id, amount, vendor, category, date, contract_number, description, created_at
         FROM expenses ORDER BY date DESC, id DESC",
    )?;
    rows_to_vec(&mut stmt, rusqlite::params![])
}

/// Return expenses whose date starts with the given YYYY-MM prefix.
pub fn list_by_month(conn: &Connection, month: &str) -> Result<Vec<Expense>> {
    let mut stmt = conn.prepare(
        "SELECT id, amount, vendor, category, date, contract_number, description, created_at
         FROM expenses WHERE date LIKE ?1 ORDER BY date DESC, id DESC",
    )?;
    let pattern = format!("{}%", month);
    rows_to_vec(&mut stmt, params![pattern])
}

/// Return expenses tied to a specific contract number (case-insensitive).
pub fn list_by_contract(conn: &Connection, contract: &str) -> Result<Vec<Expense>> {
    let mut stmt = conn.prepare(
        "SELECT id, amount, vendor, category, date, contract_number, description, created_at
         FROM expenses WHERE LOWER(contract_number) = LOWER(?1) ORDER BY date DESC, id DESC",
    )?;
    rows_to_vec(&mut stmt, params![contract])
}

/// Return expenses in the given category.
pub fn list_by_category(conn: &Connection, category: &str) -> Result<Vec<Expense>> {
    let mut stmt = conn.prepare(
        "SELECT id, amount, vendor, category, date, contract_number, description, created_at
         FROM expenses WHERE LOWER(category) = LOWER(?1) ORDER BY date DESC, id DESC",
    )?;
    rows_to_vec(&mut stmt, params![category])
}

/// (category, total) aggregates across all expenses.
pub fn sum_by_category(conn: &Connection) -> Result<Vec<(String, f64)>> {
    let mut stmt = conn.prepare(
        "SELECT category, SUM(amount) as total FROM expenses GROUP BY category ORDER BY total DESC",
    )?;
    let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?)))?;
    rows.collect()
}

/// (YYYY-MM, total) aggregates.
pub fn sum_by_month(conn: &Connection) -> Result<Vec<(String, f64)>> {
    let mut stmt = conn.prepare(
        "SELECT SUBSTR(date, 1, 7) as month, SUM(amount) as total
         FROM expenses GROUP BY month ORDER BY month DESC",
    )?;
    let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?)))?;
    rows.collect()
}

/// (contract_number, total) aggregates, excluding rows with no contract.
pub fn sum_by_contract(conn: &Connection) -> Result<Vec<(String, f64)>> {
    let mut stmt = conn.prepare(
        "SELECT contract_number, SUM(amount) as total
         FROM expenses WHERE contract_number IS NOT NULL
         GROUP BY contract_number ORDER BY total DESC",
    )?;
    let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?)))?;
    rows.collect()
}

/// Grand total of all expenses.
pub fn grand_total(conn: &Connection) -> Result<f64> {
    let total: f64 = conn.query_row(
        "SELECT COALESCE(SUM(amount), 0.0) FROM expenses",
        [],
        |row| row.get(0),
    )?;
    Ok(total)
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn rows_to_vec(
    stmt: &mut rusqlite::Statement,
    params: impl rusqlite::Params,
) -> Result<Vec<Expense>> {
    let rows = stmt.query_map(params, |row| {
        Ok(Expense {
            id: row.get(0)?,
            amount: row.get(1)?,
            vendor: row.get(2)?,
            category: row.get(3)?,
            date: row.get(4)?,
            contract_number: row.get(5)?,
            description: row.get(6)?,
            created_at: row.get(7)?,
        })
    })?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn tmp_conn() -> (NamedTempFile, Connection) {
        let f = NamedTempFile::new().unwrap();
        let conn = open(f.path().to_str().unwrap()).unwrap();
        (f, conn)
    }

    #[test]
    fn test_insert_and_list_all() {
        let (_f, conn) = tmp_conn();
        let id = insert(&conn, 45.99, "Home Depot", "supplies", "2026-04-08", Some("W9127S26QA030"), None).unwrap();
        assert!(id > 0);
        let rows = list_all(&conn).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].vendor, "Home Depot");
        assert!((rows[0].amount - 45.99).abs() < 0.001);
    }

    #[test]
    fn test_list_by_month_filter() {
        let (_f, conn) = tmp_conn();
        insert(&conn, 10.0, "A", "fuel", "2026-04-01", None, None).unwrap();
        insert(&conn, 20.0, "B", "fuel", "2026-03-15", None, None).unwrap();
        let apr = list_by_month(&conn, "2026-04").unwrap();
        assert_eq!(apr.len(), 1);
        assert_eq!(apr[0].vendor, "A");
    }

    #[test]
    fn test_list_by_contract() {
        let (_f, conn) = tmp_conn();
        insert(&conn, 100.0, "X", "labor", "2026-04-05", Some("ABC123"), None).unwrap();
        insert(&conn, 50.0, "Y", "labor", "2026-04-06", Some("ZZZ999"), None).unwrap();
        let rows = list_by_contract(&conn, "ABC123").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].vendor, "X");
    }

    #[test]
    fn test_sum_by_category() {
        let (_f, conn) = tmp_conn();
        insert(&conn, 30.0, "V1", "supplies", "2026-04-01", None, None).unwrap();
        insert(&conn, 20.0, "V2", "supplies", "2026-04-02", None, None).unwrap();
        insert(&conn, 15.0, "V3", "fuel", "2026-04-03", None, None).unwrap();
        let sums = sum_by_category(&conn).unwrap();
        let supplies = sums.iter().find(|(c, _)| c == "supplies").unwrap();
        assert!((supplies.1 - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_grand_total() {
        let (_f, conn) = tmp_conn();
        insert(&conn, 100.0, "A", "supplies", "2026-04-01", None, None).unwrap();
        insert(&conn, 50.0, "B", "fuel", "2026-04-02", None, None).unwrap();
        let total = grand_total(&conn).unwrap();
        assert!((total - 150.0).abs() < 0.001);
    }

    #[test]
    fn test_sum_by_month() {
        let (_f, conn) = tmp_conn();
        insert(&conn, 40.0, "A", "supplies", "2026-04-10", None, None).unwrap();
        insert(&conn, 60.0, "B", "fuel", "2026-04-20", None, None).unwrap();
        insert(&conn, 25.0, "C", "other", "2026-03-01", None, None).unwrap();
        let sums = sum_by_month(&conn).unwrap();
        let apr = sums.iter().find(|(m, _)| m == "2026-04").unwrap();
        assert!((apr.1 - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_sum_by_contract() {
        let (_f, conn) = tmp_conn();
        insert(&conn, 200.0, "A", "labor", "2026-04-01", Some("CON1"), None).unwrap();
        insert(&conn, 300.0, "B", "labor", "2026-04-02", Some("CON1"), None).unwrap();
        insert(&conn, 100.0, "C", "labor", "2026-04-03", None, None).unwrap();
        let sums = sum_by_contract(&conn).unwrap();
        assert_eq!(sums.len(), 1);
        assert_eq!(sums[0].0, "CON1");
        assert!((sums[0].1 - 500.0).abs() < 0.001);
    }
}
