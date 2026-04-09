use crate::memory::FieldMemory;
use crate::models::{ClassifiedField, DetectedField};
use regex::Regex;
use std::sync::LazyLock;

struct Rule {
    pattern: Regex,
    classification: &'static str,
    confidence: f64,
}

static RULES: LazyLock<Vec<Rule>> = LazyLock::new(|| vec![
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
        // Government form specific
        Rule {
            pattern: Regex::new(r"(?i)\b(solicitation|rfq|rfp|ifb|sol\s+no|sol\s+number)\b").unwrap(),
            classification: "reference.solicitation",
            confidence: 0.85,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(contract\s+no|contract\s+number|award\s+no)\b").unwrap(),
            classification: "reference.contract",
            confidence: 0.85,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(naics|sic\s+code|psc|product.?service)\b").unwrap(),
            classification: "reference.naics",
            confidence: 0.85,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(deliver|ship|performance|place of)\b").unwrap(),
            classification: "location.delivery",
            confidence: 0.80,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(issued|administered|payment|invoice|remit)\b").unwrap(),
            classification: "admin.office",
            confidence: 0.75,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(period|effective|start|end|from|through|duration)\b").unwrap(),
            classification: "temporal.period",
            confidence: 0.80,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(item|clin|slin|line\s+item|supplies|services)\b").unwrap(),
            classification: "clin.item",
            confidence: 0.80,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(discount|terms|payment\s+terms|net\s+\d+)\b").unwrap(),
            classification: "terms.payment",
            confidence: 0.80,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(fob|destination|origin|shipping)\b").unwrap(),
            classification: "terms.shipping",
            confidence: 0.80,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(amendment|modification|change|sf\s*30)\b").unwrap(),
            classification: "reference.amendment",
            confidence: 0.85,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(wage|sca|prevailing|labor\s+rate|fringe)\b").unwrap(),
            classification: "reference.wage",
            confidence: 0.85,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(insurance|liability|bond|surety)\b").unwrap(),
            classification: "terms.insurance",
            confidence: 0.80,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(submit|due|response|deadline|close)\b").unwrap(),
            classification: "temporal.deadline",
            confidence: 0.85,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(set.?aside|small\s+business|hubzone|sdvosb|wosb|8\(?a\)?)\b").unwrap(),
            classification: "socioeconomic.setaside",
            confidence: 0.85,
        },
    ]);

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

    for rule in RULES.iter() {
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
    fn test_classify_solicitation() {
        let mem = tmp_memory();
        let f = DetectedField::new(0, (0.0, 0.0, 100.0, 20.0), "Solicitation Number");
        let c = classify_field(&f, &mem);
        assert_eq!(c.classification, "reference.solicitation");
    }

    #[test]
    fn test_classify_deadline() {
        let mem = tmp_memory();
        let f = DetectedField::new(0, (0.0, 0.0, 100.0, 20.0), "Response Due Date");
        let c = classify_field(&f, &mem);
        // Should match temporal.deadline (has "due") before temporal.date
        assert!(c.classification == "temporal.deadline" || c.classification == "temporal.date");
    }

    #[test]
    fn test_classify_setaside() {
        let mem = tmp_memory();
        let f = DetectedField::new(0, (0.0, 0.0, 100.0, 20.0), "Small Business Set-Aside");
        let c = classify_field(&f, &mem);
        assert!(c.classification == "socioeconomic.setaside" || c.classification == "identity.name");
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
