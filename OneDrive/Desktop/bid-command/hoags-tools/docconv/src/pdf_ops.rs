/// PDF-level operations: merge, split, metadata, single-page text extraction.

use lopdf::{Dictionary, Document, Object, ObjectId};
use std::collections::BTreeMap;
use std::path::Path;

// ─── Public metadata struct ───────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct PdfMetadata {
    pub page_count: u32,
    pub file_size: u64,
    pub title: Option<String>,
    pub author: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
    pub creation_date: Option<String>,
    pub mod_date: Option<String>,
}

// ─── Page-range parsing ───────────────────────────────────────────────────────

/// Parse a page-range string like "1-5" or "3" into an inclusive (start, end) pair.
/// Pages are 1-indexed. Returns an error if the format is invalid.
pub fn parse_page_range(s: &str) -> anyhow::Result<(u32, u32)> {
    let s = s.trim();
    if let Some((a, b)) = s.split_once('-') {
        let start: u32 = a
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid start page: {a}"))?;
        let end: u32 = b
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid end page: {b}"))?;
        if start == 0 || end == 0 {
            anyhow::bail!("Page numbers are 1-indexed; got: {s}");
        }
        if start > end {
            anyhow::bail!("Start page ({start}) must be <= end page ({end})");
        }
        Ok((start, end))
    } else {
        let page: u32 = s
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid page number: {s}"))?;
        if page == 0 {
            anyhow::bail!("Page numbers are 1-indexed; got 0");
        }
        Ok((page, page))
    }
}

// ─── Merge ────────────────────────────────────────────────────────────────────

/// Merge multiple PDFs into a single output PDF file.
///
/// Strategy: load all documents, renumber each subsequent doc's objects to
/// avoid collisions, then build a new Pages tree that references all pages.
pub fn merge_pdfs(inputs: &[&Path], output: &Path) -> anyhow::Result<()> {
    if inputs.is_empty() {
        anyhow::bail!("No input PDFs provided");
    }

    // Load all source documents.
    let mut docs: Vec<Document> = inputs
        .iter()
        .enumerate()
        .map(|(i, p)| {
            Document::load(p).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to load input PDF #{} ({}): {e}",
                    i + 1,
                    p.display()
                )
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    // Renumber each doc so IDs don't collide.  Start the second doc's IDs
    // after the first's max_id, and so on.
    let mut offset: u32 = 0;
    for doc in &mut docs {
        if offset > 0 {
            doc.renumber_objects_with(offset + 1);
        }
        offset += doc.max_id;
    }

    // Collect all page ObjectIds in order across all documents.
    let mut all_page_ids: Vec<ObjectId> = Vec::new();
    for doc in &docs {
        let pages: BTreeMap<u32, ObjectId> = doc.get_pages();
        let mut nums: Vec<u32> = pages.keys().copied().collect();
        nums.sort_unstable();
        for n in nums {
            all_page_ids.push(pages[&n]);
        }
    }

    // Build a new merged document.
    let mut merged = Document::with_version("1.5");

    // Copy all objects from every source doc into the merged doc.
    for doc in &docs {
        for (&id, obj) in &doc.objects {
            merged.objects.insert(id, obj.clone());
        }
        if doc.max_id > merged.max_id {
            merged.max_id = doc.max_id;
        }
    }

    // Build a new Pages node with all pages as kids.
    let pages_id = merged.new_object_id();

    // Update each page's /Parent to point to our new Pages node.
    for &pid in &all_page_ids {
        if let Ok(obj) = merged.get_object_mut(pid) {
            if let Ok(dict) = obj.as_dict_mut() {
                dict.set("Parent", Object::Reference(pages_id));
            }
        }
    }

    let kids: Vec<Object> = all_page_ids
        .iter()
        .map(|&id| Object::Reference(id))
        .collect();

    let pages_dict = Dictionary::from_iter(vec![
        ("Type", Object::Name(b"Pages".to_vec())),
        ("Count", Object::Integer(all_page_ids.len() as i64)),
        ("Kids", Object::Array(kids)),
    ]);
    merged.objects.insert(pages_id, Object::Dictionary(pages_dict));

    // Build a new Catalog.
    let catalog_id = merged.add_object(Dictionary::from_iter(vec![
        ("Type", Object::Name(b"Catalog".to_vec())),
        ("Pages", Object::Reference(pages_id)),
    ]));

    merged.trailer.set("Root", Object::Reference(catalog_id));
    merged
        .trailer
        .set("Size", Object::Integer((merged.max_id + 1) as i64));

    merged
        .save(output)
        .map_err(|e| anyhow::anyhow!("Failed to write merged PDF: {e}"))?;

    Ok(())
}

// ─── Split ────────────────────────────────────────────────────────────────────

/// Extract pages `start..=end` (1-indexed, inclusive) from `source` and write to `output`.
///
/// Strategy: load document, delete all pages outside the requested range.
pub fn split_pdf(source: &Path, start: u32, end: u32, output: &Path) -> anyhow::Result<()> {
    let mut doc = Document::load(source)
        .map_err(|e| anyhow::anyhow!("Failed to load PDF '{}': {e}", source.display()))?;

    let total = doc.get_pages().len() as u32;

    if start < 1 || end > total {
        anyhow::bail!(
            "Page range {start}-{end} is out of bounds (document has {total} pages)"
        );
    }

    // Collect pages to delete (everything outside start..=end).
    let to_delete: Vec<u32> = (1..=total)
        .filter(|&n| n < start || n > end)
        .collect();

    if !to_delete.is_empty() {
        doc.delete_pages(&to_delete);
    }

    doc.save(output)
        .map_err(|e| anyhow::anyhow!("Failed to write split PDF: {e}"))?;

    Ok(())
}

// ─── Metadata ─────────────────────────────────────────────────────────────────

/// Read PDF metadata from the Info dictionary and filesystem.
pub fn read_metadata(path: &Path) -> anyhow::Result<PdfMetadata> {
    let doc = Document::load(path)
        .map_err(|e| anyhow::anyhow!("Failed to load PDF '{}': {e}", path.display()))?;

    let page_count = doc.get_pages().len() as u32;
    let file_size = std::fs::metadata(path)
        .map(|m| m.len())
        .unwrap_or(0);

    let mut meta = PdfMetadata {
        page_count,
        file_size,
        ..Default::default()
    };

    // Try to extract Info dictionary.
    if let Ok(info_ref) = doc.trailer.get(b"Info") {
        if let Ok(dict_id) = info_ref.as_reference() {
            if let Ok(Object::Dictionary(dict)) = doc.get_object(dict_id) {
                let get_str = |key: &[u8]| -> Option<String> {
                    dict.get(key)
                        .ok()
                        .and_then(|v| v.as_str().ok())
                        .map(|b| String::from_utf8_lossy(b).into_owned())
                        .filter(|s| !s.is_empty())
                };
                meta.title = get_str(b"Title");
                meta.author = get_str(b"Author");
                meta.creator = get_str(b"Creator");
                meta.producer = get_str(b"Producer");
                meta.creation_date = get_str(b"CreationDate");
                meta.mod_date = get_str(b"ModDate");
            }
        }
    }

    Ok(meta)
}

// ─── Single-page text extraction ─────────────────────────────────────────────

/// Extract text from a single page (1-indexed) of a PDF.
pub fn extract_page_text(path: &Path, page_number: u32) -> anyhow::Result<String> {
    let doc = Document::load(path)
        .map_err(|e| anyhow::anyhow!("Failed to load PDF '{}': {e}", path.display()))?;

    let pages = doc.get_pages();
    let total = pages.len() as u32;

    if page_number < 1 || page_number > total {
        anyhow::bail!(
            "Page {page_number} out of range (document has {total} pages)"
        );
    }

    let text = doc
        .extract_text(&[page_number])
        .unwrap_or_default();

    Ok(text)
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // ── parse_page_range ─────────────────────────────────────────────────────

    #[test]
    fn parse_single_page() {
        let (s, e) = parse_page_range("3").unwrap();
        assert_eq!((s, e), (3, 3));
    }

    #[test]
    fn parse_range() {
        let (s, e) = parse_page_range("1-5").unwrap();
        assert_eq!((s, e), (1, 5));
    }

    #[test]
    fn parse_range_with_spaces() {
        let (s, e) = parse_page_range(" 2 - 4 ").unwrap();
        assert_eq!((s, e), (2, 4));
    }

    #[test]
    fn parse_range_inverted_errors() {
        assert!(parse_page_range("5-2").is_err());
    }

    #[test]
    fn parse_zero_page_errors() {
        assert!(parse_page_range("0").is_err());
        assert!(parse_page_range("0-3").is_err());
    }

    #[test]
    fn parse_invalid_string_errors() {
        assert!(parse_page_range("abc").is_err());
        assert!(parse_page_range("1-z").is_err());
    }

    // ── Minimal PDF helpers for tests ────────────────────────────────────────

    /// Build the smallest valid 1-page PDF as raw bytes that lopdf can load.
    fn minimal_pdf_bytes(page_text: &str) -> Vec<u8> {
        let stream_content = format!("BT /F1 12 Tf 72 720 Td ({page_text}) Tj ET");
        let stream_len = stream_content.len();

        let mut pdf = String::new();
        pdf.push_str("%PDF-1.4\n");

        let obj1 = "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n";
        let obj2 = "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n";
        let obj3 = "3 0 obj\n<< /Type /Page /Parent 2 0 R \
             /MediaBox [0 0 612 792] \
             /Contents 4 0 R \
             /Resources << /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >> \
             >>\nendobj\n";
        let obj4 = format!(
            "4 0 obj\n<< /Length {stream_len} >>\nstream\n{stream_content}\nendstream\nendobj\n"
        );

        let off1 = 9usize; // after "%PDF-1.4\n"
        let off2 = off1 + obj1.len();
        let off3 = off2 + obj2.len();
        let off4 = off3 + obj3.len();
        let xref_start = off4 + obj4.len();

        pdf.push_str(obj1);
        pdf.push_str(obj2);
        pdf.push_str(obj3);
        pdf.push_str(&obj4);

        pdf.push_str("xref\n0 5\n");
        pdf.push_str("0000000000 65535 f \n");
        pdf.push_str(&format!("{off1:010} 00000 n \n"));
        pdf.push_str(&format!("{off2:010} 00000 n \n"));
        pdf.push_str(&format!("{off3:010} 00000 n \n"));
        pdf.push_str(&format!("{off4:010} 00000 n \n"));
        pdf.push_str("trailer\n<< /Size 5 /Root 1 0 R >>\n");
        pdf.push_str(&format!("startxref\n{xref_start}\n%%EOF\n"));

        pdf.into_bytes()
    }

    fn write_temp_pdf(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(&minimal_pdf_bytes(content)).unwrap();
        f
    }

    // ── Merge tests ──────────────────────────────────────────────────────────

    #[test]
    fn merge_two_pdfs_produces_valid_file() {
        let a = write_temp_pdf("Page A content");
        let b = write_temp_pdf("Page B content");
        let out = NamedTempFile::new().unwrap();
        let inputs: &[&Path] = &[a.path(), b.path()];

        let result = merge_pdfs(inputs, out.path());
        assert!(result.is_ok(), "merge failed: {:?}", result.err());

        // Output should be a readable PDF.
        let doc = Document::load(out.path());
        assert!(doc.is_ok(), "output not a valid PDF: {:?}", doc.err());
        let doc = doc.unwrap();
        assert!(doc.get_pages().len() >= 1, "merged PDF should have pages");
    }

    #[test]
    fn merge_empty_inputs_errors() {
        let out = NamedTempFile::new().unwrap();
        let result = merge_pdfs(&[], out.path());
        assert!(result.is_err());
    }

    #[test]
    fn merge_single_pdf_succeeds() {
        let a = write_temp_pdf("Only doc");
        let out = NamedTempFile::new().unwrap();
        let inputs: &[&Path] = &[a.path()];
        let result = merge_pdfs(inputs, out.path());
        assert!(
            result.is_ok(),
            "single-PDF merge should succeed: {:?}",
            result.err()
        );
    }

    // ── Split tests ──────────────────────────────────────────────────────────

    #[test]
    fn split_page1_produces_valid_pdf() {
        let src = write_temp_pdf("Single page document");
        let out = NamedTempFile::new().unwrap();

        let result = split_pdf(src.path(), 1, 1, out.path());
        assert!(result.is_ok(), "split failed: {:?}", result.err());

        // Output must be a loadable PDF.
        let doc = Document::load(out.path());
        assert!(
            doc.is_ok(),
            "split output not a valid PDF: {:?}",
            doc.err()
        );
    }

    #[test]
    fn split_out_of_range_errors() {
        let src = write_temp_pdf("One page");
        let out = NamedTempFile::new().unwrap();

        let result = split_pdf(src.path(), 1, 5, out.path());
        assert!(result.is_err(), "split beyond page count should error");
    }

    // ── Metadata tests ───────────────────────────────────────────────────────

    #[test]
    fn metadata_returns_page_count() {
        let src = write_temp_pdf("Some content");
        let meta = read_metadata(src.path()).unwrap();
        assert_eq!(meta.page_count, 1);
    }

    #[test]
    fn metadata_returns_file_size() {
        let src = write_temp_pdf("Size check");
        let meta = read_metadata(src.path()).unwrap();
        assert!(meta.file_size > 0, "file_size should be > 0");
    }

    // ── Single-page text extraction ──────────────────────────────────────────

    #[test]
    fn extract_page_text_page_1() {
        let src = write_temp_pdf("Hello contract world");
        // May return empty string if lopdf can't decode Type1; ensure no panic/error.
        let result = extract_page_text(src.path(), 1);
        assert!(
            result.is_ok(),
            "extract_page_text should not error: {:?}",
            result.err()
        );
    }

    #[test]
    fn extract_page_text_invalid_page_errors() {
        let src = write_temp_pdf("One page only");
        let result = extract_page_text(src.path(), 99);
        assert!(result.is_err(), "out-of-range page should error");
    }
}
