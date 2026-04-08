use crate::models::FilledField;
use serde_json::json;
use std::path::{Path, PathBuf};

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
