use crate::context::resolve_key;
use crate::memory::FieldMemory;
use crate::models::{ClassifiedField, FilledField};
use serde_json::Value;

/// Maps a classification to the ordered list of context keys to try.
/// Supports both nested ("identity.name") and flat ("name") formats.
fn context_keys(classification: &str) -> Vec<&'static str> {
    match classification {
        "identity.name" => vec!["identity.name", "company.name", "name"],
        // identity.code uses label-aware disambiguation — don't put flat keys here
        // to avoid CAGE being returned for TIN fields
        "identity.code" => vec!["identity.cage", "identity.uei", "identity.ein"],
        "identity.address" => vec!["identity.address", "address"],
        "identity.phone" => vec!["identity.phone", "phone"],
        "identity.email" => vec!["identity.email", "email"],
        "signature" => vec!["identity.signer", "signer_name", "signer"],
        "temporal.date" => vec!["bid.date", "date"],
        _ => vec![],
    }
}

/// Level 1: resolve from context JSON — label-aware for ambiguous classifications.
fn level1_context(field: &ClassifiedField, ctx: &Value) -> Option<(String, f64)> {
    // Label-aware disambiguation for identity.code
    if field.classification == "identity.code" {
        let label_lower = field.label.to_lowercase();
        if label_lower.contains("cage") {
            for key in &["identity.cage", "cage"] {
                if let Some(v) = resolve_key(ctx, key) { return Some((v, 1.0)); }
            }
        }
        if label_lower.contains("uei") {
            for key in &["identity.uei", "uei"] {
                if let Some(v) = resolve_key(ctx, key) { return Some((v, 1.0)); }
            }
        }
        if label_lower.contains("tin") || label_lower.contains("tax") || label_lower.contains("ein") {
            for key in &["identity.tin", "tin"] {
                if let Some(v) = resolve_key(ctx, key) { return Some((v, 1.0)); }
            }
        }
    }

    // Label-aware for identity.name: POC vs company name
    if field.classification == "identity.name" {
        let label_lower = field.label.to_lowercase();
        if label_lower.contains("point of contact") || label_lower.contains("poc") {
            for key in &["identity.signer", "signer_name", "signer"] {
                if let Some(v) = resolve_key(ctx, key) { return Some((v, 1.0)); }
            }
        }
    }

    // Standard classification -> key mapping
    for key in context_keys(&field.classification) {
        if let Some(v) = resolve_key(ctx, key) {
            return Some((v, 1.0));
        }
    }

    // Last resort: use the classification itself as a dotted key path
    if let Some(v) = resolve_key(ctx, &field.classification) {
        return Some((v, 1.0));
    }

    None
}

/// Level 2: recall from DAVA field memory.
fn level2_memory(field: &ClassifiedField, memory: &FieldMemory) -> Option<(String, f64)> {
    memory
        .recall(&field.label)
        .map(|hit| (hit.value, hit.confidence))
}

/// Fill a single classified field.
///
/// Resolution order:
///   1. Context JSON (exact, confidence 1.0)
///   2. DAVA memory (learned from past bids)
///   3. Airgap barrier — if `airgap` is true, stop here
///   4. (Levels 3+4 — Ollama/Claude inference — reserved for future Rust impl)
pub fn fill_field(
    field: &ClassifiedField,
    ctx: &Value,
    memory: &FieldMemory,
    airgap: bool,
) -> FilledField {
    // Signature gets "/s/ <signer>" formatting
    if field.classification == "signature" {
        for key in &["identity.signer", "signer_name", "signer"] {
            if let Some(signer) = resolve_key(ctx, key) {
                return FilledField::from_classified(
                    field,
                    &format!("/s/ {}", signer),
                    "context",
                    1.0,
                );
            }
        }
    }

    // Level 1: Context
    if let Some((val, conf)) = level1_context(field, ctx) {
        return FilledField::from_classified(field, &val, "context", conf);
    }

    // Level 2: DAVA Memory
    if let Some((val, conf)) = level2_memory(field, memory) {
        return FilledField::from_classified(field, &val, "dava_memory", conf);
    }

    // Airgap barrier — no inference allowed
    if airgap {
        return FilledField::from_classified(field, "", "none", 0.1);
    }

    // Levels 3+4 (Ollama/Claude) — not yet implemented in Rust
    FilledField::from_classified(field, "", "none", 0.1)
}

/// Fill a slice of classified fields.
pub fn fill_fields(
    fields: &[ClassifiedField],
    ctx: &Value,
    memory: &FieldMemory,
    airgap: bool,
) -> Vec<FilledField> {
    fields
        .iter()
        .map(|f| fill_field(f, ctx, memory, airgap))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::DetectedField;
    use serde_json::json;
    use tempfile::NamedTempFile;

    fn tmp_memory() -> FieldMemory {
        let tmp = NamedTempFile::new().unwrap();
        FieldMemory::open(tmp.path()).unwrap()
    }

    fn classified(label: &str, classification: &str) -> ClassifiedField {
        let det = DetectedField::new(0, (0.0, 0.0, 100.0, 20.0), label);
        ClassifiedField::from_detected(&det, classification, 0.9)
    }

    #[test]
    fn test_context_fill() {
        let mem = tmp_memory();
        let ctx = json!({"identity": {"name": "Hoags Inc."}});
        let f = classified("Offeror Name", "identity.name");
        let result = fill_field(&f, &ctx, &mem, true);
        assert_eq!(result.value, "Hoags Inc.");
        assert_eq!(result.source_level, "context");
    }

    #[test]
    fn test_memory_fill() {
        let mem = tmp_memory();
        mem.store(
            "offeror phone",
            "identity.phone",
            "(458) 239-3215",
            "identity.phone",
            "x.pdf",
            true,
        );
        let ctx = json!({});
        let f = classified("Offeror Phone", "identity.phone");
        let result = fill_field(&f, &ctx, &mem, true);
        assert_eq!(result.value, "(458) 239-3215");
        assert_eq!(result.source_level, "dava_memory");
    }

    #[test]
    fn test_signature_formatting() {
        let mem = tmp_memory();
        let ctx = json!({"identity": {"signer": "Collin Hoag"}});
        let f = classified("Authorized Signature", "signature");
        let result = fill_field(&f, &ctx, &mem, true);
        assert_eq!(result.value, "/s/ Collin Hoag");
        assert_eq!(result.source_level, "context");
    }

    #[test]
    fn test_cage_label_disambiguation() {
        let mem = tmp_memory();
        let ctx = json!({"identity": {"cage": "7ABCD", "uei": "UEIUEI123456"}});
        let f = classified("CAGE Code", "identity.code");
        let result = fill_field(&f, &ctx, &mem, true);
        assert_eq!(result.value, "7ABCD");
    }

    #[test]
    fn test_uei_label_disambiguation() {
        let mem = tmp_memory();
        let ctx = json!({"identity": {"cage": "7ABCD", "uei": "UEIUEI123456"}});
        let f = classified("UEI Number", "identity.code");
        let result = fill_field(&f, &ctx, &mem, true);
        assert_eq!(result.value, "UEIUEI123456");
    }

    #[test]
    fn test_unfillable() {
        let mem = tmp_memory();
        let ctx = json!({});
        let f = classified("Unknown Field", "unknown");
        let result = fill_field(&f, &ctx, &mem, true);
        assert!(result.value.is_empty());
        assert!(result.confidence < 0.5);
    }

    #[test]
    fn test_context_beats_memory() {
        let mem = tmp_memory();
        mem.store(
            "offeror name",
            "identity.name",
            "OldCo LLC",
            "identity.name",
            "old.pdf",
            true,
        );
        let ctx = json!({"identity": {"name": "Hoags Inc."}});
        let f = classified("Offeror Name", "identity.name");
        let result = fill_field(&f, &ctx, &mem, true);
        // Context (level 1) always wins over memory (level 2)
        assert_eq!(result.value, "Hoags Inc.");
        assert_eq!(result.source_level, "context");
    }

    #[test]
    fn test_signature_with_signer_name() {
        let mem = tmp_memory();
        // Use flat "signer_name" key (not nested under identity)
        let ctx = json!({"signer_name": "Jane Doe"});
        let f = classified("Authorized Signature", "signature");
        let result = fill_field(&f, &ctx, &mem, true);
        assert_eq!(result.value, "/s/ Jane Doe");
        assert_eq!(result.source_level, "context");
    }

    #[test]
    fn test_fill_fields_batch() {
        let mem = tmp_memory();
        let ctx = json!({"identity": {"name": "Hoags Inc.", "phone": "555-1234"}});
        let fields = vec![
            classified("Offeror Name", "identity.name"),
            classified("Offeror Phone", "identity.phone"),
            classified("Mystery Field", "unknown"),
        ];
        let results = fill_fields(&fields, &ctx, &mem, true);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].value, "Hoags Inc.");
        assert_eq!(results[1].value, "555-1234");
        assert!(results[2].value.is_empty());
    }
}
