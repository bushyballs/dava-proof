use serde::{Deserialize, Serialize};

/// A single extracted action item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionItem {
    /// Unique row ID (0 before insertion).
    pub id: i64,
    /// Human-readable description of the action.
    pub description: String,
    /// Optional person responsible.
    pub assignee: Option<String>,
    /// Optional deadline / due-date string as found in text.
    pub deadline: Option<String>,
    /// Path to the source file (or "<stdin>").
    pub source_file: String,
    /// "open" or "done".
    pub status: String,
    /// RFC-3339 timestamp when the item was recorded.
    pub created_at: String,
}

impl ActionItem {
    pub fn new(
        description: impl Into<String>,
        assignee: Option<String>,
        deadline: Option<String>,
        source_file: impl Into<String>,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: 0,
            description: description.into(),
            assignee,
            deadline,
            source_file: source_file.into(),
            status: "open".into(),
            created_at: now,
        }
    }
}
