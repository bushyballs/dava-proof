use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedField {
    pub page: usize,
    pub bbox: (f64, f64, f64, f64), // x0, y0, x1, y1
    pub label: String,
    pub field_type: String,  // "text", "checkbox", "signature", "date", "currency"
    pub source: String,      // "acroform", "structural", "vision"
    pub widget_name: String,
}

impl DetectedField {
    pub fn new(page: usize, bbox: (f64, f64, f64, f64), label: &str) -> Self {
        Self {
            page,
            bbox,
            label: label.to_string(),
            field_type: "text".to_string(),
            source: "structural".to_string(),
            widget_name: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifiedField {
    pub page: usize,
    pub bbox: (f64, f64, f64, f64),
    pub label: String,
    pub field_type: String,
    pub source: String,
    pub widget_name: String,
    pub classification: String,
    pub confidence: f64,
}

impl ClassifiedField {
    pub fn from_detected(det: &DetectedField, classification: &str, confidence: f64) -> Self {
        Self {
            page: det.page,
            bbox: det.bbox,
            label: det.label.clone(),
            field_type: det.field_type.clone(),
            source: det.source.clone(),
            widget_name: det.widget_name.clone(),
            classification: classification.to_string(),
            confidence,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilledField {
    pub page: usize,
    pub bbox: (f64, f64, f64, f64),
    pub label: String,
    pub field_type: String,
    pub source: String,
    pub widget_name: String,
    pub classification: String,
    pub value: String,
    pub source_level: String, // "context", "dava_memory", "none"
    pub confidence: f64,
}

impl FilledField {
    pub fn from_classified(clf: &ClassifiedField, value: &str, source_level: &str, confidence: f64) -> Self {
        Self {
            page: clf.page,
            bbox: clf.bbox,
            label: clf.label.clone(),
            field_type: clf.field_type.clone(),
            source: clf.source.clone(),
            widget_name: clf.widget_name.clone(),
            classification: clf.classification.clone(),
            value: value.to_string(),
            source_level: source_level.to_string(),
            confidence,
        }
    }
}
