use crate::models::FilledField;
use lopdf::{Document, Object, StringFormat};
use serde_json::json;
use std::path::{Path, PathBuf};

/// Fill AcroForm fields in a PDF by setting their /V (value) entries.
///
/// For PDFs with native form fields, this directly sets the field value
/// so it appears when the PDF is opened in any viewer. For structural
/// fields (no AcroForm), this is a no-op — use the Python engine for
/// text overlay rendering until lopdf content stream writing is added.
pub fn render_filled_pdf(
    src_path: &Path,
    fields: &[FilledField],
    output_dir: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    std::fs::create_dir_all(output_dir)?;
    let dst_path = output_dir.join("filled.pdf");

    let mut doc = Document::load(src_path)?;

    // Fill AcroForm fields by matching widget_name to field values
    let acroform_fields: Vec<&FilledField> = fields.iter()
        .filter(|f| f.source == "acroform" && !f.value.is_empty())
        .collect();

    if !acroform_fields.is_empty() {
        // Walk all objects looking for form field dictionaries
        let object_ids: Vec<lopdf::ObjectId> = doc.objects.keys().cloned().collect();
        for obj_id in object_ids {
            let is_match = {
                if let Ok(Object::Dictionary(dict)) = doc.get_object(obj_id) {
                    if let Ok(Object::String(name_bytes, _)) = dict.get(b"T") {
                        let name = String::from_utf8_lossy(name_bytes).to_string();
                        acroform_fields.iter().any(|f| f.widget_name == name)
                    } else {
                        false
                    }
                } else {
                    false
                }
            };

            if is_match {
                if let Ok(Object::Dictionary(dict)) = doc.get_object(obj_id) {
                    let name = if let Ok(Object::String(nb, _)) = dict.get(b"T") {
                        String::from_utf8_lossy(nb).to_string()
                    } else {
                        continue;
                    };
                    if let Some(field) = acroform_fields.iter().find(|f| f.widget_name == name) {
                        if let Ok(Object::Dictionary(dict_mut)) = doc.get_object_mut(obj_id) {
                            dict_mut.set(
                                b"V",
                                Object::String(field.value.as_bytes().to_vec(), StringFormat::Literal),
                            );
                        }
                    }
                }
            }
        }
    }

    // For structural fields (no AcroForm), append text to page content streams
    let structural_fields: Vec<&FilledField> = fields.iter()
        .filter(|f| f.source == "structural" && !f.value.is_empty())
        .collect();

    if !structural_fields.is_empty() {
        // Group fields by page
        let mut by_page: std::collections::HashMap<usize, Vec<&&FilledField>> = std::collections::HashMap::new();
        for f in &structural_fields {
            by_page.entry(f.page).or_default().push(f);
        }

        let pages: Vec<(u32, lopdf::ObjectId)> = doc.get_pages().into_iter().collect();

        for (page_fields_page, page_fields) in &by_page {
            // Find the page object ID (pages are 1-indexed in lopdf)
            let page_num = (*page_fields_page as u32) + 1;
            if let Some((_, page_id)) = pages.iter().find(|(pn, _)| *pn == page_num) {
                // Get page height for Y coordinate flipping
                let page_height = get_page_height_from_doc(&doc, *page_id);

                // Build content stream for text insertion
                // Use Helvetica (built-in PDF font, no embedding needed)
                let mut text_ops = String::new();
                for field in page_fields {
                    let x = field.bbox.0;
                    // Flip Y: PDF coords are bottom-up, our bbox is top-down
                    let y = page_height - field.bbox.1 - 2.0;
                    let fontsize = auto_fontsize(&field.value, field.bbox.2 - field.bbox.0);

                    text_ops.push_str(&format!(
                        "BT /Helv {} Tf {} {} Td ({}) Tj ET\n",
                        fontsize, x, y,
                        escape_pdf_string(&field.value)
                    ));
                }

                if !text_ops.is_empty() {
                    // Ensure Helvetica is in the page's font resources
                    ensure_helvetica_font(&mut doc, *page_id);

                    // Append our text as a new content stream
                    let stream = lopdf::Stream::new(
                        lopdf::Dictionary::new(),
                        text_ops.into_bytes(),
                    );
                    let stream_id = doc.add_object(Object::Stream(stream));

                    // Add to page's Contents array
                    if let Ok(page_dict) = doc.get_dictionary_mut(*page_id) {
                        match page_dict.get(b"Contents") {
                            Ok(Object::Reference(existing_ref)) => {
                                let existing = *existing_ref;
                                page_dict.set(b"Contents", Object::Array(vec![
                                    Object::Reference(existing),
                                    Object::Reference(stream_id),
                                ]));
                            }
                            Ok(Object::Array(arr)) => {
                                let mut new_arr = arr.clone();
                                new_arr.push(Object::Reference(stream_id));
                                page_dict.set(b"Contents", Object::Array(new_arr));
                            }
                            _ => {
                                page_dict.set(b"Contents", Object::Reference(stream_id));
                            }
                        }
                    }
                }
            }
        }
    }

    doc.save(&dst_path)?;
    Ok(dst_path)
}

fn get_page_height_from_doc(doc: &Document, page_id: lopdf::ObjectId) -> f64 {
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

fn auto_fontsize(value: &str, bbox_width: f64) -> f64 {
    if value.is_empty() { return 9.0; }
    let estimated = value.len() as f64 * 9.0 * 0.5;
    if estimated <= bbox_width { return 9.0; }
    let scaled = 9.0 * (bbox_width / estimated) * 0.95;
    scaled.max(5.0)
}

fn escape_pdf_string(s: &str) -> String {
    s.replace('\\', "\\\\")
     .replace('(', "\\(")
     .replace(')', "\\)")
}

fn ensure_helvetica_font(doc: &mut Document, page_id: lopdf::ObjectId) {
    // Add /Helv as Helvetica to the page's Resources/Font dictionary
    if let Ok(page_dict) = doc.get_dictionary_mut(page_id) {
        let resources = page_dict.get(b"Resources")
            .ok()
            .and_then(|r| if let Object::Dictionary(d) = r { Some(d.clone()) } else { None })
            .unwrap_or_default();

        let mut fonts = resources.get(b"Font")
            .ok()
            .and_then(|f| if let Object::Dictionary(d) = f { Some(d.clone()) } else { None })
            .unwrap_or_default();

        // Only add if /Helv not already there
        if fonts.get(b"Helv").is_err() {
            let mut font_dict = lopdf::Dictionary::new();
            font_dict.set(b"Type", Object::Name(b"Font".to_vec()));
            font_dict.set(b"Subtype", Object::Name(b"Type1".to_vec()));
            font_dict.set(b"BaseFont", Object::Name(b"Helvetica".to_vec()));
            let font_id = doc.add_object(Object::Dictionary(font_dict));
            fonts.set(b"Helv", Object::Reference(font_id));
        }

        let mut new_resources = resources.clone();
        new_resources.set(b"Font", Object::Dictionary(fonts));

        // Need to get mutable ref again since we dropped it
        if let Ok(pd) = doc.get_dictionary_mut(page_id) {
            pd.set(b"Resources", Object::Dictionary(new_resources));
        }
    }
}

/// Write a JSON fill report to `output_dir/fill_report.json`.
///
/// The report contains every field with its resolved value, confidence,
/// and source level, plus a summary tally of green/yellow/red.
///
/// Green  = confidence >= 0.85 (context or high-confidence memory)
/// Yellow = confidence >= 0.50 (memory or inference)
/// Red    = confidence <  0.50 (unknown / unfillable)
pub fn write_fill_report(
    fields: &[FilledField],
    output_dir: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    std::fs::create_dir_all(output_dir)?;
    let report_path = output_dir.join("fill_report.json");

    let green = fields.iter().filter(|f| f.confidence >= 0.85).count();
    let yellow = fields
        .iter()
        .filter(|f| f.confidence >= 0.50 && f.confidence < 0.85)
        .count();
    let red = fields.iter().filter(|f| f.confidence < 0.50).count();

    let report = json!({
        "fields": fields.iter().map(|f| json!({
            "page": f.page,
            "label": f.label,
            "classification": f.classification,
            "value": f.value,
            "source_level": f.source_level,
            "confidence": f.confidence,
            "bbox": [f.bbox.0, f.bbox.1, f.bbox.2, f.bbox.3],
        })).collect::<Vec<_>>(),
        "summary": {
            "total": fields.len(),
            "green": green,
            "yellow": yellow,
            "red": red,
        }
    });

    std::fs::write(&report_path, serde_json::to_string_pretty(&report)?)?;
    Ok(report_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ClassifiedField, DetectedField};
    use tempfile::TempDir;

    fn make_filled(label: &str, value: &str, confidence: f64) -> FilledField {
        let det = DetectedField::new(0, (100.0, 200.0, 300.0, 215.0), label);
        let clf = ClassifiedField::from_detected(&det, "identity.name", 0.9);
        FilledField::from_classified(&clf, value, "context", confidence)
    }

    #[test]
    fn test_write_fill_report_basic() {
        let dir = TempDir::new().unwrap();
        let filled = make_filled("Name", "Hoags Inc.", 1.0);

        let path = write_fill_report(&[filled], dir.path()).unwrap();
        assert!(path.exists());

        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(content["fields"][0]["value"], "Hoags Inc.");
        assert_eq!(content["summary"]["green"], 1);
        assert_eq!(content["summary"]["yellow"], 0);
        assert_eq!(content["summary"]["red"], 0);
    }

    #[test]
    fn test_write_fill_report_tally() {
        let dir = TempDir::new().unwrap();
        let fields = vec![
            make_filled("A", "val_a", 0.95), // green
            make_filled("B", "val_b", 0.70), // yellow
            make_filled("C", "", 0.10),       // red
        ];

        let path = write_fill_report(&fields, dir.path()).unwrap();
        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();

        assert_eq!(content["summary"]["total"], 3);
        assert_eq!(content["summary"]["green"], 1);
        assert_eq!(content["summary"]["yellow"], 1);
        assert_eq!(content["summary"]["red"], 1);
    }

    #[test]
    fn test_write_fill_report_empty() {
        let dir = TempDir::new().unwrap();
        let path = write_fill_report(&[], dir.path()).unwrap();
        assert!(path.exists());

        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(content["summary"]["total"], 0);
        assert!(content["fields"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_report_bbox_serialized() {
        let dir = TempDir::new().unwrap();
        let filled = make_filled("Name", "Hoags Inc.", 0.9);
        let path = write_fill_report(&[filled], dir.path()).unwrap();

        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let bbox = &content["fields"][0]["bbox"];
        assert_eq!(bbox[0], 100.0);
        assert_eq!(bbox[1], 200.0);
        assert_eq!(bbox[2], 300.0);
        assert_eq!(bbox[3], 215.0);
    }
}
