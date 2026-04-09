/// SQLite-backed persistence layer for action items.
use rusqlite::{params, Connection, Result};

use crate::models::ActionItem;

// ---------------------------------------------------------------------------
// Database path
// ---------------------------------------------------------------------------

fn db_path() -> std::path::PathBuf {
    // Prefer XDG_DATA_HOME or fall back to the user home directory.
    let base = std::env::var("ACTIONMINER_DB").ok().map(std::path::PathBuf::from).unwrap_or_else(|| {
        let mut p = dirs_next();
        p.push(".actionminer");
        let _ = std::fs::create_dir_all(&p);
        p.push("actions.db");
        p
    });
    base
}

/// Resolve the data directory — uses HOME on Unix, USERPROFILE on Windows.
fn dirs_next() -> std::path::PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Open (or create) the database and return a connection.
pub fn open() -> Result<Connection> {
    open_at(db_path())
}

/// Open a database at an explicit path (useful in tests).
pub fn open_at(path: impl AsRef<std::path::Path>) -> Result<Connection> {
    let conn = Connection::open(path)?;
    init(&conn)?;
    Ok(conn)
}

fn init(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS action_items (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            description TEXT    NOT NULL,
            assignee    TEXT,
            deadline    TEXT,
            source_file TEXT    NOT NULL DEFAULT '',
            status      TEXT    NOT NULL DEFAULT 'open',
            created_at  TEXT    NOT NULL
        );",
    )
}

/// Insert a new action item and return it with the generated `id` set.
pub fn insert(conn: &Connection, item: &ActionItem) -> Result<ActionItem> {
    conn.execute(
        "INSERT INTO action_items
             (description, assignee, deadline, source_file, status, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            item.description,
            item.assignee,
            item.deadline,
            item.source_file,
            item.status,
            item.created_at,
        ],
    )?;
    let id = conn.last_insert_rowid();
    Ok(ActionItem { id, ..item.clone() })
}

/// Insert many items at once (wrapped in a single transaction).
pub fn insert_many(conn: &Connection, items: &[ActionItem]) -> Result<Vec<ActionItem>> {
    let mut inserted = Vec::with_capacity(items.len());
    let tx = conn.unchecked_transaction()?;
    for item in items {
        inserted.push(insert(conn, item)?);
    }
    tx.commit()?;
    Ok(inserted)
}

/// Retrieve all items, optionally filtered by status.
pub fn list(conn: &Connection, status_filter: Option<&str>) -> Result<Vec<ActionItem>> {
    match status_filter {
        Some(s) => {
            let mut stmt = conn.prepare(
                "SELECT id, description, assignee, deadline, source_file, status, created_at
                 FROM action_items WHERE status = ?1 ORDER BY id",
            )?;
            let rows = stmt.query_map(params![s], row_to_item)?;
            rows.collect()
        }
        None => {
            let mut stmt = conn.prepare(
                "SELECT id, description, assignee, deadline, source_file, status, created_at
                 FROM action_items ORDER BY id",
            )?;
            let rows = stmt.query_map([], row_to_item)?;
            rows.collect()
        }
    }
}

/// Retrieve items assigned to a specific person.
pub fn list_by_assignee(conn: &Connection, assignee: &str) -> Result<Vec<ActionItem>> {
    let mut stmt = conn.prepare(
        "SELECT id, description, assignee, deadline, source_file, status, created_at
         FROM action_items WHERE assignee = ?1 ORDER BY id",
    )?;
    let rows = stmt.query_map(params![assignee], row_to_item)?;
    rows.collect()
}

/// Mark an action item as done.
pub fn complete(conn: &Connection, id: i64) -> Result<bool> {
    let n = conn.execute(
        "UPDATE action_items SET status = 'done' WHERE id = ?1",
        params![id],
    )?;
    Ok(n > 0)
}

/// Export all items as a JSON string.
pub fn export_json(conn: &Connection) -> Result<String> {
    let items = list(conn, None)?;
    Ok(serde_json::to_string_pretty(&items).unwrap_or_default())
}

// ---------------------------------------------------------------------------
// Row mapper
// ---------------------------------------------------------------------------

fn row_to_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<ActionItem> {
    Ok(ActionItem {
        id: row.get(0)?,
        description: row.get(1)?,
        assignee: row.get(2)?,
        deadline: row.get(3)?,
        source_file: row.get(4)?,
        status: row.get(5)?,
        created_at: row.get(6)?,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ActionItem;
    use tempfile::NamedTempFile;

    fn tmp_conn() -> Connection {
        let f = NamedTempFile::new().unwrap();
        open_at(f.path()).unwrap()
    }

    #[test]
    fn test_insert_and_list() {
        let conn = tmp_conn();
        let item = ActionItem::new("Write unit tests", None, None, "meeting.md");
        let inserted = insert(&conn, &item).unwrap();
        assert!(inserted.id > 0);

        let all = list(&conn, None).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].description, "Write unit tests");
    }

    #[test]
    fn test_complete() {
        let conn = tmp_conn();
        let item = ActionItem::new("Deploy service", None, None, "notes.md");
        let inserted = insert(&conn, &item).unwrap();

        let ok = complete(&conn, inserted.id).unwrap();
        assert!(ok);

        let done = list(&conn, Some("done")).unwrap();
        assert_eq!(done.len(), 1);

        let open = list(&conn, Some("open")).unwrap();
        assert!(open.is_empty());
    }

    #[test]
    fn test_complete_nonexistent_returns_false() {
        let conn = tmp_conn();
        let ok = complete(&conn, 9999).unwrap();
        assert!(!ok);
    }

    #[test]
    fn test_export_json() {
        let conn = tmp_conn();
        insert(&conn, &ActionItem::new("Item A", Some("@alice".into()), Some("Friday".into()), "f.md")).unwrap();
        let json = export_json(&conn).unwrap();
        assert!(json.contains("Item A"));
        assert!(json.contains("@alice"));
    }

    #[test]
    fn test_list_by_assignee() {
        let conn = tmp_conn();
        insert(&conn, &ActionItem::new("Task 1", Some("@alice".into()), None, "f.md")).unwrap();
        insert(&conn, &ActionItem::new("Task 2", Some("@bob".into()),   None, "f.md")).unwrap();
        let alice = list_by_assignee(&conn, "@alice").unwrap();
        assert_eq!(alice.len(), 1);
        assert_eq!(alice[0].description, "Task 1");
    }

    #[test]
    fn test_insert_many() {
        let conn = tmp_conn();
        let items = vec![
            ActionItem::new("A", None, None, "f.md"),
            ActionItem::new("B", None, None, "f.md"),
            ActionItem::new("C", None, None, "f.md"),
        ];
        let ins = insert_many(&conn, &items).unwrap();
        assert_eq!(ins.len(), 3);
        assert_eq!(list(&conn, None).unwrap().len(), 3);
    }
}
