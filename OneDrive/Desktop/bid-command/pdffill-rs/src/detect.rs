use crate::models::DetectedField;
use lopdf::Document;
use std::path::Path;

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
        Ok((_, lopdf::Object::Dictionary(d))) => d,
        _ => return fields,
    };

    let field_refs = match acroform_dict.get(b"Fields") {
        Ok(lopdf::Object::Array(arr)) => arr.clone(),
        _ => return fields,
    };

    for field_ref in &field_refs {
        if let Ok((_, lopdf::Object::Dictionary(field_dict))) = doc.dereference(field_ref) {
            let field_name = match field_dict.get(b"T") {
                Ok(lopdf::Object::String(bytes, _)) => {
                    String::from_utf8(bytes.clone()).unwrap_or_default()
                }
                _ => String::new(),
            };

            // Get the widget rectangle
            let rect = match field_dict.get(b"Rect") {
                Ok(lopdf::Object::Array(arr)) if arr.len() == 4 => {
                    let nums: Vec<f64> = arr
                        .iter()
                        .filter_map(|o| match o {
                            lopdf::Object::Real(f) => Some(*f as f64),
                            lopdf::Object::Integer(i) => Some(*i as f64),
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
                let mut det = DetectedField::new(0, bbox, &field_name);
                det.source = "acroform".to_string();
                det.widget_name = field_name.clone();

                // Check field type (FT key): Tx=text, Btn=checkbox/radio, Ch=choice
                if let Ok(lopdf::Object::Name(ft)) = field_dict.get(b"FT") {
                    let ft_str = String::from_utf8_lossy(ft);
                    if ft_str == "Btn" {
                        det.field_type = "checkbox".to_string();
                    }
                }

                fields.push(det);
            }
        }
    }

    fields
}

/// Detect fields from page text content using "Label:" pattern heuristics.
fn detect_structural(doc: &Document) -> Vec<DetectedField> {
    let mut fields = Vec::new();

    for (page_num, _page_id) in doc.get_pages() {
        let page_idx = (page_num as usize).saturating_sub(1);

        let text = doc.extract_text(&[page_num]).unwrap_or_default();

        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.ends_with(':') && trimmed.len() > 2 && trimmed.len() < 60 {
                let label = trimmed.trim_end_matches(':').trim();
                if !label.is_empty() {
                    let det = DetectedField {
                        page: page_idx,
                        bbox: (100.0, 0.0, 400.0, 12.0),
                        label: label.to_string(),
                        field_type: "text".to_string(),
                        source: "structural".to_string(),
                        widget_name: String::new(),
                    };
                    fields.push(det);
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
}
