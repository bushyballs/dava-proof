use crate::context::resolve_key;
use crate::memory::FieldMemory;
use crate::models::{ClassifiedField, FilledField};
use chrono::Local;
use serde_json::Value;

/// Returns today's date formatted as MM/DD/YYYY.
fn today_date() -> String {
    Local::now().format("%m/%d/%Y").to_string()
}

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
        // Payment / discount terms
        "terms.payment" => vec!["discount_terms", "payment_terms"],
        // Signer title
        "identity.title" => vec!["signer_title", "title"],
        // Socioeconomic / set-aside
        "socioeconomic.setaside" => vec!["setaside"],
        // Period of performance
        "temporal.period" => vec!["period_of_performance"],
        // Delivery address
        "location.delivery" => vec!["delivery_address", "address"],
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
        // TIN / EIN / tax-id — check label for "tin", "tax", "ein", "employer"
        // Note: "Employer Identification Number" does NOT contain the substring "ein"
        // so we also check for "employer" as a trigger.
        if label_lower.contains("tin")
            || label_lower.contains("tax")
            || label_lower.contains("ein")
            || label_lower.contains("employer")
        {
            for key in &["identity.tin", "tin", "ein", "tax_id"] {
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

        // "Bid Provided By" — combines company name, signer name, and title
        if label_lower.contains("bid provided by") || label_lower.contains("provided by") {
            let company = resolve_key(ctx, "identity.name")
                .or_else(|| resolve_key(ctx, "company.name"))
                .or_else(|| resolve_key(ctx, "name"))
                .unwrap_or_default();
            let signer = resolve_key(ctx, "identity.signer")
                .or_else(|| resolve_key(ctx, "signer_name"))
                .or_else(|| resolve_key(ctx, "signer"))
                .unwrap_or_default();
            let title = resolve_key(ctx, "signer_title")
                .or_else(|| resolve_key(ctx, "title"))
                .unwrap_or_default();
            if !company.is_empty() && !signer.is_empty() {
                let combined = if !title.is_empty() {
                    format!("{} \u{2014} {}, {}", company, signer, title)
                } else {
                    format!("{} \u{2014} {}", company, signer)
                };
                return Some((combined, 1.0));
            }
        }
    }

    // temporal.date: auto-generate today's date when no key found in context
    if field.classification == "temporal.date" {
        for key in context_keys("temporal.date") {
            if let Some(v) = resolve_key(ctx, key) {
                return Some((v, 1.0));
            }
        }
        // Fall back to today's date automatically (confidence 0.9 — auto-generated)
        return Some((today_date(), 0.9));
    }

    // socioeconomic.setaside: default to "Small Business" when missing
    if field.classification == "socioeconomic.setaside" {
        if let Some(v) = resolve_key(ctx, "setaside") {
            return Some((v, 1.0));
        }
        return Some(("Small Business".to_string(), 0.8));
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

    // ── New tests for expanded context keys ──────────────────────────────────

    #[test]
    fn test_payment_terms_from_discount_terms() {
        let mem = tmp_memory();
        let ctx = json!({"discount_terms": "Net 30"});
        let f = classified("Payment Terms", "terms.payment");
        let result = fill_field(&f, &ctx, &mem, true);
        assert_eq!(result.value, "Net 30");
        assert_eq!(result.source_level, "context");
    }

    #[test]
    fn test_payment_terms_from_payment_terms_key() {
        let mem = tmp_memory();
        let ctx = json!({"payment_terms": "Net 60"});
        let f = classified("Discount Terms", "terms.payment");
        let result = fill_field(&f, &ctx, &mem, true);
        assert_eq!(result.value, "Net 60");
        assert_eq!(result.source_level, "context");
    }

    #[test]
    fn test_identity_title_from_signer_title() {
        let mem = tmp_memory();
        let ctx = json!({"signer_title": "President"});
        let f = classified("Title", "identity.title");
        let result = fill_field(&f, &ctx, &mem, true);
        assert_eq!(result.value, "President");
        assert_eq!(result.source_level, "context");
    }

    #[test]
    fn test_bid_provided_by_combines_name_and_title() {
        let mem = tmp_memory();
        let ctx = json!({
            "identity": {"name": "Hoags Inc.", "signer": "Collin Hoag"},
            "signer_title": "President"
        });
        let f = classified("Bid Provided By", "identity.name");
        let result = fill_field(&f, &ctx, &mem, true);
        assert_eq!(result.value, "Hoags Inc. \u{2014} Collin Hoag, President");
        assert_eq!(result.source_level, "context");
    }

    #[test]
    fn test_bid_provided_by_without_title() {
        let mem = tmp_memory();
        let ctx = json!({
            "identity": {"name": "Hoags Inc.", "signer": "Collin Hoag"}
        });
        let f = classified("Bid Provided By", "identity.name");
        let result = fill_field(&f, &ctx, &mem, true);
        assert_eq!(result.value, "Hoags Inc. \u{2014} Collin Hoag");
        assert_eq!(result.source_level, "context");
    }

    #[test]
    fn test_date_auto_generates_when_missing() {
        let mem = tmp_memory();
        let ctx = json!({});
        let f = classified("Date", "temporal.date");
        let result = fill_field(&f, &ctx, &mem, true);
        // Should be non-empty and look like a date MM/DD/YYYY
        assert!(!result.value.is_empty());
        assert!(result.value.contains('/'));
        assert_eq!(result.source_level, "context");
    }

    #[test]
    fn test_date_from_context_wins_over_auto() {
        let mem = tmp_memory();
        let ctx = json!({"date": "01/01/2025"});
        let f = classified("Date", "temporal.date");
        let result = fill_field(&f, &ctx, &mem, true);
        assert_eq!(result.value, "01/01/2025");
        assert_eq!(result.confidence, 1.0);
    }

    #[test]
    fn test_setaside_defaults_to_small_business() {
        let mem = tmp_memory();
        let ctx = json!({});
        let f = classified("Set-Aside", "socioeconomic.setaside");
        let result = fill_field(&f, &ctx, &mem, true);
        assert_eq!(result.value, "Small Business");
        assert_eq!(result.source_level, "context");
    }

    #[test]
    fn test_setaside_from_context_key() {
        let mem = tmp_memory();
        let ctx = json!({"setaside": "8(a)"});
        let f = classified("Set-Aside", "socioeconomic.setaside");
        let result = fill_field(&f, &ctx, &mem, true);
        assert_eq!(result.value, "8(a)");
        assert_eq!(result.source_level, "context");
    }

    #[test]
    fn test_period_of_performance() {
        let mem = tmp_memory();
        let ctx = json!({"period_of_performance": "12 Months"});
        let f = classified("Period of Performance", "temporal.period");
        let result = fill_field(&f, &ctx, &mem, true);
        assert_eq!(result.value, "12 Months");
        assert_eq!(result.source_level, "context");
    }

    #[test]
    fn test_delivery_address_from_delivery_address_key() {
        let mem = tmp_memory();
        let ctx = json!({"delivery_address": "123 Main St, Portland, OR 97201"});
        let f = classified("Delivery Address", "location.delivery");
        let result = fill_field(&f, &ctx, &mem, true);
        assert_eq!(result.value, "123 Main St, Portland, OR 97201");
        assert_eq!(result.source_level, "context");
    }

    #[test]
    fn test_tin_flat_key_disambiguation() {
        let mem = tmp_memory();
        // Flat "tin" key (not nested under identity)
        let ctx = json!({"tin": "12-3456789"});
        let f = classified("TIN / EIN", "identity.code");
        let result = fill_field(&f, &ctx, &mem, true);
        assert_eq!(result.value, "12-3456789");
        assert_eq!(result.source_level, "context");
    }

    #[test]
    fn test_ein_flat_key_disambiguation() {
        let mem = tmp_memory();
        // "Employer Identification Number" — triggers via "employer" substring
        let ctx = json!({"ein": "98-7654321"});
        let f = classified("Employer Identification Number", "identity.code");
        let result = fill_field(&f, &ctx, &mem, true);
        assert_eq!(result.value, "98-7654321");
        assert_eq!(result.source_level, "context");
    }

    #[test]
    fn test_tax_id_flat_key_disambiguation() {
        let mem = tmp_memory();
        let ctx = json!({"tax_id": "55-5555555"});
        let f = classified("Tax ID", "identity.code");
        let result = fill_field(&f, &ctx, &mem, true);
        assert_eq!(result.value, "55-5555555");
        assert_eq!(result.source_level, "context");
    }
}
