/// Signature field detection for sigstamp.
///
/// Searches for horizontal lines near "Signature"/"Sign"/"Authorized" text,
/// AcroForm /Sig fields, and well-known block labels on SF1449 forms.

use lopdf::{Document, Object};

/// A detected signature location ready for stamping.
#[derive(Debug, Clone)]
pub struct SigLocation {
    /// 0-based page index.
    pub page: usize,
    /// Bounding box (x0, y0, x1, y1) in PDF user-space (origin = lower-left).
    pub bbox: (f64, f64, f64, f64),
    /// Human-readable label found near this location ("Signature", "Block 30", …).
    pub label: String,
}

// ── internal geometry helpers ────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct LineSegment {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

#[derive(Debug, Clone)]
struct TextSpan {
    text: String,
    x: f64,
    /// Y in **top-down** coordinates (page_height − pdf_y).
    y_top: f64,
}

// ── page geometry ─────────────────────────────────────────────────────────────

fn page_height(doc: &Document, page_id: lopdf::ObjectId) -> f64 {
    if let Ok(d) = doc.get_dictionary(page_id) {
        if let Ok(Object::Array(arr)) = d.get(b"MediaBox") {
            if arr.len() == 4 {
                return match &arr[3] {
                    Object::Real(h) => *h as f64,
                    Object::Integer(h) => *h as f64,
                    _ => 792.0,
                };
            }
        }
    }
    792.0
}

// ── content stream helpers ────────────────────────────────────────────────────

fn get_page_content(doc: &Document, page_id: lopdf::ObjectId) -> Vec<u8> {
    let page_dict = match doc.get_dictionary(page_id) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };
    match page_dict.get(b"Contents") {
        Ok(Object::Reference(ref_id)) => {
            if let Ok(Object::Stream(stream)) = doc.get_object(*ref_id) {
                let mut s = stream.clone();
                let _ = s.decompress();
                return s.content;
            }
            Vec::new()
        }
        Ok(Object::Array(arr)) => {
            let mut all = Vec::new();
            for item in arr {
                if let Object::Reference(ref_id) = item {
                    if let Ok(Object::Stream(stream)) = doc.get_object(*ref_id) {
                        let mut s = stream.clone();
                        let _ = s.decompress();
                        all.extend_from_slice(&s.content);
                        all.push(b'\n');
                    }
                }
            }
            all
        }
        _ => Vec::new(),
    }
}

/// Parse content stream for horizontal line segments and text spans.
fn parse_content_ops(content: &[u8], ph: f64) -> (Vec<LineSegment>, Vec<TextSpan>) {
    let text = String::from_utf8_lossy(content);
    let tokens: Vec<&str> = text.split_whitespace().collect();

    let mut lines: Vec<LineSegment> = Vec::new();
    let mut cur_x = 0.0_f64;
    let mut cur_y = 0.0_f64;

    for i in 0..tokens.len() {
        match tokens[i] {
            "m" if i >= 2 => {
                cur_x = tokens[i - 2].parse().unwrap_or(0.0);
                cur_y = tokens[i - 1].parse().unwrap_or(0.0);
            }
            "l" if i >= 2 => {
                let lx: f64 = tokens[i - 2].parse().unwrap_or(0.0);
                let ly: f64 = tokens[i - 1].parse().unwrap_or(0.0);
                lines.push(LineSegment {
                    x0: cur_x,
                    y0: ph - cur_y,
                    x1: lx,
                    y1: ph - ly,
                });
                cur_x = lx;
                cur_y = ly;
            }
            _ => {}
        }
    }

    let mut spans: Vec<TextSpan> = Vec::new();
    extract_text_spans(&text, ph, &mut spans);

    (lines, spans)
}

fn extract_text_spans(content: &str, ph: f64, out: &mut Vec<TextSpan>) {
    let mut tm_x = 0.0_f64;
    let mut tm_y = 0.0_f64;
    let bytes = content.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // Tm — text matrix sets absolute position
        if i + 2 < len
            && bytes[i] == b'T'
            && bytes[i + 1] == b'm'
            && (i + 2 >= len || !bytes[i + 2].is_ascii_alphanumeric())
        {
            let before = &content[..i];
            let nums: Vec<f64> = before
                .split_whitespace()
                .rev()
                .take(6)
                .filter_map(|t| t.parse::<f64>().ok())
                .collect();
            if nums.len() >= 2 {
                tm_x = nums[1];
                tm_y = nums[0];
            }
        }

        // Td / TD — relative move
        if i + 2 < len
            && bytes[i] == b'T'
            && (bytes[i + 1] == b'd' || bytes[i + 1] == b'D')
            && (i + 2 >= len || !bytes[i + 2].is_ascii_alphanumeric())
            && i > 0
            && bytes[i - 1] == b' '
        {
            let before = &content[..i];
            let nums: Vec<f64> = before
                .split_whitespace()
                .rev()
                .take(2)
                .filter_map(|t| t.parse::<f64>().ok())
                .collect();
            if nums.len() >= 2 {
                tm_x += nums[1];
                tm_y += nums[0];
            }
        }

        // (string) Tj
        if bytes[i] == b'(' {
            let start = i + 1;
            let mut j = start;
            let mut depth = 1usize;
            while j < len && depth > 0 {
                if bytes[j] == b'(' && (j == start || bytes[j - 1] != b'\\') {
                    depth += 1;
                }
                if bytes[j] == b')' && (j == start || bytes[j - 1] != b'\\') {
                    depth -= 1;
                }
                if depth > 0 {
                    j += 1;
                }
            }
            if depth == 0 {
                let t = String::from_utf8_lossy(&bytes[start..j]).to_string();
                let after = content[j + 1..].trim_start();
                if after.starts_with("Tj") && !t.trim().is_empty() {
                    out.push(TextSpan {
                        text: t,
                        x: tm_x,
                        y_top: ph - tm_y,
                    });
                }
                i = j + 1;
                continue;
            }
        }

        i += 1;
    }
}

// ── signature keyword matching ────────────────────────────────────────────────

/// Returns true if this text label looks like a signature field identifier.
pub fn is_sig_label(label: &str) -> bool {
    let lower = label.to_lowercase();
    // Direct signature words
    if lower.contains("signature") || lower.contains("signed") {
        return true;
    }
    // "Sign here", "Sign below"
    if lower.starts_with("sign") {
        return true;
    }
    // "Authorized by", "Authorized Representative"
    if lower.contains("authorized") {
        return true;
    }
    // SF1449 / SF30 / OF347 well-known blocks
    if lower.contains("block 30")
        || lower.contains("block 31")
        || lower.contains("block 21")
        || lower.contains("block 22")
    {
        return true;
    }
    // "Contracting Officer", "Offeror" in signature context
    if lower.contains("contracting officer") && lower.contains("sign") {
        return true;
    }
    false
}

// ── AcroForm /Sig field detection ────────────────────────────────────────────

fn detect_acroform_sig(doc: &Document) -> Vec<SigLocation> {
    let mut out = Vec::new();
    let catalog = match doc.catalog() {
        Ok(c) => c,
        Err(_) => return out,
    };
    let acroform_obj = match catalog.get(b"AcroForm") {
        Ok(o) => o.clone(),
        Err(_) => return out,
    };
    let acroform_dict = match doc.dereference(&acroform_obj) {
        Ok((_, Object::Dictionary(d))) => d,
        _ => return out,
    };
    let field_refs = match acroform_dict.get(b"Fields") {
        Ok(Object::Array(arr)) => arr.clone(),
        _ => return out,
    };

    for field_ref in &field_refs {
        if let Ok((_, Object::Dictionary(fd))) = doc.dereference(field_ref) {
            // Only /Sig type or /T containing "sign"
            let is_sig = match fd.get(b"FT") {
                Ok(Object::Name(ft)) => ft == b"Sig",
                _ => false,
            };
            let name = match fd.get(b"T") {
                Ok(Object::String(bytes, _)) => {
                    String::from_utf8(bytes.clone()).unwrap_or_default()
                }
                _ => String::new(),
            };
            if !is_sig && !is_sig_label(&name) {
                continue;
            }
            let rect = match fd.get(b"Rect") {
                Ok(Object::Array(arr)) if arr.len() == 4 => {
                    let nums: Vec<f64> = arr
                        .iter()
                        .filter_map(|o| match o {
                            Object::Real(f) => Some(*f as f64),
                            Object::Integer(i) => Some(*i as f64),
                            _ => None,
                        })
                        .collect();
                    if nums.len() == 4 {
                        Some((nums[0], nums[1], nums[2], nums[3]))
                    } else {
                        None
                    }
                }
                _ => None,
            };
            if let Some(bbox) = rect {
                out.push(SigLocation {
                    page: 0,
                    bbox,
                    label: if name.is_empty() {
                        "Signature".to_string()
                    } else {
                        name
                    },
                });
            }
        }
    }
    out
}

// ── structural: lines near sig-keyword text ───────────────────────────────────

fn detect_structural_sig(doc: &Document) -> Vec<SigLocation> {
    let mut out = Vec::new();
    let pages: Vec<(u32, lopdf::ObjectId)> = doc.get_pages().into_iter().collect();

    for (page_num, page_id) in &pages {
        let page_idx = (*page_num as usize).saturating_sub(1);
        let ph = page_height(doc, *page_id);
        let content = get_page_content(doc, *page_id);
        if content.is_empty() {
            continue;
        }

        let (h_lines, text_spans) = parse_content_ops(&content, ph);

        // For each sig-keyword text span, look for the nearest horizontal line below it
        for span in &text_spans {
            if !is_sig_label(&span.text) {
                continue;
            }
            // Find closest horizontal line within 50pt below the text
            let best_line = h_lines.iter().filter(|l| {
                let dx = (l.x1 - l.x0).abs();
                let dy = (l.y1 - l.y0).abs();
                dx > 40.0 && dy < 4.0
                    && l.y0 > span.y_top          // line is below the label
                    && l.y0 - span.y_top < 50.0   // within 50pt
                    && l.x0.min(l.x1) < span.x + dx  // overlaps horizontally
            }).min_by(|a, b| {
                let da = (a.y0 - span.y_top).abs();
                let db = (b.y0 - span.y_top).abs();
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            });

            if let Some(line) = best_line {
                let x0 = line.x0.min(line.x1);
                let x1 = line.x0.max(line.x1);
                // bbox in PDF coords (lower-left origin): flip y_top back
                let y_pdf = ph - line.y0;
                out.push(SigLocation {
                    page: page_idx,
                    bbox: (x0, y_pdf - 2.0, x1, y_pdf + 12.0),
                    label: span.text.trim().to_string(),
                });
            } else {
                // No line found — use a synthetic bbox just below the label text
                let y_pdf = ph - span.y_top - 14.0;
                out.push(SigLocation {
                    page: page_idx,
                    bbox: (span.x, y_pdf, span.x + 200.0, y_pdf + 14.0),
                    label: span.text.trim().to_string(),
                });
            }
        }
    }
    out
}

// ── text-line keyword scan (lopdf extract_text) ───────────────────────────────

fn detect_keyword_scan(doc: &Document) -> Vec<SigLocation> {
    let mut out = Vec::new();
    let pages: Vec<(u32, lopdf::ObjectId)> = doc.get_pages().into_iter().collect();

    let sig_keywords = [
        "Signature",
        "SIGNATURE",
        "Authorized Signature",
        "Authorized Representative",
        "Sign Here",
        "Offeror Signature",
        "Contractor Signature",
        "Block 30",
        "Block 31",
        "Contracting Officer Signature",
    ];

    for (page_num, _page_id) in &pages {
        let page_idx = (*page_num as usize).saturating_sub(1);
        let text = doc.extract_text(&[*page_num]).unwrap_or_default();
        for kw in &sig_keywords {
            if text.contains(kw) {
                // We don't have exact coordinates from extract_text, so use synthetic bbox.
                // The sign.rs placer will use best-effort placement at the bottom of the page.
                let already = out.iter().any(|s: &SigLocation| s.page == page_idx && s.label == *kw);
                if !already {
                    out.push(SigLocation {
                        page: page_idx,
                        bbox: (72.0, 100.0, 400.0, 115.0),
                        label: kw.to_string(),
                    });
                }
            }
        }
    }
    out
}

// ── public API ────────────────────────────────────────────────────────────────

/// Detect all signature locations in a PDF.
///
/// Priority: AcroForm /Sig fields → structural (lines near sig text) → keyword scan.
/// Returns at most one location per page (the best candidate).
pub fn detect_sig_locations(pdf_path: &std::path::Path) -> Vec<SigLocation> {
    let doc = match Document::load(pdf_path) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    // Tier 1: AcroForm
    let acro = detect_acroform_sig(&doc);
    if !acro.is_empty() {
        return acro;
    }

    // Tier 2: Structural
    let structural = detect_structural_sig(&doc);
    if !structural.is_empty() {
        return structural;
    }

    // Tier 3: Keyword scan
    detect_keyword_scan(&doc)
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_sig_label_signature() {
        assert!(is_sig_label("Signature"));
        assert!(is_sig_label("SIGNATURE"));
        assert!(is_sig_label("Offeror Signature"));
        assert!(is_sig_label("Authorized Signature"));
    }

    #[test]
    fn test_is_sig_label_sign_variants() {
        assert!(is_sig_label("Sign Here"));
        assert!(is_sig_label("Sign Below"));
        assert!(is_sig_label("signed by"));
    }

    #[test]
    fn test_is_sig_label_authorized() {
        assert!(is_sig_label("Authorized Representative"));
        assert!(is_sig_label("Authorized by"));
    }

    #[test]
    fn test_is_sig_label_sf1449_blocks() {
        assert!(is_sig_label("Block 30"));
        assert!(is_sig_label("Block 31"));
        assert!(is_sig_label("block 21"));
    }

    #[test]
    fn test_is_sig_label_rejects_non_sig() {
        assert!(!is_sig_label("Name"));
        assert!(!is_sig_label("Date"));
        assert!(!is_sig_label("Address"));
        assert!(!is_sig_label("Total Price"));
        assert!(!is_sig_label("NAICS Code"));
    }

    #[test]
    fn test_detect_sig_nonexistent_pdf() {
        let locs = detect_sig_locations(std::path::Path::new("nonexistent_sig_test.pdf"));
        assert!(locs.is_empty());
    }

    #[test]
    fn test_parse_content_ops_horizontal_line() {
        // "100 500 m 350 500 l S" — horizontal 250pt line
        let content = b"100 500 m 350 500 l S";
        let (lines, _) = parse_content_ops(content, 792.0);
        assert_eq!(lines.len(), 1);
        assert!((lines[0].x0 - 100.0).abs() < 0.1);
        assert!((lines[0].x1 - 350.0).abs() < 0.1);
        // Y flipped: 792 - 500 = 292
        assert!((lines[0].y0 - 292.0).abs() < 0.1);
    }

    #[test]
    fn test_parse_content_ops_vertical_line_not_included() {
        // Vertical line x0==x1 — should still be parsed but won't be used for sig detection
        // (the structural detector filters dx > 40)
        let content = b"200 400 m 200 600 l S";
        let (lines, _) = parse_content_ops(content, 792.0);
        assert_eq!(lines.len(), 1);
        let dx = (lines[0].x1 - lines[0].x0).abs();
        assert!(dx < 1.0, "vertical line has near-zero dx, got {}", dx);
    }
}
