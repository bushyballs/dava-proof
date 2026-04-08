use crate::memory::FieldMemory;
use crate::models::{ClassifiedField, DetectedField};
use regex::Regex;

struct Rule {
    pattern: Regex,
    classification: &'static str,
    confidence: f64,
}

fn build_rules() -> Vec<Rule> {
    vec![
        Rule {
            pattern: Regex::new(r"(?i)\b(cage)\b").unwrap(),
            classification: "identity.code",
            confidence: 0.95,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(uei)\b").unwrap(),
            classification: "identity.code",
            confidence: 0.95,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(duns)\b").unwrap(),
            classification: "identity.code",
            confidence: 0.95,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(tin|ein|tax.?id)\b").unwrap(),
            classification: "identity.code",
            confidence: 0.95,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(phone|tel(?:ephone)?|fax|mobile|cell)\b").unwrap(),
            classification: "identity.phone",
            confidence: 0.90,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(email|e-mail)\b").unwrap(),
            classification: "identity.email",
            confidence: 0.95,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(address|street|city|state|zip|postal)\b").unwrap(),
            classification: "identity.address",
            confidence: 0.90,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(signature|sign)\b").unwrap(),
            classification: "signature",
            confidence: 0.90,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(point of contact|poc)\b").unwrap(),
            classification: "identity.name",
            confidence: 0.90,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(date|dated)\b").unwrap(),
            classification: "temporal.date",
            confidence: 0.90,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(price|amount|total|cost|\$|dollar)\b").unwrap(),
            classification: "currency",
            confidence: 0.90,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(quantity|qty|count)\b").unwrap(),
            classification: "numeric",
            confidence: 0.85,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(describe|explain|narrative|experience)\b").unwrap(),
            classification: "essay",
            confidence: 0.80,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(name|offeror|contractor|company|vendor|firm)\b").unwrap(),
            classification: "identity.name",
            confidence: 0.95,
        },
    ]
}

/// Classify a single detected field using memory lookup then rule-based matching.
pub fn classify_field(field: &DetectedField, memory: &FieldMemory) -> ClassifiedField {
    // Checkbox / signature type overrides — no ambiguity
    if field.field_type == "checkbox" {
        return ClassifiedField::from_detected(field, "checkbox", 0.95);
    }
    if field.field_type == "signature" {
        return ClassifiedField::from_detected(field, "signature", 0.90);
    }

    // DAVA memory lookup (highest priority after type overrides)
    if let Some(hit) = memory.recall(&field.label) {
        return ClassifiedField::from_detected(field, &hit.classification, hit.confidence);
    }

    // Rule-based: normalize punctuation to spaces for easier matching
    let label = format!("{} {}", field.label, field.widget_name)
        .replace('_', " ")
        .replace('-', " ");

    let rules = build_rules();
    for rule in &rules {
        if rule.pattern.is_match(&label) {
            return ClassifiedField::from_detected(field, rule.classification, rule.confidence);
        }
    }

    ClassifiedField::from_detected(field, "unknown", 0.3)
}

/// Classify a slice of detected fields.
pub fn classify_fields(fields: &[DetectedField], memory: &FieldMemory) -> Vec<ClassifiedField> {
    fields.iter().map(|f| classify_field(f, memory)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn tmp_memory() -> FieldMemory {
        let tmp = NamedTempFile::new().unwrap();
        FieldMemory::open(tmp.path()).unwrap()
    }

    #[test]
    fn test_classify_name() {
        let mem = tmp_memory();
        let f = DetectedField::new(0, (0.0, 0.0, 100.0, 20.0), "Offeror Name");
        let c = classify_field(&f, &mem);
        assert_eq!(c.classification, "identity.name");
    }

    #[test]
    fn test_classify_phone() {
        let mem = tmp_memory();
        let f = DetectedField::new(0, (0.0, 0.0, 100.0, 20.0), "Offeror Telephone");
        let c = classify_field(&f, &mem);
        assert_eq!(c.classification, "identity.phone");
    }

    #[test]
    fn test_classify_checkbox() {
        let mem = tmp_memory();
        let mut f = DetectedField::new(0, (0.0, 0.0, 20.0, 20.0), "Agree");
        f.field_type = "checkbox".to_string();
        let c = classify_field(&f, &mem);
        assert_eq!(c.classification, "checkbox");
        assert!(c.confidence >= 0.9);
    }

    #[test]
    fn test_classify_unknown() {
        let mem = tmp_memory();
        let f = DetectedField::new(0, (0.0, 0.0, 100.0, 20.0), "Zygomorphic Coefficient");
        let c = classify_field(&f, &mem);
        assert_eq!(c.classification, "unknown");
        assert!(c.confidence < 0.5);
    }

    #[test]
    fn test_memory_wins_over_rules() {
        let mem = tmp_memory();
        mem.store(
            "offeror telephone",
            "identity.fax",
            "555-0100",
            "identity.fax",
            "form.pdf",
            true,
        );
        let f = DetectedField::new(0, (0.0, 0.0, 100.0, 20.0), "Offeror Telephone");
        let c = classify_field(&f, &mem);
        // Memory overrides the regex rule that would say identity.phone
        assert_eq!(c.classification, "identity.fax");
    }
}
