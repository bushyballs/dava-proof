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

    doc.save(&dst_path)?;
    Ok(dst_path)
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
