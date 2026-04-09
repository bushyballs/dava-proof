use crate::reader::Sheet;

/// Supported comparison operators for filter expressions.
#[derive(Debug, Clone, PartialEq)]
pub enum FilterOp {
    Eq,
    Ne,
    Gt,
    Lt,
    Gte,
    Lte,
    Contains,
    StartsWith,
}

impl FilterOp {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "eq" => Some(Self::Eq),
            "ne" => Some(Self::Ne),
            "gt" => Some(Self::Gt),
            "lt" => Some(Self::Lt),
            "gte" => Some(Self::Gte),
            "lte" => Some(Self::Lte),
            "contains" => Some(Self::Contains),
            "starts_with" | "startswith" => Some(Self::StartsWith),
            _ => None,
        }
    }
}

/// A single filter expression: column op value.
#[derive(Debug, Clone)]
pub struct Filter {
    pub column: String,
    pub op: FilterOp,
    pub value: String,
}

/// Apply a single filter to a sheet, returning matching rows.
/// Numeric comparisons are done as f64; string comparisons as case-insensitive.
pub fn apply_filter<'a>(sheet: &'a Sheet, filter: &Filter) -> Vec<&'a Vec<String>> {
    let col_idx = match sheet
        .columns
        .iter()
        .position(|c| c.name.eq_ignore_ascii_case(&filter.column))
    {
        Some(i) => i,
        None => return vec![],
    };

    let filter_num = filter.value.parse::<f64>().ok();

    sheet
        .rows
        .iter()
        .filter(|row| {
            let cell = &row[col_idx];
            match &filter.op {
                FilterOp::Eq => cell.eq_ignore_ascii_case(&filter.value),
                FilterOp::Ne => !cell.eq_ignore_ascii_case(&filter.value),
                FilterOp::Contains => cell
                    .to_ascii_lowercase()
                    .contains(&filter.value.to_ascii_lowercase()),
                FilterOp::StartsWith => cell
                    .to_ascii_lowercase()
                    .starts_with(&filter.value.to_ascii_lowercase()),
                op => {
                    // Numeric comparison
                    if let (Some(fn_), Some(cv)) = (filter_num, parse_num(cell)) {
                        match op {
                            FilterOp::Gt => cv > fn_,
                            FilterOp::Lt => cv < fn_,
                            FilterOp::Gte => cv >= fn_,
                            FilterOp::Lte => cv <= fn_,
                            _ => false,
                        }
                    } else {
                        // Fall back to lexicographic
                        match op {
                            FilterOp::Gt => cell.as_str() > filter.value.as_str(),
                            FilterOp::Lt => cell.as_str() < filter.value.as_str(),
                            FilterOp::Gte => cell.as_str() >= filter.value.as_str(),
                            FilterOp::Lte => cell.as_str() <= filter.value.as_str(),
                            _ => false,
                        }
                    }
                }
            }
        })
        .collect()
}

/// Apply multiple filters (AND logic) and return owned rows.
#[allow(dead_code)]
pub fn apply_filters(sheet: &Sheet, filters: &[Filter]) -> Vec<Vec<String>> {
    if filters.is_empty() {
        return sheet.rows.clone();
    }

    // Start with all rows, progressively narrow down
    let mut candidates: Vec<&Vec<String>> = sheet.rows.iter().collect();
    for filter in filters {
        let col_idx = sheet
            .columns
            .iter()
            .position(|c| c.name.eq_ignore_ascii_case(&filter.column));
        let col_idx = match col_idx {
            Some(i) => i,
            None => return vec![],
        };
        let filter_num = filter.value.parse::<f64>().ok();
        candidates = candidates
            .into_iter()
            .filter(|row| {
                let cell = &row[col_idx];
                match &filter.op {
                    FilterOp::Eq => cell.eq_ignore_ascii_case(&filter.value),
                    FilterOp::Ne => !cell.eq_ignore_ascii_case(&filter.value),
                    FilterOp::Contains => cell
                        .to_ascii_lowercase()
                        .contains(&filter.value.to_ascii_lowercase()),
                    FilterOp::StartsWith => cell
                        .to_ascii_lowercase()
                        .starts_with(&filter.value.to_ascii_lowercase()),
                    op => {
                        if let (Some(fn_), Some(cv)) = (filter_num, parse_num(cell)) {
                            match op {
                                FilterOp::Gt => cv > fn_,
                                FilterOp::Lt => cv < fn_,
                                FilterOp::Gte => cv >= fn_,
                                FilterOp::Lte => cv <= fn_,
                                _ => false,
                            }
                        } else {
                            match op {
                                FilterOp::Gt => cell.as_str() > filter.value.as_str(),
                                FilterOp::Lt => cell.as_str() < filter.value.as_str(),
                                FilterOp::Gte => cell.as_str() >= filter.value.as_str(),
                                FilterOp::Lte => cell.as_str() <= filter.value.as_str(),
                                _ => false,
                            }
                        }
                    }
                }
            })
            .collect();
    }
    candidates.into_iter().cloned().collect()
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

    fn make_sheet() -> Sheet {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "Name,Amount,City").unwrap();
        writeln!(tmp, "Alice,500,Denver").unwrap();
        writeln!(tmp, "Bob,1500,Austin").unwrap();
        writeln!(tmp, "Carol,1000,Denver").unwrap();
        writeln!(tmp, "Dave,2000,Boston").unwrap();
        read_sheet(tmp.path()).unwrap()
    }

    #[test]
    fn test_filter_gt() {
        let sheet = make_sheet();
        let f = Filter {
            column: "Amount".to_string(),
            op: FilterOp::Gt,
            value: "1000".to_string(),
        };
        let rows = apply_filter(&sheet, &f);
        assert_eq!(rows.len(), 2); // Bob 1500, Dave 2000
    }

    #[test]
    fn test_filter_eq() {
        let sheet = make_sheet();
        let f = Filter {
            column: "City".to_string(),
            op: FilterOp::Eq,
            value: "Denver".to_string(),
        };
        let rows = apply_filter(&sheet, &f);
        assert_eq!(rows.len(), 2); // Alice, Carol
    }

    #[test]
    fn test_filter_contains() {
        let sheet = make_sheet();
        let f = Filter {
            column: "City".to_string(),
            op: FilterOp::Contains,
            value: "on".to_string(),
        };
        let rows = apply_filter(&sheet, &f);
        assert_eq!(rows.len(), 1); // only Boston contains "on"
    }

    #[test]
    fn test_filter_starts_with() {
        let sheet = make_sheet();
        let f = Filter {
            column: "Name".to_string(),
            op: FilterOp::StartsWith,
            value: "A".to_string(),
        };
        let rows = apply_filter(&sheet, &f);
        assert_eq!(rows.len(), 1); // Alice
    }

    #[test]
    fn test_chained_filters() {
        let sheet = make_sheet();
        let filters = vec![
            Filter {
                column: "Amount".to_string(),
                op: FilterOp::Gte,
                value: "1000".to_string(),
            },
            Filter {
                column: "City".to_string(),
                op: FilterOp::Ne,
                value: "Boston".to_string(),
            },
        ];
        let rows = apply_filters(&sheet, &filters);
        // Bob(1500,Austin) and Carol(1000,Denver) — not Dave because Boston excluded
        assert_eq!(rows.len(), 2);
    }
}
