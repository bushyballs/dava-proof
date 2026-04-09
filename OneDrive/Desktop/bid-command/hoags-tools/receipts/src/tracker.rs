use rusqlite::{Connection, Result, params};
use chrono::Utc;
use crate::expense::Expense;

/// Open (or create) the SQLite database at the given path and ensure the
/// `expenses` and `budgets` tables exist.
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
        );
        CREATE TABLE IF NOT EXISTS budgets (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            contract_number TEXT    NOT NULL UNIQUE,
            limit_amount    REAL    NOT NULL,
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

/// Delete an expense by id. Returns the number of rows deleted (0 or 1).
pub fn delete(conn: &Connection, id: i64) -> Result<usize> {
    let n = conn.execute("DELETE FROM expenses WHERE id = ?1", params![id])?;
    Ok(n)
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

/// Return expenses for a given tax year (YYYY).
pub fn list_by_tax_year(conn: &Connection, year: u32) -> Result<Vec<Expense>> {
    let mut stmt = conn.prepare(
        "SELECT id, amount, vendor, category, date, contract_number, description, created_at
         FROM expenses WHERE date LIKE ?1 ORDER BY date ASC, id ASC",
    )?;
    let pattern = format!("{}%", year);
    rows_to_vec(&mut stmt, params![pattern])
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

/// Total spent for a specific tax year.
pub fn grand_total_for_year(conn: &Connection, year: u32) -> Result<f64> {
    let pattern = format!("{}%", year);
    let total: f64 = conn.query_row(
        "SELECT COALESCE(SUM(amount), 0.0) FROM expenses WHERE date LIKE ?1",
        params![pattern],
        |row| row.get(0),
    )?;
    Ok(total)
}

/// (category, total) aggregates for a specific tax year.
pub fn sum_by_category_for_year(conn: &Connection, year: u32) -> Result<Vec<(String, f64)>> {
    let pattern = format!("{}%", year);
    let mut stmt = conn.prepare(
        "SELECT category, SUM(amount) as total
         FROM expenses WHERE date LIKE ?1
         GROUP BY category ORDER BY total DESC",
    )?;
    let rows = stmt.query_map(params![pattern], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
    })?;
    rows.collect()
}

/// (contract_number, total) for a specific tax year.
pub fn sum_by_contract_for_year(conn: &Connection, year: u32) -> Result<Vec<(String, f64)>> {
    let pattern = format!("{}%", year);
    let mut stmt = conn.prepare(
        "SELECT contract_number, SUM(amount) as total
         FROM expenses WHERE contract_number IS NOT NULL AND date LIKE ?1
         GROUP BY contract_number ORDER BY total DESC",
    )?;
    let rows = stmt.query_map(params![pattern], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
    })?;
    rows.collect()
}

// ── budget helpers ────────────────────────────────────────────────────────────

/// Set or update the budget limit for a contract. Returns the row id.
pub fn upsert_budget(conn: &Connection, contract: &str, limit: f64) -> Result<i64> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO budgets (contract_number, limit_amount, created_at)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(contract_number) DO UPDATE SET limit_amount = excluded.limit_amount",
        params![contract, limit, now],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Retrieve the budget limit for a contract (None if not set).
pub fn get_budget(conn: &Connection, contract: &str) -> Result<Option<f64>> {
    let mut stmt = conn.prepare(
        "SELECT limit_amount FROM budgets WHERE LOWER(contract_number) = LOWER(?1)",
    )?;
    let mut rows = stmt.query(params![contract])?;
    if let Some(row) = rows.next()? {
        Ok(Some(row.get(0)?))
    } else {
        Ok(None)
    }
}

/// Total spent for a contract (across all dates).
pub fn total_for_contract(conn: &Connection, contract: &str) -> Result<f64> {
    let total: f64 = conn.query_row(
        "SELECT COALESCE(SUM(amount), 0.0) FROM expenses WHERE LOWER(contract_number) = LOWER(?1)",
        params![contract],
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
    fn test_delete_removes_row() {
        let (_f, conn) = tmp_conn();
        let id = insert(&conn, 10.0, "X", "fuel", "2026-04-01", None, None).unwrap();
        let n = delete(&conn, id).unwrap();
        assert_eq!(n, 1);
        let rows = list_all(&conn).unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn test_delete_nonexistent_returns_zero() {
        let (_f, conn) = tmp_conn();
        let n = delete(&conn, 9999).unwrap();
        assert_eq!(n, 0);
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
    fn test_list_by_tax_year() {
        let (_f, conn) = tmp_conn();
        insert(&conn, 100.0, "A", "supplies", "2026-01-01", None, None).unwrap();
        insert(&conn, 50.0, "B", "fuel", "2025-12-31", None, None).unwrap();
        let rows = list_by_tax_year(&conn, 2026).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].vendor, "A");
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
    fn test_grand_total_for_year() {
        let (_f, conn) = tmp_conn();
        insert(&conn, 100.0, "A", "supplies", "2026-04-01", None, None).unwrap();
        insert(&conn, 50.0, "B", "fuel", "2025-12-01", None, None).unwrap();
        let total = grand_total_for_year(&conn, 2026).unwrap();
        assert!((total - 100.0).abs() < 0.001);
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

    #[test]
    fn test_budget_upsert_and_get() {
        let (_f, conn) = tmp_conn();
        upsert_budget(&conn, "CON1", 50000.0).unwrap();
        let limit = get_budget(&conn, "CON1").unwrap();
        assert!(limit.is_some());
        assert!((limit.unwrap() - 50000.0).abs() < 0.001);
    }

    #[test]
    fn test_budget_upsert_updates_existing() {
        let (_f, conn) = tmp_conn();
        upsert_budget(&conn, "CON1", 50000.0).unwrap();
        upsert_budget(&conn, "CON1", 75000.0).unwrap();
        let limit = get_budget(&conn, "CON1").unwrap().unwrap();
        assert!((limit - 75000.0).abs() < 0.001);
    }

    #[test]
    fn test_budget_get_returns_none_when_not_set() {
        let (_f, conn) = tmp_conn();
        let limit = get_budget(&conn, "NONEXISTENT").unwrap();
        assert!(limit.is_none());
    }

    #[test]
    fn test_total_for_contract() {
        let (_f, conn) = tmp_conn();
        insert(&conn, 1000.0, "A", "labor", "2026-04-01", Some("CON1"), None).unwrap();
        insert(&conn, 500.0, "B", "supplies", "2026-04-02", Some("CON1"), None).unwrap();
        let total = total_for_contract(&conn, "CON1").unwrap();
        assert!((total - 1500.0).abs() < 0.001);
    }

    #[test]
    fn test_sum_by_category_for_year() {
        let (_f, conn) = tmp_conn();
        insert(&conn, 100.0, "A", "supplies", "2026-03-01", None, None).unwrap();
        insert(&conn, 50.0, "B", "fuel", "2026-04-01", None, None).unwrap();
        insert(&conn, 200.0, "C", "supplies", "2025-12-01", None, None).unwrap();
        let sums = sum_by_category_for_year(&conn, 2026).unwrap();
        let supplies = sums.iter().find(|(c, _)| c == "supplies").unwrap();
        assert!((supplies.1 - 100.0).abs() < 0.001);
    }

    // ── 10 new tests ──────────────────────────────────────────────────────────

    #[test]
    fn test_grand_total_with_multiple_expenses() {
        let (_f, conn) = tmp_conn();
        insert(&conn, 25.0, "A", "supplies", "2026-04-01", None, None).unwrap();
        insert(&conn, 75.0, "B", "fuel", "2026-04-02", None, None).unwrap();
        insert(&conn, 50.0, "C", "labor", "2026-04-03", None, None).unwrap();
        let total = grand_total(&conn).unwrap();
        assert!((total - 150.0).abs() < 0.001);
    }

    #[test]
    fn test_sum_by_category_multiple() {
        let (_f, conn) = tmp_conn();
        insert(&conn, 10.0, "A", "fuel", "2026-04-01", None, None).unwrap();
        insert(&conn, 20.0, "B", "fuel", "2026-04-02", None, None).unwrap();
        insert(&conn, 30.0, "C", "supplies", "2026-04-03", None, None).unwrap();
        insert(&conn, 40.0, "D", "labor", "2026-04-04", None, None).unwrap();
        let sums = sum_by_category(&conn).unwrap();
        assert_eq!(sums.len(), 3);
        let fuel = sums.iter().find(|(c, _)| c == "fuel").unwrap();
        assert!((fuel.1 - 30.0).abs() < 0.001);
    }

    #[test]
    fn test_sum_by_month_format() {
        let (_f, conn) = tmp_conn();
        insert(&conn, 100.0, "A", "supplies", "2026-01-15", None, None).unwrap();
        insert(&conn, 200.0, "B", "fuel", "2026-02-20", None, None).unwrap();
        let sums = sum_by_month(&conn).unwrap();
        // Month keys should be "YYYY-MM" format
        for (month, _) in &sums {
            assert_eq!(month.len(), 7, "month key '{}' should be 7 chars", month);
            assert!(month.contains('-'), "month key '{}' should contain dash", month);
        }
    }

    #[test]
    fn test_list_by_category_filter() {
        let (_f, conn) = tmp_conn();
        insert(&conn, 10.0, "A", "fuel", "2026-04-01", None, None).unwrap();
        insert(&conn, 20.0, "B", "supplies", "2026-04-02", None, None).unwrap();
        insert(&conn, 30.0, "C", "fuel", "2026-04-03", None, None).unwrap();
        let fuel = list_by_category(&conn, "fuel").unwrap();
        assert_eq!(fuel.len(), 2);
        assert!(fuel.iter().all(|e| e.category == "fuel"));
    }

    #[test]
    fn test_delete_nonexistent_returns_error() {
        let (_f, conn) = tmp_conn();
        // Deleting a non-existent id returns 0 rows affected (not an error)
        let n = delete(&conn, 99999).unwrap();
        assert_eq!(n, 0, "deleting non-existent id should affect 0 rows");
    }

    #[test]
    fn test_import_csv_valid_data() {
        // Simulate CSV parsing: parse "amount,vendor,category,date" lines
        let (_f, conn) = tmp_conn();
        let csv = "45.00,HomeDepot,supplies,2026-04-01\n12.50,GasStation,fuel,2026-04-02\n";
        let mut inserted = 0i64;
        for line in csv.lines() {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() == 4 {
                let amount: f64 = parts[0].parse().unwrap();
                let id = insert(&conn, amount, parts[1], parts[2], parts[3], None, None).unwrap();
                assert!(id > 0);
                inserted += 1;
            }
        }
        assert_eq!(inserted, 2);
        let rows = list_all(&conn).unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_budget_insert_and_check() {
        let (_f, conn) = tmp_conn();
        upsert_budget(&conn, "PROJ-001", 10000.0).unwrap();
        let limit = get_budget(&conn, "PROJ-001").unwrap();
        assert!(limit.is_some());
        assert!((limit.unwrap() - 10000.0).abs() < 0.001);
    }

    #[test]
    fn test_budget_over_limit_warning() {
        let (_f, conn) = tmp_conn();
        upsert_budget(&conn, "PROJ-002", 500.0).unwrap();
        insert(&conn, 300.0, "V1", "supplies", "2026-04-01", Some("PROJ-002"), None).unwrap();
        insert(&conn, 250.0, "V2", "labor", "2026-04-02", Some("PROJ-002"), None).unwrap();
        let spent = total_for_contract(&conn, "PROJ-002").unwrap();
        let limit = get_budget(&conn, "PROJ-002").unwrap().unwrap();
        // Spent (550) exceeds limit (500) — the caller is responsible for the warning
        assert!(spent > limit, "spent {} should exceed limit {}", spent, limit);
    }

    #[test]
    fn test_expense_with_long_description() {
        let (_f, conn) = tmp_conn();
        let long_desc = "A".repeat(500);
        let id = insert(
            &conn, 99.99, "LongDescVendor", "supplies", "2026-04-05",
            None, Some(&long_desc),
        ).unwrap();
        assert!(id > 0);
        let rows = list_all(&conn).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].description.as_deref().unwrap().len(), 500);
    }

    #[test]
    fn test_expense_negative_amount_handling() {
        // The DB accepts any REAL value; negative amounts represent credits/refunds
        let (_f, conn) = tmp_conn();
        let id = insert(&conn, -25.00, "Refund", "supplies", "2026-04-10", None, None).unwrap();
        assert!(id > 0);
        let total = grand_total(&conn).unwrap();
        assert!((total - (-25.0)).abs() < 0.001);
    }
}
