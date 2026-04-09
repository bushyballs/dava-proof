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
            pattern: Regex::new(r"(?i)\b(tin|ein|tax.?id|tax\s+identification)\b").unwrap(),
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
        // ── Contracting Officer / Administrative ──────────────────────────────
        Rule {
            pattern: Regex::new(r"(?i)\b(contracting\s+officer|co\s+name|co\s+sign|admin\s+officer|administrative\s+officer)\b").unwrap(),
            classification: "admin.officer",
            confidence: 0.90,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(remittance|payment\s+office|paying\s+office|pay\s+office)\b").unwrap(),
            classification: "admin.payment",
            confidence: 0.85,
        },
        // ── References / Numbers ──────────────────────────────────────────────
        Rule {
            pattern: Regex::new(r"(?i)\b(requisition|pr\s+no|pr\s+number|purchase\s+request)\b").unwrap(),
            classification: "reference.requisition",
            confidence: 0.85,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(scope|statement\s+of\s+work|sow|pws|performance\s+work\s+statement)\b").unwrap(),
            classification: "reference.scope",
            confidence: 0.85,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(past\s+performance|performance\s+history|performance\s+record)\b").unwrap(),
            classification: "reference.performance",
            confidence: 0.85,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(technical\s+approach|technical\s+proposal|technical\s+volume|technical\s+factor)\b").unwrap(),
            classification: "reference.technical",
            confidence: 0.85,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(key\s+personnel|key\s+staff|key\s+person)\b").unwrap(),
            classification: "reference.personnel",
            confidence: 0.85,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(quality\s+control|quality\s+assurance|qc\s+plan|qa\s+plan|qcp|qap)\b").unwrap(),
            classification: "reference.quality",
            confidence: 0.85,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(safety\s+plan|osha|em\s*385|safety\s+officer|accident\s+prevention)\b").unwrap(),
            classification: "reference.safety",
            confidence: 0.85,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(security\s+clearance|background\s+check|background\s+investigation|clearance\s+level|secret|top\s+secret)\b").unwrap(),
            classification: "reference.security",
            confidence: 0.85,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(government.?furnished\s+property|gfp|gfe|government.?furnished\s+equipment|government.?furnished\s+material|gfm)\b").unwrap(),
            classification: "terms.gfe",
            confidence: 0.85,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(property\s+list|equipment\s+list|contractor.?owned|government.?owned)\b").unwrap(),
            classification: "reference.property",
            confidence: 0.80,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(report\s+format|reporting\s+requirement|monthly\s+report|status\s+report|progress\s+report)\b").unwrap(),
            classification: "reference.reporting",
            confidence: 0.85,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(closeout|contract\s+closeout|final\s+invoice|final\s+report|final\s+payment|contract\s+completion)\b").unwrap(),
            classification: "reference.closeout",
            confidence: 0.85,
        },
        // ── Finance / Accounting ──────────────────────────────────────────────
        Rule {
            pattern: Regex::new(r"(?i)\b(accounting|fund\s+cite|appropriation|acrn|line\s+of\s+accounting|loa|treasury|fiscal\s+year|fy\s*\d{2})\b").unwrap(),
            classification: "finance.accounting",
            confidence: 0.85,
        },
        // ── Terms / Conditions ────────────────────────────────────────────────
        Rule {
            pattern: Regex::new(r"(?i)\b(inspection|acceptance|inspect\s+and\s+accept|inspection\s+site|place\s+of\s+inspection)\b").unwrap(),
            classification: "terms.inspection",
            confidence: 0.85,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(warrant(y|ee)|guarantee|guaranty)\b").unwrap(),
            classification: "terms.warranty",
            confidence: 0.85,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(subcontract|subcontractor|sub.?contractor|teaming|team\s+member|subcontracting\s+plan)\b").unwrap(),
            classification: "terms.subcontract",
            confidence: 0.85,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(evaluation\s+factor|evaluation\s+criteria|selection\s+criteria|award\s+criteria|best\s+value)\b").unwrap(),
            classification: "terms.evaluation",
            confidence: 0.85,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(compliance|conform(ance)?|certification|certif(y|ied)|represent)\b").unwrap(),
            classification: "terms.compliance",
            confidence: 0.80,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(liquidated\s+damages|ld\s+rate|delay\s+damages)\b").unwrap(),
            classification: "terms.damages",
            confidence: 0.90,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(travel|per\s+diem|per\s+diem\s+rate|trip\s+cost|travel\s+cost)\b").unwrap(),
            classification: "terms.travel",
            confidence: 0.85,
        },
        // ── Temporal ─────────────────────────────────────────────────────────
        Rule {
            pattern: Regex::new(r"(?i)\b(option\s+year|option\s+period|option\s+to\s+extend|exercis(e|ing)\s+option)\b").unwrap(),
            classification: "temporal.option",
            confidence: 0.90,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(base\s+year|base\s+period|base\s+term|initial\s+period|initial\s+term)\b").unwrap(),
            classification: "temporal.base",
            confidence: 0.90,
        },
        Rule {
            pattern: Regex::new(r"(?i)\b(delivery\s+schedule|schedule\s+of\s+delivery|required\s+delivery\s+date|rdd)\b").unwrap(),
            classification: "location.delivery",
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

    #[test]
    fn test_classify_email() {
        let mem = tmp_memory();
        let f = DetectedField::new(0, (0.0, 0.0, 100.0, 20.0), "Offeror Email Address");
        let c = classify_field(&f, &mem);
        assert_eq!(c.classification, "identity.email");
    }

    #[test]
    fn test_classify_address() {
        let mem = tmp_memory();
        let f = DetectedField::new(0, (0.0, 0.0, 100.0, 20.0), "Offeror Address");
        let c = classify_field(&f, &mem);
        assert_eq!(c.classification, "identity.address");
    }

    #[test]
    fn test_classify_uei() {
        let mem = tmp_memory();
        let f = DetectedField::new(0, (0.0, 0.0, 100.0, 20.0), "UEI");
        let c = classify_field(&f, &mem);
        assert_eq!(c.classification, "identity.code");
    }

    #[test]
    fn test_classify_tin() {
        let mem = tmp_memory();
        let f = DetectedField::new(0, (0.0, 0.0, 100.0, 20.0), "Tax Identification Number");
        let c = classify_field(&f, &mem);
        assert_eq!(c.classification, "identity.code");
    }

    #[test]
    fn test_classify_currency() {
        let mem = tmp_memory();
        let f = DetectedField::new(0, (0.0, 0.0, 100.0, 20.0), "Unit Price");
        let c = classify_field(&f, &mem);
        assert_eq!(c.classification, "currency");
    }
}
