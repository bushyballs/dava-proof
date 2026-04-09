use serde::{Deserialize, Serialize};

/// Valid expense categories for federal contracting cost tracking.
pub const VALID_CATEGORIES: &[&str] = &[
    "supplies", "fuel", "labor", "equipment", "travel", "office", "other",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Expense {
    pub id: i64,
    pub amount: f64,
    pub vendor: String,
    pub category: String,
    pub date: String,
    pub contract_number: Option<String>,
    pub description: Option<String>,
    pub created_at: String,
}

impl Expense {
    /// Validate that the category is one of the known values.
    /// Unknown values are coerced to "other" with a warning, not rejected,
    /// so the CLI stays flexible for novel inputs.
    pub fn normalize_category(raw: &str) -> String {
        let lower = raw.to_lowercase();
        if VALID_CATEGORIES.contains(&lower.as_str()) {
            lower
        } else {
            eprintln!(
                "Warning: unknown category '{}', storing as 'other'. \
                 Valid categories: {}",
                raw,
                VALID_CATEGORIES.join(", ")
            );
            "other".to_string()
        }
    }

    /// Return a display-friendly header line.
    pub fn header() -> &'static str {
        "ID  | Date       | Amount    | Category   | Vendor                        | Contract"
    }

    /// Format the expense as a fixed-width table row.
    pub fn to_row(&self) -> String {
        format!(
            "{:<4}| {:<11}| ${:<9.2}| {:<11}| {:<30}| {}",
            self.id,
            self.date,
            self.amount,
            self.category,
            self.vendor,
            self.contract_number.as_deref().unwrap_or("—"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_known_category() {
        assert_eq!(Expense::normalize_category("Supplies"), "supplies");
        assert_eq!(Expense::normalize_category("FUEL"), "fuel");
        assert_eq!(Expense::normalize_category("labor"), "labor");
    }

    #[test]
    fn test_normalize_unknown_category_falls_back_to_other() {
        assert_eq!(Expense::normalize_category("random_thing"), "other");
    }

    #[test]
    fn test_to_row_with_contract() {
        let e = Expense {
            id: 1,
            amount: 45.99,
            vendor: "Home Depot".to_string(),
            category: "supplies".to_string(),
            date: "2026-04-08".to_string(),
            contract_number: Some("W9127S26QA030".to_string()),
            description: None,
            created_at: "2026-04-08T12:00:00Z".to_string(),
        };
        let row = e.to_row();
        assert!(row.contains("Home Depot"));
        assert!(row.contains("W9127S26QA030"));
        assert!(row.contains("45.99"));
    }

    #[test]
    fn test_to_row_no_contract() {
        let e = Expense {
            id: 2,
            amount: 10.00,
            vendor: "Gas Station".to_string(),
            category: "fuel".to_string(),
            date: "2026-04-01".to_string(),
            contract_number: None,
            description: None,
            created_at: "2026-04-01T08:00:00Z".to_string(),
        };
        let row = e.to_row();
        assert!(row.contains("—"));
    }
}
