use crate::reader::Sheet;
use std::collections::{BTreeMap, HashMap};

/// Result of a pivot operation: group label -> sum value.
pub struct PivotResult {
    pub group_col: String,
    pub sum_col: String,
    /// Sorted by group label.
    pub rows: Vec<(String, f64)>,
}

/// Group rows by `group_col` and sum `sum_col` for each group.
pub fn pivot(sheet: &Sheet, group_col: &str, sum_col: &str) -> anyhow::Result<PivotResult> {
    let g_idx = sheet
        .columns
        .iter()
        .position(|c| c.name.eq_ignore_ascii_case(group_col))
        .ok_or_else(|| anyhow::anyhow!("Column not found: {group_col}"))?;

    let s_idx = sheet
        .columns
        .iter()
        .position(|c| c.name.eq_ignore_ascii_case(sum_col))
        .ok_or_else(|| anyhow::anyhow!("Column not found: {sum_col}"))?;

    let mut sums: BTreeMap<String, f64> = BTreeMap::new();
    let mut counts: HashMap<String, usize> = HashMap::new();

    for row in &sheet.rows {
        let group = row[g_idx].clone();
        let val_str = &row[s_idx];
        let val = parse_num(val_str).unwrap_or(0.0);
        *sums.entry(group.clone()).or_insert(0.0) += val;
        *counts.entry(group).or_insert(0) += 1;
    }

    let rows: Vec<(String, f64)> = sums.into_iter().collect();

    Ok(PivotResult {
        group_col: group_col.to_string(),
        sum_col: sum_col.to_string(),
        rows,
    })
}

fn parse_num(s: &str) -> Option<f64> {
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
        .collect();
    cleaned.parse::<f64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::read_sheet;
    use std::io::Write;

    #[test]
    fn test_pivot_sum() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "Category,Amount").unwrap();
        writeln!(tmp, "A,100").unwrap();
        writeln!(tmp, "B,200").unwrap();
        writeln!(tmp, "A,50").unwrap();
        writeln!(tmp, "B,75").unwrap();
        let sheet = read_sheet(tmp.path()).unwrap();
        let result = pivot(&sheet, "Category", "Amount").unwrap();
        assert_eq!(result.rows.len(), 2);
        let a = result.rows.iter().find(|(k, _)| k == "A").unwrap();
        let b = result.rows.iter().find(|(k, _)| k == "B").unwrap();
        assert!((a.1 - 150.0).abs() < 0.001);
        assert!((b.1 - 275.0).abs() < 0.001);
    }

    #[test]
    fn test_pivot_missing_column() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "Category,Amount").unwrap();
        writeln!(tmp, "A,100").unwrap();
        let sheet = read_sheet(tmp.path()).unwrap();
        assert!(pivot(&sheet, "NoCol", "Amount").is_err());
    }
}
