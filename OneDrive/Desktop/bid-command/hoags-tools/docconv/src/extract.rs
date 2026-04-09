/// Text and table extraction from PDF documents.

use lopdf::Document;
use serde_json::{json, Value};
use std::path::Path;

/// Extract all text from a PDF, page by page.
/// Returns a vector of (page_number, text) tuples (1-indexed).
pub fn extract_text_pages(path: &Path) -> anyhow::Result<Vec<(u32, String)>> {
    let doc = Document::load(path)
        .map_err(|e| anyhow::anyhow!("Failed to load PDF: {}", e))?;

    let pages = doc.get_pages();
    let mut results: Vec<(u32, String)> = Vec::new();

    let mut page_numbers: Vec<u32> = pages.keys().copied().collect();
    page_numbers.sort_unstable();

    for page_num in page_numbers {
        let text = doc
            .extract_text(&[page_num])
            .unwrap_or_default();
        results.push((page_num, text));
    }

    Ok(results)
}

/// Extract all text from a PDF as a single string.
pub fn extract_all_text(path: &Path) -> anyhow::Result<String> {
    let pages = extract_text_pages(path)?;
    let text = pages
        .into_iter()
        .map(|(_, t)| t)
        .collect::<Vec<_>>()
        .join("\n\n");
    Ok(text)
}

/// Build a structured JSON document from a PDF.
/// Shape: { "page_count": N, "metadata": {...}, "pages": [ { "page": N, "text": "..." } ] }
pub fn extract_to_json(path: &Path) -> anyhow::Result<Value> {
    let doc = Document::load(path)
        .map_err(|e| anyhow::anyhow!("Failed to load PDF: {}", e))?;

    let page_count = doc.get_pages().len() as u64;

    // Pull a small set of well-known Info dict fields.
    let mut metadata = serde_json::Map::new();
    if let Ok(info) = doc.trailer.get(b"Info") {
        if let Ok(dict_id) = info.as_reference() {
            if let Ok(lopdf::Object::Dictionary(dict)) = doc.get_object(dict_id) {
                for key in &[
                    b"Title" as &[u8],
                    b"Author",
                    b"Subject",
                    b"Creator",
                    b"Producer",
                    b"CreationDate",
                    b"ModDate",
                ] {
                    if let Ok(val) = dict.get(*key) {
                        let s = val
                            .as_str()
                            .map(|b| String::from_utf8_lossy(b).into_owned())
                            .unwrap_or_default();
                        let field = String::from_utf8_lossy(key).to_lowercase();
                        metadata.insert(field, Value::String(s));
                    }
                }
            }
        }
    }

    let pages_json = extract_text_pages(path)?
        .into_iter()
        .map(|(n, t)| json!({ "page": n, "text": t }))
        .collect::<Vec<_>>();

    Ok(json!({
        "page_count": page_count,
        "metadata": metadata,
        "pages": pages_json,
    }))
}

/// Naively extract table-like rows from a PDF page.
///
/// Strategy: split text into lines; lines that contain 2+ whitespace-separated
/// "cells" (tab or multiple spaces) are treated as table rows. Returns each
/// candidate row as a `Vec<String>` of cells.
pub fn extract_tables(path: &Path) -> anyhow::Result<Vec<Vec<String>>> {
    let all_text = extract_all_text(path)?;
    let mut rows: Vec<Vec<String>> = Vec::new();

    for line in all_text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Split on tabs or runs of 2+ spaces.
        let cells: Vec<String> = split_cells(line)
            .into_iter()
            .filter(|c| !c.is_empty())
            .collect();
        if cells.len() >= 2 {
            rows.push(cells);
        }
    }

    Ok(rows)
}

/// Split a line on tab characters or two-or-more consecutive spaces.
fn split_cells(line: &str) -> Vec<String> {
    let mut cells: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut prev_space = false;
    let mut space_run = 0usize;

    for ch in line.chars() {
        if ch == '\t' {
            cells.push(current.trim().to_string());
            current = String::new();
            prev_space = false;
            space_run = 0;
        } else if ch == ' ' {
            if prev_space {
                space_run += 1;
                if space_run >= 2 {
                    // flush current cell
                    cells.push(current.trim().to_string());
                    current = String::new();
                    space_run = 0;
                    prev_space = false;
                    continue;
                }
            }
            current.push(ch);
            prev_space = true;
        } else {
            prev_space = false;
            space_run = 0;
            current.push(ch);
        }
    }

    cells.push(current.trim().to_string());
    cells
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_cells_tab_separated() {
        let cells = split_cells("Name\tAge\tCity");
        assert_eq!(cells, vec!["Name", "Age", "City"]);
    }

    #[test]
    fn split_cells_multi_space() {
        let cells = split_cells("Alice   30   NYC");
        assert!(cells.len() >= 2);
        assert!(cells.iter().any(|c| c == "Alice"));
    }

    #[test]
    fn split_cells_single_word_no_split() {
        let cells = split_cells("hello");
        assert_eq!(cells, vec!["hello"]);
    }
}
