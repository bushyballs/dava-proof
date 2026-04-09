//! generate.rs — PDF invoice generation using lopdf.
//!
//! Creates a professional government invoice PDF:
//!   • Header  — "HOAGS INC. — INVOICE" + company block
//!   • Invoice meta — invoice number, contract, billing period, date
//!   • Table   — CLIN | Description | Qty | Unit | Unit Price | Amount
//!   • Footer  — Total, payment terms, remittance info
//!
//! All text uses the built-in Helvetica family (no external font files needed).

use lopdf::{Document, Object, Stream, dictionary};
use lopdf::content::{Content, Operation};

use crate::invoice::Invoice;

// ── Page geometry ─────────────────────────────────────────────────────────────
const PAGE_W: f32 = 612.0; // US Letter
const PAGE_H: f32 = 792.0;
const MARGIN_L: f32 = 60.0;
const MARGIN_R: f32 = 60.0;
const BODY_W: f32 = PAGE_W - MARGIN_L - MARGIN_R;

// ── Column widths ─────────────────────────────────────────────────────────────
const COL_CLIN: f32 = 45.0;
const COL_DESC: f32 = 185.0;
const COL_QTY: f32 = 55.0;
const COL_UNIT: f32 = 40.0;
const COL_UP: f32 = 80.0;
const COL_AMT: f32 = BODY_W - COL_CLIN - COL_DESC - COL_QTY - COL_UNIT - COL_UP;

// ── Font helpers ──────────────────────────────────────────────────────────────

fn op_font(name: &str, size: f32) -> Operation {
    Operation::new("Tf", vec![name.into(), size.into()])
}
fn op_move(x: f32, y: f32) -> Operation {
    Operation::new("Td", vec![x.into(), y.into()])
}
fn op_show(text: &str) -> Operation {
    Operation::new("Tj", vec![Object::string_literal(text)])
}
fn op_rg(r: f32, g: f32, b: f32) -> Operation {
    Operation::new("rg", vec![r.into(), g.into(), b.into()])
}
fn op_rg_stroke(r: f32, g: f32, b: f32) -> Operation {
    Operation::new("RG", vec![r.into(), g.into(), b.into()])
}
fn op_rect(x: f32, y: f32, w: f32, h: f32) -> Operation {
    Operation::new("re", vec![x.into(), y.into(), w.into(), h.into()])
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Generate a PDF invoice and write it to `output_path`.
pub fn generate_pdf(invoice: &Invoice, output_path: &str) -> Result<(), String> {
    let mut doc = Document::with_version("1.5");

    // ── Font resources ────────────────────────────────────────────────────────
    let helv_id = doc.add_object(dictionary! {
        "Type"    => "Font",
        "Subtype" => "Type1",
        "BaseFont"=> "Helvetica",
    });
    let helv_bold_id = doc.add_object(dictionary! {
        "Type"    => "Font",
        "Subtype" => "Type1",
        "BaseFont"=> "Helvetica-Bold",
    });

    let resources_id = doc.add_object(dictionary! {
        "Font" => dictionary! {
            "Helv"     => helv_id,
            "HelvBold" => helv_bold_id,
        }
    });

    // ── Page object ───────────────────────────────────────────────────────────
    let page_id = doc.add_object(dictionary! {
        "Type"      => "Page",
        "MediaBox"  => vec![0.into(), 0.into(), PAGE_W.into(), PAGE_H.into()],
        "Resources" => resources_id,
    });

    // ── Build content ops ─────────────────────────────────────────────────────
    let mut ops: Vec<Operation> = Vec::new();

    // Save graphics state
    ops.push(Operation::new("q", vec![]));

    let mut y = PAGE_H - 50.0;

    // ── HEADER BAND ───────────────────────────────────────────────────────────
    // Dark navy rectangle
    ops.push(op_rg(0.094, 0.149, 0.286)); // #182660 ish
    ops.push(op_rect(MARGIN_L, y - 30.0, BODY_W, 42.0));
    ops.push(Operation::new("f", vec![]));

    // Title text
    ops.push(Operation::new("BT", vec![]));
    ops.push(op_rg(1.0, 1.0, 1.0)); // white
    ops.push(op_font("HelvBold", 18.0));
    ops.push(op_move(MARGIN_L + 8.0, y - 18.0));
    ops.push(op_show("HOAGS INC.  \u{2014}  INVOICE"));
    ops.push(Operation::new("ET", vec![]));

    // Company info (right side)
    ops.push(Operation::new("BT", vec![]));
    ops.push(op_rg(0.9, 0.9, 0.9));
    ops.push(op_font("Helv", 7.5));
    ops.push(op_move(MARGIN_L + BODY_W - 180.0, y - 10.0));
    ops.push(op_show("Hoags Inc.  |  Federal Contracting Services"));
    ops.push(op_move(0.0, -9.5));
    ops.push(op_show("UEI: [UEI]  |  CAGE: [CAGE]"));
    ops.push(op_move(0.0, -9.5));
    ops.push(op_show("collinhoag@gmail.com"));
    ops.push(Operation::new("ET", vec![]));

    y -= 50.0;

    // ── INVOICE META BLOCK ────────────────────────────────────────────────────
    ops.push(Operation::new("BT", vec![]));
    ops.push(op_rg(0.0, 0.0, 0.0));

    // Left column
    ops.push(op_font("HelvBold", 9.0));
    ops.push(op_move(MARGIN_L, y));
    ops.push(op_show("Invoice Number:"));
    ops.push(op_move(0.0, -13.0));
    ops.push(op_show("Contract Number:"));
    ops.push(op_move(0.0, -13.0));
    ops.push(op_show("Billing Period:"));
    ops.push(op_move(0.0, -13.0));
    ops.push(op_show("Invoice Date:"));

    ops.push(op_font("Helv", 9.0));
    ops.push(op_move(MARGIN_L + 105.0 - (MARGIN_L + 0.0), y));
    // Absolute positioning by resetting with BT/ET is complex in lopdf; use
    // a second BT block for the value column.
    ops.push(Operation::new("ET", vec![]));

    ops.push(Operation::new("BT", vec![]));
    ops.push(op_rg(0.0, 0.0, 0.0));
    ops.push(op_font("Helv", 9.0));
    ops.push(op_move(MARGIN_L + 115.0, y));
    ops.push(op_show(&invoice.invoice_number));
    ops.push(op_move(0.0, -13.0));
    ops.push(op_show(&invoice.contract_number));
    ops.push(op_move(0.0, -13.0));
    ops.push(op_show(&invoice.billing_period));
    ops.push(op_move(0.0, -13.0));
    // Trim the timestamp to just the date
    let date_only = invoice.generated_at.get(..10).unwrap_or(&invoice.generated_at);
    ops.push(op_show(date_only));
    ops.push(Operation::new("ET", vec![]));

    y -= 70.0;

    // ── TABLE HEADER ─────────────────────────────────────────────────────────
    // Blue header row
    ops.push(op_rg(0.2, 0.38, 0.62));
    ops.push(op_rect(MARGIN_L, y - 4.0, BODY_W, 18.0));
    ops.push(Operation::new("f", vec![]));

    ops.push(Operation::new("BT", vec![]));
    ops.push(op_rg(1.0, 1.0, 1.0));
    ops.push(op_font("HelvBold", 8.5));

    let mut x = MARGIN_L + 3.0;
    ops.push(op_move(x, y + 2.0));
    ops.push(op_show("CLIN"));
    x += COL_CLIN;
    ops.push(op_move(COL_CLIN - 3.0, 0.0));
    ops.push(op_show("Description"));
    x += COL_DESC;
    ops.push(op_move(COL_DESC, 0.0));
    ops.push(op_show("Qty"));
    x += COL_QTY;
    ops.push(op_move(COL_QTY, 0.0));
    ops.push(op_show("Unit"));
    x += COL_UNIT;
    ops.push(op_move(COL_UNIT, 0.0));
    ops.push(op_show("Unit Price"));
    x += COL_UP;
    ops.push(op_move(COL_UP, 0.0));
    ops.push(op_show("Amount"));
    let _ = x; // suppress warning

    ops.push(Operation::new("ET", vec![]));

    y -= 20.0;

    // ── TABLE ROWS ────────────────────────────────────────────────────────────
    let row_h = 14.0;
    for (i, line) in invoice.lines.iter().enumerate() {
        // Alternating background
        if i % 2 == 1 {
            ops.push(op_rg(0.94, 0.96, 0.99));
            ops.push(op_rect(MARGIN_L, y - 3.0, BODY_W, row_h));
            ops.push(Operation::new("f", vec![]));
        }

        ops.push(Operation::new("BT", vec![]));
        ops.push(op_rg(0.0, 0.0, 0.0));
        ops.push(op_font("Helv", 8.5));

        let mut cx = MARGIN_L + 3.0;
        ops.push(op_move(cx, y + 1.0));
        ops.push(op_show(&line.clin));
        cx += COL_CLIN;

        ops.push(op_move(COL_CLIN - 3.0, 0.0));
        // Truncate long descriptions to fit the column
        let desc = truncate(&line.description, 32);
        ops.push(op_show(desc));
        cx += COL_DESC;

        ops.push(op_move(COL_DESC, 0.0));
        ops.push(op_show(&format!("{:.2}", line.qty)));
        cx += COL_QTY;

        ops.push(op_move(COL_QTY, 0.0));
        ops.push(op_show(&line.unit));
        cx += COL_UNIT;

        ops.push(op_move(COL_UNIT, 0.0));
        ops.push(op_show(&format!("${:.2}", line.unit_price)));
        cx += COL_UP;

        ops.push(op_move(COL_UP, 0.0));
        ops.push(op_show(&format!("${:.2}", line.amount)));
        let _ = cx;

        ops.push(Operation::new("ET", vec![]));
        y -= row_h;
    }

    y -= 6.0;

    // ── HORIZONTAL RULE ───────────────────────────────────────────────────────
    ops.push(op_rg_stroke(0.2, 0.38, 0.62));
    ops.push(Operation::new("w", vec![Object::from(1.0f32)]));
    ops.push(Operation::new("m", vec![(MARGIN_L).into(), y.into()]));
    ops.push(Operation::new("l", vec![(MARGIN_L + BODY_W).into(), y.into()]));
    ops.push(Operation::new("S", vec![]));

    y -= 14.0;

    // ── TOTAL LINE ────────────────────────────────────────────────────────────
    ops.push(Operation::new("BT", vec![]));
    ops.push(op_rg(0.0, 0.0, 0.0));
    ops.push(op_font("HelvBold", 10.0));
    ops.push(op_move(MARGIN_L + BODY_W - COL_AMT - COL_UP, y));
    ops.push(op_show("TOTAL DUE:"));
    ops.push(op_move(COL_UP, 0.0));
    ops.push(op_show(&format!("${:.2}", invoice.total)));
    ops.push(Operation::new("ET", vec![]));

    y -= 40.0;

    // ── PAYMENT TERMS ─────────────────────────────────────────────────────────
    ops.push(Operation::new("BT", vec![]));
    ops.push(op_rg(0.2, 0.2, 0.2));
    ops.push(op_font("HelvBold", 8.5));
    ops.push(op_move(MARGIN_L, y));
    ops.push(op_show("Payment Terms:"));
    ops.push(op_font("Helv", 8.5));
    ops.push(op_move(80.0, 0.0));
    ops.push(op_show("Net 30 — payment due within 30 days of invoice date."));
    ops.push(Operation::new("ET", vec![]));

    y -= 13.0;

    ops.push(Operation::new("BT", vec![]));
    ops.push(op_rg(0.2, 0.2, 0.2));
    ops.push(op_font("HelvBold", 8.5));
    ops.push(op_move(MARGIN_L, y));
    ops.push(op_show("Remittance:"));
    ops.push(op_font("Helv", 8.5));
    ops.push(op_move(80.0, 0.0));
    ops.push(op_show("ACH — banking details provided via SF3881 on file."));
    ops.push(Operation::new("ET", vec![]));

    // ── FOOTER BAND ───────────────────────────────────────────────────────────
    ops.push(op_rg(0.094, 0.149, 0.286));
    ops.push(op_rect(MARGIN_L, 28.0, BODY_W, 18.0));
    ops.push(Operation::new("f", vec![]));

    ops.push(Operation::new("BT", vec![]));
    ops.push(op_rg(0.9, 0.9, 0.9));
    ops.push(op_font("Helv", 7.5));
    ops.push(op_move(MARGIN_L + 4.0, 34.0));
    ops.push(op_show(&format!(
        "Hoags Inc.  |  {}  |  Generated by Invoicer v0.1  |  {}",
        invoice.contract_number,
        invoice.generated_at.get(..10).unwrap_or("")
    )));
    ops.push(Operation::new("ET", vec![]));

    // Page number — right-aligned in the footer band
    ops.push(Operation::new("BT", vec![]));
    ops.push(op_rg(0.9, 0.9, 0.9));
    ops.push(op_font("Helv", 7.5));
    ops.push(op_move(MARGIN_L + BODY_W - 40.0, 34.0));
    ops.push(op_show("Page 1 of 1"));
    ops.push(Operation::new("ET", vec![]));

    // Restore graphics state
    ops.push(Operation::new("Q", vec![]));

    // ── Encode content stream ─────────────────────────────────────────────────
    let content = Content { operations: ops };
    let content_bytes = content.encode().map_err(|e| format!("Encode error: {e}"))?;
    let stream = Stream::new(dictionary! {}, content_bytes);
    let content_id = doc.add_object(stream);

    // Attach content stream to page
    doc.get_object_mut(page_id)
        .map_err(|e| format!("Page object error: {e}"))?
        .as_dict_mut()
        .map_err(|e| format!("Page dict error: {e}"))?
        .set("Contents", content_id);

    // ── Build page tree ───────────────────────────────────────────────────────
    let pages_id = doc.add_object(dictionary! {
        "Type"  => "Pages",
        "Kids"  => vec![page_id.into()],
        "Count" => 1i64,
    });

    doc.get_object_mut(page_id)
        .map_err(|e| format!("Page parent error: {e}"))?
        .as_dict_mut()
        .map_err(|e| format!("Page dict parent error: {e}"))?
        .set("Parent", pages_id);

    let catalog_id = doc.add_object(dictionary! {
        "Type"  => "Catalog",
        "Pages" => pages_id,
    });

    doc.trailer.set("Root", catalog_id);

    doc.save(output_path).map_err(|e| format!("Save error: {e}"))?;
    Ok(())
}

fn truncate(s: &str, max_chars: usize) -> &str {
    if s.len() <= max_chars {
        s
    } else {
        // Truncate at a char boundary
        let mut idx = max_chars;
        while !s.is_char_boundary(idx) {
            idx -= 1;
        }
        &s[..idx]
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::invoice::{Invoice, InvoiceLine};

    fn sample_invoice() -> Invoice {
        Invoice {
            invoice_number: "HOAGS-INV-W9127S-202604-001".to_string(),
            contract_number: "W9127S26QA030".to_string(),
            contractor: "Hoags Inc.".to_string(),
            billing_period: "2026-04".to_string(),
            lines: vec![
                InvoiceLine {
                    clin: "0001".to_string(),
                    description: "Daily Janitorial Service".to_string(),
                    qty: 6.78,
                    unit: "EA".to_string(),
                    unit_price: 91.19,
                    amount: 618.27,
                },
                InvoiceLine {
                    clin: "0002".to_string(),
                    description: "Semi-Annual Deep Clean".to_string(),
                    qty: 0.12,
                    unit: "EA".to_string(),
                    unit_price: 307.61,
                    amount: 36.91,
                },
            ],
            total: 655.18,
            generated_at: "2026-04-08T12:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_generate_pdf_creates_file() {
        let inv = sample_invoice();
        let dir = tempfile::tempdir().expect("tmpdir");
        let path = dir.path().join("test_invoice.pdf");
        let path_str = path.to_str().unwrap();
        generate_pdf(&inv, path_str).expect("generate_pdf should succeed");
        assert!(path.exists(), "PDF file should exist");
        let meta = std::fs::metadata(&path).unwrap();
        assert!(meta.len() > 500, "PDF should have real content");
    }

    #[test]
    fn test_generate_pdf_empty_lines() {
        let mut inv = sample_invoice();
        inv.lines.clear();
        inv.total = 0.0;
        let dir = tempfile::tempdir().expect("tmpdir");
        let path = dir.path().join("empty_invoice.pdf");
        generate_pdf(&inv, path.to_str().unwrap()).expect("should handle empty lines");
        assert!(path.exists());
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 5), "hello");
        // Ensure it doesn't panic on multibyte chars
        let s = "abc";
        assert_eq!(truncate(s, 2), "ab");
    }

    /// Generate an invoice PDF then read it back with lopdf and verify expected
    /// text fragments appear in the extracted page content.
    #[test]
    fn test_generate_pdf_text_content() {
        use lopdf::Document;

        let inv = sample_invoice();
        let dir = tempfile::tempdir().expect("tmpdir");
        let path = dir.path().join("text_check.pdf");
        let path_str = path.to_str().unwrap();

        generate_pdf(&inv, path_str).expect("generate_pdf should succeed");
        assert!(path.exists(), "PDF file should be written");

        // Load the PDF back and extract text from page 1
        let doc = Document::load(&path).expect("lopdf should load the generated PDF");
        let page_nums: Vec<u32> = doc.get_pages().keys().cloned().collect();
        assert!(!page_nums.is_empty(), "PDF must have at least one page");

        let text = doc
            .extract_text(&[page_nums[0]])
            .expect("should be able to extract text from page 1");

        // Invoice number must appear in the document
        assert!(
            text.contains("HOAGS-INV-W9127S-202604-001"),
            "invoice number not found in PDF text: {text}"
        );
        // Contract number must appear
        assert!(
            text.contains("W9127S26QA030"),
            "contract number not found in PDF text: {text}"
        );
        // Total should be present
        assert!(
            text.contains("655.18"),
            "invoice total not found in PDF text: {text}"
        );
        // At least one CLIN should be present
        assert!(
            text.contains("0001") || text.contains("0002"),
            "CLIN number not found in PDF text: {text}"
        );
        // Page number footer
        assert!(
            text.contains("Page 1 of 1"),
            "page number not found in PDF text: {text}"
        );
    }
}
