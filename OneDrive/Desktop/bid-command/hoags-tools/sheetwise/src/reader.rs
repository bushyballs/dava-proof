use std::path::Path;
use std::fs;

/// Column data type inferred from cell values.
#[derive(Debug, Clone, PartialEq)]
pub enum ColType {
    Integer,
    Float,
    Boolean,
    Date,
    String,
}

impl std::fmt::Display for ColType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColType::Integer => write!(f, "integer"),
            ColType::Float => write!(f, "float"),
            ColType::Boolean => write!(f, "boolean"),
            ColType::Date => write!(f, "date"),
            ColType::String => write!(f, "string"),
        }
    }
}

/// A single parsed column descriptor.
#[derive(Debug, Clone)]
pub struct Column {
    pub name: String,
    pub col_type: ColType,
    pub index: usize,
}

/// The complete result of reading a CSV/TSV file.
#[derive(Debug)]
pub struct Sheet {
    pub path: String,
    pub delimiter: u8,
    pub columns: Vec<Column>,
    /// Rows: each row is a Vec<String> aligned to `columns`.
    pub rows: Vec<Vec<String>>,
}

impl Sheet {
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn col_count(&self) -> usize {
        self.columns.len()
    }
}

/// Detect delimiter by sampling the first line.
fn detect_delimiter(first_line: &str) -> u8 {
    let counts = [
        (b',', first_line.chars().filter(|&c| c == ',').count()),
        (b'\t', first_line.chars().filter(|&c| c == '\t').count()),
        (b'|', first_line.chars().filter(|&c| c == '|').count()),
        (b';', first_line.chars().filter(|&c| c == ';').count()),
    ];
    counts
        .iter()
        .max_by_key(|(_, cnt)| *cnt)
        .map(|(delim, _)| *delim)
        .unwrap_or(b',')
}

/// Infer the type of a column by sampling its non-empty values.
fn infer_type(values: &[&str]) -> ColType {
    let non_empty: Vec<&str> = values.iter().copied().filter(|v| !v.is_empty()).collect();
    if non_empty.is_empty() {
        return ColType::String;
    }

    let bool_count = non_empty
        .iter()
        .filter(|v| {
            matches!(
                v.to_ascii_lowercase().as_str(),
                "true" | "false" | "yes" | "no" | "1" | "0"
            )
        })
        .count();
    if bool_count == non_empty.len() {
        return ColType::Boolean;
    }

    // Date patterns: YYYY-MM-DD, MM/DD/YYYY, DD-Mon-YYYY, etc.
    let date_count = non_empty
        .iter()
        .filter(|v| {
            let v = v.trim();
            // ISO 8601 and common US patterns
            (v.len() >= 8 && v.len() <= 10)
                && (v.contains('-') || v.contains('/'))
                && v.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
        })
        .count();
    if date_count as f64 / non_empty.len() as f64 >= 0.8 {
        return ColType::Date;
    }

    let int_count = non_empty
        .iter()
        .filter(|v| v.trim().parse::<i64>().is_ok())
        .count();
    if int_count == non_empty.len() {
        return ColType::Integer;
    }

    // Strip currency/comma and try float
    let float_count = non_empty
        .iter()
        .filter(|v| {
            let cleaned: String = v
                .chars()
                .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
                .collect();
            !cleaned.is_empty() && cleaned.parse::<f64>().is_ok()
        })
        .count();
    if float_count == non_empty.len() {
        return ColType::Float;
    }

    ColType::String
}

/// Read a CSV/TSV file, auto-detecting delimiter and types.
pub fn read_sheet(path: &Path) -> anyhow::Result<Sheet> {
    let raw = fs::read_to_string(path)?;
    let first_line = raw.lines().next().unwrap_or("");
    let delimiter = detect_delimiter(first_line);

    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(true)
        .flexible(true)
        .from_path(path)?;

    let headers: Vec<String> = rdr
        .headers()?
        .iter()
        .map(|h| h.trim().to_string())
        .collect();

    let ncols = headers.len();
    let mut rows: Vec<Vec<String>> = Vec::new();

    for result in rdr.records() {
        let record = result?;
        let mut row: Vec<String> = record.iter().map(|f| f.trim().to_string()).collect();
        // Pad or truncate to header width
        row.resize(ncols, String::new());
        rows.push(row);
    }

    // Infer types per column using up to first 500 rows
    let sample_len = rows.len().min(500);
    let columns: Vec<Column> = (0..ncols)
        .map(|i| {
            let values: Vec<&str> = rows[..sample_len]
                .iter()
                .map(|r| r[i].as_str())
                .collect();
            let col_type = infer_type(&values);
            Column {
                name: headers[i].clone(),
                col_type,
                index: i,
            }
        })
        .collect();

    Ok(Sheet {
        path: path.display().to_string(),
        delimiter,
        columns,
        rows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_delimiter_comma() {
        assert_eq!(detect_delimiter("a,b,c"), b',');
    }

    #[test]
    fn test_detect_delimiter_tab() {
        assert_eq!(detect_delimiter("a\tb\tc"), b'\t');
    }

    #[test]
    fn test_detect_delimiter_pipe() {
        assert_eq!(detect_delimiter("a|b|c"), b'|');
    }

    #[test]
    fn test_infer_integer() {
        let vals = vec!["1", "2", "3", "42"];
        assert_eq!(infer_type(&vals), ColType::Integer);
    }

    #[test]
    fn test_infer_float() {
        let vals = vec!["1.5", "2.7", "3.14"];
        assert_eq!(infer_type(&vals), ColType::Float);
    }

    #[test]
    fn test_infer_boolean() {
        let vals = vec!["true", "false", "True", "False"];
        assert_eq!(infer_type(&vals), ColType::Boolean);
    }

    #[test]
    fn test_infer_string() {
        let vals = vec!["hello", "world", "foo"];
        assert_eq!(infer_type(&vals), ColType::String);
    }

    #[test]
    fn test_read_csv() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "Name,Amount,Active").unwrap();
        writeln!(tmp, "Alice,100,true").unwrap();
        writeln!(tmp, "Bob,200,false").unwrap();
        let sheet = read_sheet(tmp.path()).unwrap();
        assert_eq!(sheet.col_count(), 3);
        assert_eq!(sheet.row_count(), 2);
        assert_eq!(sheet.columns[1].col_type, ColType::Integer);
        assert_eq!(sheet.columns[2].col_type, ColType::Boolean);
    }
}
