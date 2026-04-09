/// Signature placement engine for sigstamp.
///
/// Writes a content stream overlay with:
///   - `/s/ {name}` in Helvetica-Oblique (italic), 10pt, blue ink
///   - Date stamp to the right of the signature
///   - Name / Title below the signature line
///
/// Placement strategies:
///   1. Explicit --x --y coordinates (caller supplies them).
///   2. Auto-detected SigLocation from detect.rs.

use chrono::Local;
use lopdf::{Dictionary, Document, Object, Stream};
use std::path::{Path, PathBuf};

use crate::detect::SigLocation;

// ── public types ──────────────────────────────────────────────────────────────

/// Parameters for a single signature stamp operation.
#[derive(Debug, Clone)]
pub struct StampParams {
    pub signer: String,
    pub title: Option<String>,
    /// 0-based page index. Defaults to the first detected sig page (or 0).
    pub page: usize,
    /// Explicit X coordinate in PDF user-space (origin = lower-left).
    /// If None, the auto-detected position is used.
    pub x: Option<f64>,
    /// Explicit Y coordinate in PDF user-space.
    pub y: Option<f64>,
    /// If true, stamp initials (first letter of each word) instead of full name.
    pub initials: bool,
    /// Optional 1-based page range (inclusive). If None, all/detected pages are used.
    pub page_range: Option<(usize, usize)>,
}

// ── helper: compute initials from a full name ─────────────────────────────────

pub fn name_initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|word| word.chars().next())
        .collect::<String>()
        .to_uppercase()
}

// ── helper: PDF string escaping ───────────────────────────────────────────────

fn escape_pdf(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
}

// ── helper: font resources ────────────────────────────────────────────────────

fn ensure_sig_fonts(doc: &mut Document, page_id: lopdf::ObjectId) {
    let font_specs: &[(&[u8], &[u8])] = &[
        (b"HelvO", b"Helvetica-Oblique"),
        (b"Helv", b"Helvetica"),
    ];

    // Read existing Resources (need to clone to avoid borrow conflict)
    let resources_clone = if let Ok(pd) = doc.get_dictionary(page_id) {
        pd.get(b"Resources")
            .ok()
            .and_then(|r| {
                if let Object::Dictionary(d) = r {
                    Some(d.clone())
                } else {
                    None
                }
            })
            .unwrap_or_default()
    } else {
        Dictionary::new()
    };

    let mut fonts = resources_clone
        .get(b"Font")
        .ok()
        .and_then(|f| {
            if let Object::Dictionary(d) = f {
                Some(d.clone())
            } else {
                None
            }
        })
        .unwrap_or_default();

    for (key, base_font) in font_specs {
        if fonts.get(*key).is_err() {
            let mut font_dict = Dictionary::new();
            font_dict.set(b"Type", Object::Name(b"Font".to_vec()));
            font_dict.set(b"Subtype", Object::Name(b"Type1".to_vec()));
            font_dict.set(b"BaseFont", Object::Name(base_font.to_vec()));
            font_dict.set(b"Encoding", Object::Name(b"WinAnsiEncoding".to_vec()));
            let font_id = doc.add_object(Object::Dictionary(font_dict));
            fonts.set(*key, Object::Reference(font_id));
        }
    }

    let mut new_resources = resources_clone.clone();
    new_resources.set(b"Font", Object::Dictionary(fonts));

    if let Ok(pd) = doc.get_dictionary_mut(page_id) {
        pd.set(b"Resources", Object::Dictionary(new_resources));
    }
}

// ── helper: append stream to page Contents ────────────────────────────────────

fn append_stream(doc: &mut Document, page_id: lopdf::ObjectId, ops: String) {
    let stream = Stream::new(Dictionary::new(), ops.into_bytes());
    let stream_id = doc.add_object(Object::Stream(stream));

    if let Ok(page_dict) = doc.get_dictionary_mut(page_id) {
        match page_dict.get(b"Contents") {
            Ok(Object::Reference(existing)) => {
                let existing = *existing;
                page_dict.set(
                    b"Contents",
                    Object::Array(vec![
                        Object::Reference(existing),
                        Object::Reference(stream_id),
                    ]),
                );
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

// ── helper: page height ───────────────────────────────────────────────────────

fn get_page_height(doc: &Document, page_id: lopdf::ObjectId) -> f64 {
    if let Ok(d) = doc.get_dictionary(page_id) {
        if let Ok(Object::Array(arr)) = d.get(b"MediaBox") {
            if arr.len() == 4 {
                return match &arr[3] {
                    Object::Real(h) => *h as f64,
                    Object::Integer(h) => *h as f64,
                    _ => 792.0,
                };
            }
        }
    }
    792.0
}

// ── content stream builder ────────────────────────────────────────────────────

/// Build the PDF content stream operators for one signature stamp.
///
/// Layout (all at the target (x, y) in PDF user-space, origin = lower-left):
///   Line 1 (y + 2):  `/s/ {signer}` or `/s/ {initials}` — HelvO 10pt blue
///   Line 2 (y + 2):  `{date}`                            — Helv 8pt blue, offset right by ~120pt
///   Line 3 (y - 8):  `{signer}` or `{initials}`          — Helv 8pt blue
///   Line 4 (y - 18): `{title}`                           — Helv 7pt blue (if provided)
///
/// If `use_initials` is true, the signature token is the initials of `signer`
/// (first letter of each word, upper-cased) instead of the full name.
pub fn build_stamp_ops(
    signer: &str,
    title: Option<&str>,
    x: f64,
    y: f64,
    date_str: &str,
    use_initials: bool,
) -> String {
    let display_name = if use_initials {
        name_initials(signer)
    } else {
        signer.to_string()
    };
    let slash_sig = format!("/s/ {}", escape_pdf(&display_name));
    let name_line = escape_pdf(&display_name);
    let date_line = escape_pdf(date_str);

    let mut ops = String::new();

    // 0 0 0.6 rg = dark blue fill (RGB)
    // Signature line: italic Helvetica-Oblique 10pt
    ops.push_str(&format!(
        "BT 0 0 0.6 rg /HelvO 10 Tf {} {} Td ({}) Tj ET\n",
        x,
        y + 2.0,
        slash_sig
    ));

    // Date: right of the signature, regular Helvetica 8pt
    ops.push_str(&format!(
        "BT 0 0 0.6 rg /Helv 8 Tf {} {} Td ({}) Tj ET\n",
        x + 130.0,
        y + 2.0,
        date_line
    ));

    // Printed name: below the signature line
    ops.push_str(&format!(
        "BT 0 0 0.6 rg /Helv 8 Tf {} {} Td ({}) Tj ET\n",
        x,
        y - 10.0,
        name_line
    ));

    // Title (if provided)
    if let Some(t) = title {
        if !t.is_empty() {
            ops.push_str(&format!(
                "BT 0 0 0.6 rg /Helv 7 Tf {} {} Td ({}) Tj ET\n",
                x,
                y - 20.0,
                escape_pdf(t)
            ));
        }
    }

    ops
}

// ── date stamping ─────────────────────────────────────────────────────────────

/// Stamp just a date on a known signature line.
/// Searches for the first "Date" or "DATE" label and writes today's date there.
pub fn stamp_date_only(
    pdf_path: &Path,
    output_dir: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    std::fs::create_dir_all(output_dir)?;
    let mut doc = Document::load(pdf_path)?;
    let date_str = Local::now().format("%m/%d/%Y").to_string();

    // We look for AcroForm fields named "Date" first
    let date_page: usize = 0;
    let mut date_x = 300.0_f64;
    let mut date_y = 200.0_f64;
    let mut found = false;

    {
        let catalog = doc.catalog().ok().cloned();
        if let Some(catalog) = catalog {
            if let Ok(acroform_obj) = catalog.get(b"AcroForm").cloned() {
                if let Ok((_, Object::Dictionary(acroform))) = doc.dereference(&acroform_obj) {
                    if let Ok(Object::Array(fields)) = acroform.get(b"Fields").cloned() {
                        for fr in &fields {
                            if let Ok((_, Object::Dictionary(fd))) = doc.dereference(fr) {
                                let name = match fd.get(b"T") {
                                    Ok(Object::String(b, _)) => {
                                        String::from_utf8(b.clone()).unwrap_or_default()
                                    }
                                    _ => String::new(),
                                };
                                if name.to_lowercase().contains("date") {
                                    if let Ok(Object::Array(rect)) = fd.get(b"Rect").cloned() {
                                        let nums: Vec<f64> = rect
                                            .iter()
                                            .filter_map(|o| match o {
                                                Object::Real(f) => Some(*f as f64),
                                                Object::Integer(i) => Some(*i as f64),
                                                _ => None,
                                            })
                                            .collect();
                                        if nums.len() == 4 {
                                            date_x = nums[0];
                                            date_y = nums[1];
                                            found = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Fallback: use a generic bottom-third placement if no AcroForm date field
    if !found {
        let pages: Vec<(u32, lopdf::ObjectId)> = doc.get_pages().into_iter().collect();
        if let Some((_, page_id)) = pages.first() {
            let ph = get_page_height(&doc, *page_id);
            date_y = ph * 0.30; // ~30% up from bottom
            date_x = 350.0;
        }
    }

    let pages: Vec<(u32, lopdf::ObjectId)> = doc.get_pages().into_iter().collect();
    if let Some((_, page_id)) = pages.get(date_page) {
        ensure_sig_fonts(&mut doc, *page_id);
        let ops = format!(
            "BT 0 0 0.6 rg /Helv 10 Tf {} {} Td ({}) Tj ET\n",
            date_x,
            date_y,
            escape_pdf(&date_str)
        );
        append_stream(&mut doc, *page_id, ops);
    }

    let dst = output_dir.join("dated.pdf");
    doc.save(&dst)?;
    Ok(dst)
}

// ── main signature stamp ──────────────────────────────────────────────────────

/// Sign a PDF with the given parameters.
///
/// If `params.x` and `params.y` are supplied, the signature is placed there
/// exactly on `params.page`. Otherwise the auto-detected locations from
/// `detect::detect_sig_locations` are used.
pub fn sign_pdf(
    pdf_path: &Path,
    output_dir: &Path,
    params: &StampParams,
    sig_locations: &[SigLocation],
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    std::fs::create_dir_all(output_dir)?;
    let mut doc = Document::load(pdf_path)?;
    let pages: Vec<(u32, lopdf::ObjectId)> = doc.get_pages().into_iter().collect();
    let date_str = Local::now().format("%m/%d/%Y").to_string();

    // Determine placement(s)
    let placements: Vec<(usize, f64, f64)> = if params.x.is_some() && params.y.is_some() {
        // Explicit position — single stamp on the requested page
        vec![(params.page, params.x.unwrap(), params.y.unwrap())]
    } else if !sig_locations.is_empty() {
        // Auto-detected — stamp every detected location on the target page (or all pages)
        sig_locations
            .iter()
            .map(|loc| {
                // Use the bottom-left corner of the detected bbox as our anchor
                (loc.page, loc.bbox.0, loc.bbox.1)
            })
            .collect()
    } else {
        // No detection and no explicit coords — default to page 0 at a safe position
        let default_y = if let Some((_, pid)) = pages.first() {
            get_page_height(&doc, *pid) * 0.25
        } else {
            200.0
        };
        vec![(0, 72.0, default_y)]
    };

    for (page_idx, x, y) in &placements {
        // Apply page_range filter: page_range is 1-based inclusive
        if let Some((range_start, range_end)) = params.page_range {
            let one_based = page_idx + 1;
            if one_based < range_start || one_based > range_end {
                continue;
            }
        }
        // pages is 1-indexed in lopdf
        let page_num = (*page_idx as u32) + 1;
        if let Some((_, page_id)) = pages.iter().find(|(pn, _)| *pn == page_num) {
            ensure_sig_fonts(&mut doc, *page_id);
            let ops = build_stamp_ops(
                &params.signer,
                params.title.as_deref(),
                *x,
                *y,
                &date_str,
                params.initials,
            );
            append_stream(&mut doc, *page_id, ops);
        }
    }

    let dst = output_dir.join("signed.pdf");
    doc.save(&dst)?;
    Ok(dst)
}

// ── signature verification ────────────────────────────────────────────────────

/// Verify whether a PDF already has a /s/ signature stamped in it.
///
/// Uses two strategies:
///   1. Scan all stream objects via lopdf for the `/s/` byte sequence.
///   2. Scan the raw PDF file bytes for `/s/` inside string literals.
///
/// Returns `Ok(true)` if found, `Ok(false)` if not, `Err(...)` on I/O failure.
pub fn verify_pdf_has_signature(
    pdf_path: &Path,
) -> Result<bool, Box<dyn std::error::Error>> {
    // Strategy 1: lopdf Document — scan all stream objects
    if let Ok(doc) = Document::load(pdf_path) {
        let sig_bytes = b"/s/";
        for (_, obj) in doc.objects.iter() {
            if let Object::Stream(stream) = obj {
                if stream.content.windows(sig_bytes.len()).any(|w| w == sig_bytes) {
                    return Ok(true);
                }
                let mut s = stream.clone();
                if s.decompress().is_ok() && s.content.windows(sig_bytes.len()).any(|w| w == sig_bytes) {
                    return Ok(true);
                }
            }
        }

        // Also try extract_text
        let pages: Vec<(u32, lopdf::ObjectId)> = doc.get_pages().into_iter().collect();
        for (page_num, _) in &pages {
            if let Ok(text) = doc.extract_text(&[*page_num]) {
                if text.contains("/s/") {
                    return Ok(true);
                }
            }
        }
    }

    // Strategy 2: raw file bytes scan — our stamp ops put `/s/` inside PDF string
    // literals as `(/s/ Name) Tj` so the bytes appear literally in the file
    let raw = std::fs::read(pdf_path)?;
    let sig_bytes = b"/s/";
    if raw.windows(sig_bytes.len()).any(|w| w == sig_bytes) {
        return Ok(true);
    }

    Ok(false)
}

// ── batch signing ─────────────────────────────────────────────────────────────

/// Sign every PDF in `dir` and write results to `output_dir`.
///
/// Returns the list of output paths (one per successfully signed PDF).
pub fn sign_batch(
    dir: &Path,
    output_dir: &Path,
    params: &StampParams,
) -> Vec<(PathBuf, Result<PathBuf, String>)> {
    use crate::detect::detect_sig_locations;
    use rayon::prelude::*;

    let entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| {
                    p.extension()
                        .and_then(|e| e.to_str())
                        .map(|e| e.eq_ignore_ascii_case("pdf"))
                        .unwrap_or(false)
                })
                .collect()
        })
        .unwrap_or_default();

    entries
        .par_iter()
        .map(|pdf_path| {
            let stem = pdf_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("output");
            let out_dir = output_dir.join(stem);
            let locs = detect_sig_locations(pdf_path);
            let result = sign_pdf(pdf_path, &out_dir, params, &locs)
                .map_err(|e| e.to_string());
            (pdf_path.clone(), result)
        })
        .collect()
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_pdf_parens() {
        let escaped = escape_pdf("Hoags (Inc.)");
        assert_eq!(escaped, "Hoags \\(Inc.\\)");
    }

    #[test]
    fn test_escape_pdf_backslash() {
        let escaped = escape_pdf("path\\to\\file");
        assert_eq!(escaped, "path\\\\to\\\\file");
    }

    #[test]
    fn test_build_stamp_ops_contains_slash_s() {
        let ops = build_stamp_ops("Collin Hoag", Some("President"), 72.0, 200.0, "04/08/2026", false);
        assert!(ops.contains("/s/ Collin Hoag"), "ops should contain /s/ signature");
    }

    #[test]
    fn test_build_stamp_ops_initials() {
        let ops = build_stamp_ops("Collin Hoag", None, 72.0, 200.0, "04/08/2026", true);
        // Initials of "Collin Hoag" = "CH"
        assert!(ops.contains("/s/ CH"), "ops should contain /s/ CH when initials=true");
        assert!(!ops.contains("/s/ Collin Hoag"), "should not use full name when initials=true");
    }

    #[test]
    fn test_name_initials() {
        assert_eq!(name_initials("Collin Hoag"), "CH");
        assert_eq!(name_initials("John A Smith"), "JAS");
        assert_eq!(name_initials("Alice"), "A");
    }

    #[test]
    fn test_build_stamp_ops_contains_date() {
        let ops = build_stamp_ops("Collin Hoag", None, 72.0, 200.0, "04/08/2026", false);
        assert!(ops.contains("04/08/2026"), "ops should contain the date");
    }

    #[test]
    fn test_build_stamp_ops_contains_name_below() {
        let ops = build_stamp_ops("Collin Hoag", None, 72.0, 200.0, "04/08/2026", false);
        // Name line should appear at y - 10
        assert!(ops.contains("190"), "ops should contain printed name at y-10");
    }

    #[test]
    fn test_build_stamp_ops_contains_title() {
        let ops = build_stamp_ops("Collin Hoag", Some("President"), 72.0, 200.0, "04/08/2026", false);
        assert!(ops.contains("President"), "ops should contain the title");
    }

    #[test]
    fn test_build_stamp_ops_no_title_when_none() {
        let ops = build_stamp_ops("Collin Hoag", None, 72.0, 200.0, "04/08/2026", false);
        // When title is None, no title line should appear
        assert!(
            !ops.contains("President"),
            "ops should not contain a title when None given"
        );
    }

    #[test]
    fn test_build_stamp_ops_blue_ink() {
        let ops = build_stamp_ops("Collin Hoag", None, 72.0, 200.0, "04/08/2026", false);
        assert!(
            ops.contains("0 0 0.6 rg"),
            "ops should use blue ink (0 0 0.6 rg)"
        );
    }

    #[test]
    fn test_build_stamp_ops_italic_font() {
        let ops = build_stamp_ops("Collin Hoag", None, 72.0, 200.0, "04/08/2026", false);
        assert!(
            ops.contains("HelvO"),
            "signature line should use Helvetica-Oblique (HelvO)"
        );
    }

    #[test]
    fn test_sign_pdf_nonexistent() {
        use tempfile::TempDir;
        let dir = TempDir::new().unwrap();
        let params = StampParams {
            signer: "Collin Hoag".to_string(),
            title: Some("President".to_string()),
            page: 0,
            x: Some(72.0),
            y: Some(200.0),
            initials: false,
            page_range: None,
        };
        let result = sign_pdf(
            Path::new("does_not_exist.pdf"),
            dir.path(),
            &params,
            &[],
        );
        assert!(result.is_err(), "should fail on nonexistent PDF");
    }


    // ── create a minimal valid single-page PDF using lopdf API ────────────────
    // Using lopdf's own API ensures save/load round-trips correctly.

    fn make_test_pdf(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
        use lopdf::{Document, Object, Stream, dictionary};
        use lopdf::content::{Content, Operation};

        let mut doc = Document::with_version("1.5");

        let helv_id = doc.add_object(dictionary! {
            "Type"     => "Font",
            "Subtype"  => "Type1",
            "BaseFont" => "Helvetica",
        });
        let resources_id = doc.add_object(dictionary! {
            "Font" => dictionary! { "Helv" => helv_id }
        });

        let ops = vec![
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec![Object::Name(b"Helv".to_vec()), 12.into()]),
            Operation::new("Td", vec![72.into(), 700.into()]),
            Operation::new("Tj", vec![Object::string_literal("Signature: ________")]),
            Operation::new("ET", vec![]),
        ];
        let content_bytes = Content { operations: ops }.encode().expect("encode");
        let stream = Stream::new(dictionary! {}, content_bytes);
        let content_id = doc.add_object(stream);

        let page_id = doc.add_object(dictionary! {
            "Type"      => "Page",
            "MediaBox"  => vec![0.into(), 0.into(), 612.into(), 792.into()],
            "Resources" => resources_id,
            "Contents"  => content_id,
        });

        let pages_id = doc.add_object(dictionary! {
            "Type"  => "Pages",
            "Kids"  => vec![page_id.into()],
            "Count" => 1i64,
        });
        doc.get_object_mut(page_id).unwrap()
            .as_dict_mut().unwrap()
            .set("Parent", pages_id);

        let catalog_id = doc.add_object(dictionary! {
            "Type"  => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);

        let path = dir.join(name);
        doc.save(&path).expect("save test pdf");
        path
    }

    #[test]
    fn test_sign_pdf_creates_output() {
        use tempfile::TempDir;
        let dir = TempDir::new().unwrap();
        let pdf_path = make_test_pdf(dir.path(), "test.pdf");

        let out_dir = dir.path().join("out");
        let params = StampParams {
            signer: "Collin Hoag".to_string(),
            title: Some("President".to_string()),
            page: 0,
            x: Some(72.0),
            y: Some(200.0),
            initials: false,
            page_range: None,
        };
        let result = sign_pdf(&pdf_path, &out_dir, &params, &[]);
        assert!(result.is_ok(), "sign_pdf failed: {:?}", result.err());
        assert!(result.unwrap().exists());
    }

    #[test]
    fn test_stamp_date_only_creates_output() {
        use tempfile::TempDir;
        let dir = TempDir::new().unwrap();
        let pdf_path = make_test_pdf(dir.path(), "test.pdf");

        let out_dir = dir.path().join("out");
        let result = stamp_date_only(&pdf_path, &out_dir);
        assert!(result.is_ok(), "stamp_date_only failed: {:?}", result.err());
        assert!(result.unwrap().exists());
    }

    /// Sign a PDF then verify that /s/ signature text appears in the output.
    #[test]
    fn test_sign_pdf_and_verify_signature_present() {
        use tempfile::TempDir;
        let dir = TempDir::new().unwrap();
        let pdf_path = make_test_pdf(dir.path(), "to_sign.pdf");
        let out_dir = dir.path().join("signed_out");
        let params = StampParams {
            signer: "Collin Hoag".to_string(),
            title: Some("President".to_string()),
            page: 0,
            x: Some(72.0),
            y: Some(200.0),
            initials: false,
            page_range: None,
        };
        let signed_path = sign_pdf(&pdf_path, &out_dir, &params, &[])
            .expect("sign_pdf should succeed");
        assert!(signed_path.exists(), "signed PDF should exist");

        let has_sig = verify_pdf_has_signature(&signed_path)
            .expect("verify should succeed");
        assert!(has_sig, "signed PDF should contain /s/ signature");
    }

    /// An unsigned PDF should have verify return false.
    #[test]
    fn test_verify_unsigned_pdf_returns_false() {
        use tempfile::TempDir;
        let dir = TempDir::new().unwrap();
        let pdf_path = make_test_pdf(dir.path(), "unsigned.pdf");

        let has_sig = verify_pdf_has_signature(&pdf_path)
            .expect("verify should succeed on a valid PDF");
        assert!(!has_sig, "unsigned PDF should not contain /s/ signature");
    }

    /// Sign with initials flag and verify initials appear in the content.
    #[test]
    fn test_sign_pdf_with_initials() {
        use tempfile::TempDir;
        let dir = TempDir::new().unwrap();
        let pdf_path = make_test_pdf(dir.path(), "initials.pdf");
        let out_dir = dir.path().join("init_out");
        let params = StampParams {
            signer: "Collin Hoag".to_string(),
            title: None,
            page: 0,
            x: Some(72.0),
            y: Some(200.0),
            initials: true,
            page_range: None,
        };
        let signed_path = sign_pdf(&pdf_path, &out_dir, &params, &[])
            .expect("sign_pdf with initials should succeed");

        // Scan the raw signed PDF bytes for the initials stamp.
        let raw_bytes = std::fs::read(&signed_path).expect("read signed pdf");
        let sig_bytes = b"/s/ CH";
        let found = raw_bytes.windows(sig_bytes.len()).any(|w| w == sig_bytes);
        assert!(found, "signed PDF should contain /s/ CH (initials of Collin Hoag)");
    }
}
