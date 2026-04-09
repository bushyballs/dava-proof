use crate::models::DetectedField;
use lopdf::{Document, Object};
use std::path::Path;

/// A line segment from PDF drawing operators.
#[derive(Debug, Clone)]
struct LineSegment {
    x0: f64, y0: f64, x1: f64, y1: f64,
}

/// A rectangle from PDF drawing operators.
#[derive(Debug, Clone)]
struct PdfRect {
    x: f64, y: f64, w: f64, h: f64,
}

/// Detect AcroForm fields from the PDF's interactive form dictionary.
fn detect_acroform(doc: &Document) -> Vec<DetectedField> {
    let mut fields = Vec::new();

    let catalog = match doc.catalog() {
        Ok(c) => c,
        Err(_) => return fields,
    };

    let acroform_obj = match catalog.get(b"AcroForm") {
        Ok(obj) => obj.clone(),
        Err(_) => return fields,
    };

    let acroform_dict = match doc.dereference(&acroform_obj) {
        Ok((_, Object::Dictionary(d))) => d,
        _ => return fields,
    };

    let field_refs = match acroform_dict.get(b"Fields") {
        Ok(Object::Array(arr)) => arr.clone(),
        _ => return fields,
    };

    for field_ref in &field_refs {
        if let Ok((_, Object::Dictionary(field_dict))) = doc.dereference(field_ref) {
            let field_name = match field_dict.get(b"T") {
                Ok(Object::String(bytes, _)) => {
                    String::from_utf8(bytes.clone()).unwrap_or_default()
                }
                _ => String::new(),
            };

            let rect = match field_dict.get(b"Rect") {
                Ok(Object::Array(arr)) if arr.len() == 4 => {
                    let nums: Vec<f64> = arr
                        .iter()
                        .filter_map(|o| match o {
                            Object::Real(f) => Some(*f as f64),
                            Object::Integer(i) => Some(*i as f64),
                            _ => None,
                        })
                        .collect();
                    if nums.len() == 4 { Some((nums[0], nums[1], nums[2], nums[3])) } else { None }
                }
                _ => None,
            };

            if let Some(bbox) = rect {
                let mut det = DetectedField::new(0, bbox, &field_name);
                det.source = "acroform".to_string();
                det.widget_name = field_name.clone();

                if let Ok(Object::Name(ft)) = field_dict.get(b"FT") {
                    if ft == b"Btn" { det.field_type = "checkbox".to_string(); }
                    if ft == b"Sig" { det.field_type = "signature".to_string(); }
                }

                fields.push(det);
            }
        }
    }

    fields
}

/// Parse raw content stream bytes for line (m/l) and rectangle (re) operators.
fn parse_drawing_ops(content: &[u8], page_height: f64) -> (Vec<LineSegment>, Vec<PdfRect>) {
    let mut lines = Vec::new();
    let mut rects = Vec::new();
    let content_str = String::from_utf8_lossy(content);
    let tokens: Vec<&str> = content_str.split_whitespace().collect();

    let mut cur_x: f64 = 0.0;
    let mut cur_y: f64 = 0.0;

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
                    x0: cur_x, y0: page_height - cur_y,
                    x1: lx, y1: page_height - ly,
                });
                cur_x = lx;
                cur_y = ly;
            }
            "re" if i >= 4 => {
                let rx: f64 = tokens[i - 4].parse().unwrap_or(0.0);
                let ry: f64 = tokens[i - 3].parse().unwrap_or(0.0);
                let rw: f64 = tokens[i - 2].parse().unwrap_or(0.0);
                let rh: f64 = tokens[i - 1].parse().unwrap_or(0.0);
                rects.push(PdfRect {
                    x: rx, y: page_height - ry - rh.abs(),
                    w: rw.abs(), h: rh.abs(),
                });
            }
            _ => {}
        }
    }
    (lines, rects)
}

/// Get page content bytes (handles Reference, Array, and inline Stream).
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
        Ok(Object::Stream(stream)) => {
            let mut s = stream.clone();
            let _ = s.decompress();
            s.content
        }
        _ => Vec::new(),
    }
}

fn get_page_height(doc: &Document, page_id: lopdf::ObjectId) -> f64 {
    if let Ok(d) = doc.get_dictionary(page_id) {
        if let Ok(Object::Array(arr)) = d.get(b"MediaBox") {
            if arr.len() == 4 {
                match &arr[3] {
                    Object::Real(h) => return *h as f64,
                    Object::Integer(h) => return *h as f64,
                    _ => {}
                }
            }
        }
    }
    792.0
}

/// Structural detection: parses content streams for lines + rects, plus text "Label:" heuristics.
fn detect_structural(doc: &Document) -> Vec<DetectedField> {
    let mut fields = Vec::new();

    // Only extract text from first 15 pages (form fields are almost never later).
    // This is the expensive operation — lopdf decodes content streams for text.
    let max_text_pages = 15;
    let mut page_texts: Vec<(usize, String)> = Vec::new();
    for (page_num, _) in doc.get_pages() {
        if page_num as usize > max_text_pages { break; }
        let text = doc.extract_text(&[page_num]).unwrap_or_default();
        page_texts.push(((page_num as usize).saturating_sub(1), text));
    }

    // Parse content streams for drawing operators (first 20 pages only for speed)
    let max_draw_pages = 20;
    for (page_num, page_id) in doc.get_pages() {
        if page_num as usize > max_draw_pages { break; }
        let page_idx = (page_num as usize).saturating_sub(1);
        let page_height = get_page_height(doc, page_id);
        let content = get_page_content(doc, page_id);
        if content.is_empty() { continue; }

        let (h_lines, rects) = parse_drawing_ops(&content, page_height);

        // Horizontal lines → potential underscore blanks
        // Filter: must be >30pt wide, <3pt tall, not a page border (not at edges),
        // and not full-page-width (borders are usually 0..612 or similar)
        for line in &h_lines {
            let dx = (line.x1 - line.x0).abs();
            let dy = (line.y1 - line.y0).abs();
            let min_x = line.x0.min(line.x1);
            let max_x = line.x0.max(line.x1);

            if dx > 30.0 && dx < 500.0 && dy < 3.0
                && min_x > 10.0  // not at left page edge
                && line.y0 > 20.0 && line.y0 < (page_height - 20.0) // not at top/bottom
            {
                fields.push(DetectedField {
                    page: page_idx,
                    bbox: (min_x, line.y0 - 12.0, max_x, line.y0 + 2.0),
                    label: String::new(),
                    field_type: "text".to_string(),
                    source: "structural".to_string(),
                    widget_name: String::new(),
                });
            }
        }

        // Small squares → checkboxes
        for rect in &rects {
            if rect.w > 6.0 && rect.w < 20.0 && rect.h > 6.0 && rect.h < 20.0
                && (rect.w - rect.h).abs() < 5.0
            {
                fields.push(DetectedField {
                    page: page_idx,
                    bbox: (rect.x, rect.y, rect.x + rect.w, rect.y + rect.h),
                    label: String::new(),
                    field_type: "checkbox".to_string(),
                    source: "structural".to_string(),
                    widget_name: String::new(),
                });
            }
        }
    }

    // Text-based "Label:" detection (always runs — catches what content stream misses)
    // Handles both single labels ("Name:") and concatenated labels ("Name:  Address:  Phone:")
    for (page_idx, text) in &page_texts {
        for text_line in text.lines() {
            let trimmed = text_line.trim();

            // Split on ":  " (colon + 2+ spaces) to handle concatenated fields
            // "Offeror Name:  Offeror Address:  Offeror Phone:" → 3 separate labels
            let segments: Vec<&str> = trimmed.split(":").collect();

            for (seg_idx, segment) in segments.iter().enumerate() {
                let label = segment.trim();
                // Skip empty segments and the last empty one after trailing ":"
                if label.is_empty() || label.len() > 60 {
                    continue;
                }
                // Must look like a field label: not too long, contains letters
                if !label.chars().any(|c| c.is_alphabetic()) {
                    continue;
                }
                // Skip if it looks like a sentence (has periods or is very long)
                if label.contains('.') && label.len() > 30 {
                    continue;
                }

                let already = fields.iter().any(|f| f.page == *page_idx && f.label == label);
                if !already {
                    fields.push(DetectedField {
                        page: *page_idx,
                        bbox: (100.0, 0.0, 400.0, 12.0),
                        label: label.to_string(),
                        field_type: "text".to_string(),
                        source: "structural".to_string(),
                        widget_name: String::new(),
                    });
                }
            }
        }
    }

    fields
}

/// Detect all fillable fields in a PDF using a tiered approach.
///
/// Tier 1: AcroForm interactive fields (instant, exact positions).
/// Tier 2: Structural heuristics from text content streams.
pub fn detect_all_fields(pdf_path: &Path) -> Vec<DetectedField> {
    let doc = match Document::load(pdf_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!(
                "Failed to load PDF {}: {}",
                pdf_path.display(),
                e
            );
            return Vec::new();
        }
    };

    // Tier 1: AcroForm fields
    let acro = detect_acroform(&doc);
    if !acro.is_empty() {
        return acro;
    }

    // Tier 2: Structural text analysis
    detect_structural(&doc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_nonexistent_pdf() {
        let fields = detect_all_fields(Path::new("nonexistent_file_that_does_not_exist.pdf"));
        assert!(fields.is_empty());
    }

    #[test]
    fn test_parse_horizontal_line() {
        let content = b"100 500 m 300 500 l S";
        let (lines, _) = parse_drawing_ops(content, 792.0);
        assert_eq!(lines.len(), 1);
        assert!((lines[0].x0 - 100.0).abs() < 0.1);
        assert!((lines[0].x1 - 300.0).abs() < 0.1);
        // Y should be flipped: 792 - 500 = 292
        assert!((lines[0].y0 - 292.0).abs() < 0.1);
    }

    #[test]
    fn test_parse_rectangle() {
        let content = b"50 700 12 12 re S";
        let (_, rects) = parse_drawing_ops(content, 792.0);
        assert_eq!(rects.len(), 1);
        assert!((rects[0].w - 12.0).abs() < 0.1);
        assert!((rects[0].h - 12.0).abs() < 0.1);
    }

    #[test]
    fn test_checkbox_from_small_square() {
        let content = b"50 700 10 10 re S";
        let (_, rects) = parse_drawing_ops(content, 792.0);
        assert!(!rects.is_empty());
        let r = &rects[0];
        // Should qualify as checkbox: 6 < w < 20, 6 < h < 20, roughly square
        assert!(r.w > 6.0 && r.w < 20.0);
        assert!(r.h > 6.0 && r.h < 20.0);
        assert!((r.w - r.h).abs() < 5.0);
    }

    #[test]
    fn test_large_rect_not_checkbox() {
        let content = b"50 700 200 50 re S";
        let (_, rects) = parse_drawing_ops(content, 792.0);
        assert!(!rects.is_empty());
        let r = &rects[0];
        // 200x50 is NOT a checkbox
        assert!(!(r.w > 6.0 && r.w < 20.0 && r.h > 6.0 && r.h < 20.0));
    }

    #[test]
    fn test_multiple_lines_and_rects() {
        let content = b"100 500 m 300 500 l S 50 700 10 10 re S 200 400 m 400 400 l S";
        let (lines, rects) = parse_drawing_ops(content, 792.0);
        assert_eq!(lines.len(), 2);
        assert_eq!(rects.len(), 1);
    }
}
