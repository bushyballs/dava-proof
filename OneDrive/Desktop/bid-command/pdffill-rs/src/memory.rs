use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};

pub struct FieldMemory {
    conn: Connection,
}

pub struct MemoryHit {
    pub value: String,
    pub classification: String,
    pub context_key: String,
    pub confidence: f64,
    pub times_seen: i64,
    pub times_approved: i64,
}

pub struct MemoryStats {
    pub total_fields: i64,
    pub total_templates: i64,
}

impl FieldMemory {
    pub fn open(db_path: &Path) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(db_path)?;
        let mem = Self { conn };
        mem.init_tables()?;
        Ok(mem)
    }

    pub fn open_default() -> Result<Self, rusqlite::Error> {
        let path = default_memory_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        Self::open(&path)
    }

    fn init_tables(&self) -> Result<(), rusqlite::Error> {
        self.conn.execute_batch("
            CREATE TABLE IF NOT EXISTS field_memory (
                id INTEGER PRIMARY KEY,
                label_normalized TEXT UNIQUE,
                classification TEXT,
                typical_value TEXT,
                context_key TEXT,
                times_seen INTEGER DEFAULT 0,
                times_approved INTEGER DEFAULT 0,
                last_seen TEXT,
                source_pdfs TEXT DEFAULT '[]'
            );
            CREATE TABLE IF NOT EXISTS template_memory (
                id INTEGER PRIMARY KEY,
                pdf_hash TEXT UNIQUE,
                form_name TEXT,
                field_count INTEGER DEFAULT 0,
                fields_json TEXT DEFAULT '[]',
                times_seen INTEGER DEFAULT 1,
                last_seen TEXT
            );
        ")?;
        Ok(())
    }

    pub fn store(&self, label: &str, classification: &str, value: &str, context_key: &str, source_pdf: &str, approved: bool) {
        let normalized = label.trim().to_lowercase();
        let now = chrono::Utc::now().to_rfc3339();
        let approved_inc: i64 = if approved { 1 } else { 0 };

        let existing: Option<(i64, String)> = self.conn.query_row(
            "SELECT id, source_pdfs FROM field_memory WHERE label_normalized = ?",
            params![normalized],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).ok();

        if let Some((id, pdfs_json)) = existing {
            let mut pdfs: Vec<String> = serde_json::from_str(&pdfs_json).unwrap_or_default();
            if !pdfs.contains(&source_pdf.to_string()) {
                pdfs.push(source_pdf.to_string());
            }
            let pdfs_str = serde_json::to_string(&pdfs).unwrap_or_default();
            self.conn.execute(
                "UPDATE field_memory SET classification=?, typical_value=?, context_key=?,
                 times_seen=times_seen+1, times_approved=times_approved+?, last_seen=?, source_pdfs=?
                 WHERE id=?",
                params![classification, value, context_key, approved_inc, now, pdfs_str, id],
            ).ok();
        } else {
            let pdfs_str = serde_json::to_string(&vec![source_pdf]).unwrap_or_default();
            self.conn.execute(
                "INSERT INTO field_memory (label_normalized, classification, typical_value, context_key,
                 times_seen, times_approved, last_seen, source_pdfs) VALUES (?,?,?,?,1,?,?,?)",
                params![normalized, classification, value, context_key, approved_inc, now, pdfs_str],
            ).ok();
        }
    }

    pub fn recall(&self, label: &str) -> Option<MemoryHit> {
        let normalized = label.trim().to_lowercase();
        self.conn.query_row(
            "SELECT typical_value, classification, context_key, times_seen, times_approved
             FROM field_memory WHERE label_normalized = ? AND times_approved > 0",
            params![normalized],
            |row| {
                let times_approved: i64 = row.get(4)?;
                let confidence = (0.7 + (times_approved as f64 / (times_approved as f64 + 10.0)) * 0.25).min(0.95);
                Ok(MemoryHit {
                    value: row.get(0)?,
                    classification: row.get(1)?,
                    context_key: row.get(2)?,
                    confidence,
                    times_seen: row.get(3)?,
                    times_approved,
                })
            },
        ).ok()
    }

    pub fn stats(&self) -> MemoryStats {
        let total_fields: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM field_memory", [], |row| row.get(0)
        ).unwrap_or(0);
        let total_templates: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM template_memory", [], |row| row.get(0)
        ).unwrap_or(0);
        MemoryStats { total_fields, total_templates }
    }
}

fn default_memory_path() -> PathBuf {
    let mut path = std::env::current_dir().unwrap_or_default();
    path.push("data");
    path.push("dava_memory.db");
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn tmp_memory() -> FieldMemory {
        let tmp = NamedTempFile::new().unwrap();
        FieldMemory::open(tmp.path()).unwrap()
    }

    #[test]
    fn test_empty_recall() {
        let mem = tmp_memory();
        assert!(mem.recall("nothing").is_none());
    }

    #[test]
    fn test_store_and_recall() {
        let mem = tmp_memory();
        mem.store("offeror name", "identity.name", "Hoags Inc.", "identity.name", "test.pdf", true);
        let hit = mem.recall("offeror name").unwrap();
        assert_eq!(hit.value, "Hoags Inc.");
        assert_eq!(hit.classification, "identity.name");
        assert_eq!(hit.times_approved, 1);
    }

    #[test]
    fn test_unapproved_not_recalled() {
        let mem = tmp_memory();
        mem.store("secret", "identity.name", "val", "key", "a.pdf", false);
        assert!(mem.recall("secret").is_none());
    }

    #[test]
    fn test_confidence_scales() {
        let mem = tmp_memory();
        mem.store("field", "identity.name", "v", "k", "a.pdf", true);
        let c1 = mem.recall("field").unwrap().confidence;
        for i in 0..9 {
            mem.store("field", "identity.name", "v", "k", &format!("{i}.pdf"), true);
        }
        let c10 = mem.recall("field").unwrap().confidence;
        assert!(c10 > c1);
    }

    #[test]
    fn test_store_multiple_pdfs() {
        let mem = tmp_memory();
        mem.store("offeror name", "identity.name", "Hoags Inc.", "identity.name", "form_a.pdf", true);
        mem.store("offeror name", "identity.name", "Hoags Inc.", "identity.name", "form_b.pdf", true);
        mem.store("offeror name", "identity.name", "Hoags Inc.", "identity.name", "form_c.pdf", true);
        // Each unique PDF should be tracked in source_pdfs
        let row: String = mem.conn.query_row(
            "SELECT source_pdfs FROM field_memory WHERE label_normalized = ?",
            rusqlite::params!["offeror name"],
            |r| r.get(0),
        ).unwrap();
        let pdfs: Vec<String> = serde_json::from_str(&row).unwrap();
        assert_eq!(pdfs.len(), 3);
    }

    #[test]
    fn test_stats_after_stores() {
        let mem = tmp_memory();
        assert_eq!(mem.stats().total_fields, 0);
        mem.store("field one", "identity.name", "v1", "k", "a.pdf", true);
        assert_eq!(mem.stats().total_fields, 1);
        mem.store("field two", "identity.phone", "v2", "k", "b.pdf", true);
        assert_eq!(mem.stats().total_fields, 2);
        // Storing field one again should not increase count (it's an update)
        mem.store("field one", "identity.name", "v1_updated", "k", "c.pdf", true);
        assert_eq!(mem.stats().total_fields, 2);
    }
}
