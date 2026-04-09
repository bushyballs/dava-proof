/// Conversion logic: ties together format detection, extraction, and output.

use crate::extract;
use crate::formats::{detect_format, is_supported, Format};
use serde_json::Value;
use std::path::Path;

/// The result of a successful conversion.
pub struct ConvertResult {
    pub output: String,
    pub format: Format,
}

/// Convert `input_path` to `target_format`, returning the converted text.
pub fn convert(input_path: &Path, target_format: &Format) -> anyhow::Result<ConvertResult> {
    let src_format = detect_format(input_path);

    if !is_supported(&src_format, target_format) {
        anyhow::bail!(
            "Conversion from {} to {} is not supported",
            src_format,
            target_format
        );
    }

    let output = match (&src_format, target_format) {
        (Format::Pdf, Format::Text) => pdf_to_text(input_path)?,
        (Format::Pdf, Format::Json) => pdf_to_json(input_path)?,
        (Format::Csv, Format::Json) => csv_to_json(input_path)?,
        (Format::Json, Format::Csv) => json_to_csv(input_path)?,
        _ => unreachable!("is_supported should have caught this"),
    };

    Ok(ConvertResult {
        output,
        format: target_format.clone(),
    })
}

// ─── PDF → Text ──────────────────────────────────────────────────────────────

fn pdf_to_text(path: &Path) -> anyhow::Result<String> {
    extract::extract_all_text(path)
}

// ─── PDF → JSON ──────────────────────────────────────────────────────────────

fn pdf_to_json(path: &Path) -> anyhow::Result<String> {
    let doc = extract::extract_to_json(path)?;
    Ok(serde_json::to_string_pretty(&doc)?)
}

// ─── CSV → JSON ──────────────────────────────────────────────────────────────

/// Parse a CSV file (no external crate) and convert to a JSON array of objects.
pub fn csv_to_json(path: &Path) -> anyhow::Result<String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read CSV: {}", e))?;
    let records = parse_csv_records(&raw)?;
    let json = serde_json::to_string_pretty(&records)?;
    Ok(json)
}

/// Parse CSV text into a JSON Value (array of objects keyed by header row).
pub fn parse_csv_records(raw: &str) -> anyhow::Result<Value> {
    let mut lines = raw.lines();

    let header_line = lines
        .next()
        .ok_or_else(|| anyhow::anyhow!("CSV is empty"))?;
    let headers: Vec<String> = parse_csv_line(header_line);

    if headers.is_empty() {
        anyhow::bail!("CSV has no headers");
    }

    let mut rows: Vec<Value> = Vec::new();
    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let values = parse_csv_line(line);
        let mut obj = serde_json::Map::new();
        for (i, header) in headers.iter().enumerate() {
            let val = values.get(i).cloned().unwrap_or_default();
            obj.insert(header.clone(), Value::String(val));
        }
        rows.push(Value::Object(obj));
    }

    Ok(Value::Array(rows))
}

/// Parse a single CSV line, handling double-quoted fields (RFC 4180 basics).
pub fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                if in_quotes {
                    // Peek: escaped quote "\"\"" → literal quote
                    if chars.peek() == Some(&'"') {
                        chars.next();
                        current.push('"');
                    } else {
                        in_quotes = false;
                    }
                } else {
                    in_quotes = true;
                }
            }
            ',' if !in_quotes => {
                fields.push(current.trim().to_string());
                current = String::new();
            }
            other => {
                current.push(other);
            }
        }
    }
    fields.push(current.trim().to_string());
    fields
}

// ─── JSON → CSV ──────────────────────────────────────────────────────────────

/// Convert a JSON file (array of flat objects) to CSV.
pub fn json_to_csv(path: &Path) -> anyhow::Result<String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read JSON: {}", e))?;
    json_str_to_csv(&raw)
}

/// Convert a JSON string (array of flat objects) to CSV text.
pub fn json_str_to_csv(raw: &str) -> anyhow::Result<String> {
    let parsed: Value =
        serde_json::from_str(raw).map_err(|e| anyhow::anyhow!("Invalid JSON: {}", e))?;

    let rows = parsed
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("JSON must be an array of objects"))?;

    if rows.is_empty() {
        return Ok(String::new());
    }

    // Collect all unique headers (preserving insertion order from first row).
    let mut headers: Vec<String> = Vec::new();
    for row in rows.iter() {
        if let Some(obj) = row.as_object() {
            for key in obj.keys() {
                if !headers.contains(key) {
                    headers.push(key.clone());
                }
            }
        }
    }

    let mut out = String::new();
    // Header row
    out.push_str(&csv_join(&headers));
    out.push('\n');

    // Data rows
    for row in rows {
        if let Some(obj) = row.as_object() {
            let values: Vec<String> = headers
                .iter()
                .map(|h| {
                    obj.get(h)
                        .map(value_to_csv_cell)
                        .unwrap_or_default()
                })
                .collect();
            out.push_str(&csv_join(&values));
            out.push('\n');
        }
    }

    Ok(out)
}

/// Escape and join fields into a CSV line.
fn csv_join(fields: &[String]) -> String {
    fields
        .iter()
        .map(|f| csv_escape(f))
        .collect::<Vec<_>>()
        .join(",")
}

/// RFC 4180 CSV escaping: wrap in quotes if field contains comma, quote, or newline.
fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Convert a JSON Value to a flat CSV cell string.
fn value_to_csv_cell(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── CSV parsing ────────────────────────────────────────────────────────

    #[test]
    fn csv_to_json_basic() {
        let csv = "name,age,city\nAlice,30,NYC\nBob,25,LA\n";
        let val = parse_csv_records(csv).unwrap();
        let arr = val.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["name"], "Alice");
        assert_eq!(arr[0]["age"], "30");
        assert_eq!(arr[1]["city"], "LA");
    }

    #[test]
    fn csv_quoted_field() {
        let csv = "name,notes\nAlice,\"hello, world\"\n";
        let val = parse_csv_records(csv).unwrap();
        let arr = val.as_array().unwrap();
        assert_eq!(arr[0]["notes"], "hello, world");
    }

    #[test]
    fn csv_empty_fields() {
        let csv = "a,b,c\n1,,3\n";
        let val = parse_csv_records(csv).unwrap();
        assert_eq!(val[0]["b"], "");
    }

    #[test]
    fn csv_single_row_no_data() {
        // Only headers, no data rows → empty array
        let csv = "name,age\n";
        let val = parse_csv_records(csv).unwrap();
        assert_eq!(val.as_array().unwrap().len(), 0);
    }

    // ─── JSON → CSV ─────────────────────────────────────────────────────────

    #[test]
    fn json_to_csv_basic() {
        let json = r#"[{"name":"Alice","age":"30"},{"name":"Bob","age":"25"}]"#;
        let csv = json_str_to_csv(json).unwrap();
        let lines: Vec<&str> = csv.lines().collect();
        // Header row must contain both columns (order depends on JSON parser).
        assert!(lines[0].contains("name") && lines[0].contains("age"), "header: {}", lines[0]);
        // Data rows must contain the values.
        assert!(lines[1].contains("Alice") && lines[1].contains("30"), "row1: {}", lines[1]);
        assert!(lines[2].contains("Bob") && lines[2].contains("25"), "row2: {}", lines[2]);
    }

    #[test]
    fn json_to_csv_needs_quoting() {
        let json = r#"[{"note":"hello, world"}]"#;
        let csv = json_str_to_csv(json).unwrap();
        assert!(csv.contains("\"hello, world\""));
    }

    #[test]
    fn json_to_csv_empty_array() {
        let csv = json_str_to_csv("[]").unwrap();
        assert_eq!(csv, "");
    }

    #[test]
    fn json_to_csv_not_array_errors() {
        let result = json_str_to_csv(r#"{"key":"val"}"#);
        assert!(result.is_err());
    }

    // ─── CSV escape helper ───────────────────────────────────────────────────

    #[test]
    fn csv_escape_no_special() {
        assert_eq!(csv_escape("hello"), "hello");
    }

    #[test]
    fn csv_escape_comma() {
        assert_eq!(csv_escape("a,b"), "\"a,b\"");
    }

    #[test]
    fn csv_escape_quote() {
        assert_eq!(csv_escape("say \"hi\""), "\"say \"\"hi\"\"\"");
    }
}
