use chrono::Local;
use lopdf::{Document, Object, ObjectId};
use std::path::Path;

use crate::context::{Clin, ProposalContext, SolicitationMeta};
use crate::templates::{
    self, Color, HRule, Rect, TextLine, PAGE_H, PAGE_W, MARGIN,
};

// ─── Solicitation parsing ─────────────────────────────────────────────────────

/// Very lightweight text extraction from PDF: uses lopdf to pull raw text
/// objects and runs regex-based heuristics to identify sol number, CO name,
/// due date, and issuing agency.
pub fn extract_sol_meta(pdf_path: &Path) -> SolicitationMeta {
    let doc = match Document::load(pdf_path) {
        Ok(d) => d,
        Err(_) => return SolicitationMeta::default(),
    };

    let mut all_text = String::new();
    for (_, obj) in doc.objects.iter() {
        if let Ok(stream) = obj.as_stream() {
            if let Ok(decoded) = stream.decode_content() {
                for op in &decoded.operations {
                    for operand in &op.operands {
                        if let Ok(s) = operand.as_str() {
                            all_text.push(' ');
                            all_text.push_str(&String::from_utf8_lossy(s));
                        }
                    }
                }
            }
        }
    }

    let text_lc = all_text.to_lowercase();
    let mut meta = SolicitationMeta::default();

    // Sol number: look for patterns like 12444626P0025 or SOL-2026-001
    for line in all_text.split_whitespace() {
        if looks_like_sol_number(line) && meta.number.is_empty() {
            meta.number = line.to_string();
        }
    }

    // CO name: line after "contracting officer" or "issued by"
    if let Some(idx) = text_lc.find("contracting officer") {
        let snippet = &all_text[idx..idx.min(idx + 80)];
        let after: String = snippet.chars().skip("contracting officer".len()).collect();
        let name = after.trim().split('\n').next().unwrap_or("").trim().to_string();
        if !name.is_empty() && name.len() < 50 {
            meta.co_name = name;
        }
    }

    // Due date: look for "due" + something that looks like a date
    if let Some(idx) = text_lc.find("due date") {
        let snippet: String = all_text[idx..].chars().take(60).collect();
        let date_part = snippet.trim_start_matches(|c: char| !c.is_ascii_digit()).trim().to_string();
        if !date_part.is_empty() {
            meta.due_date = date_part.chars().take(20).collect();
        }
    }

    // Agency: look for USDA, USFS, BLM, DOD patterns
    if text_lc.contains("forest service") || text_lc.contains("usfs") {
        meta.agency = "USDA Forest Service".to_string();
    } else if text_lc.contains("bureau of land management") || text_lc.contains("blm") {
        meta.agency = "Bureau of Land Management".to_string();
    } else if text_lc.contains("department of defense") || text_lc.contains("dod") {
        meta.agency = "Department of Defense".to_string();
    } else if text_lc.contains("department of the army") {
        meta.agency = "Department of the Army".to_string();
    }

    meta
}

fn looks_like_sol_number(s: &str) -> bool {
    // Government solicitation numbers tend to be 10-20 chars mixing digits, letters, dashes
    if s.len() < 8 || s.len() > 25 { return false; }
    let has_digit = s.chars().any(|c| c.is_ascii_digit());
    let has_alpha = s.chars().any(|c| c.is_ascii_alphabetic());
    let all_valid = s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    has_digit && has_alpha && all_valid
}

// ─── Cover letter PDF ─────────────────────────────────────────────────────────

pub fn build_cover_letter(ctx: &ProposalContext, co_name_override: Option<&str>) -> Document {
    let mut doc = Document::with_version("1.4");
    let mut pages_kids: Vec<Object> = Vec::new();

    let today = Local::now().format("%B %d, %Y").to_string();
    let co_name = co_name_override
        .filter(|s| !s.is_empty())
        .or_else(|| {
            let n = ctx.solicitation.co_name.as_str();
            if n.is_empty() { None } else { Some(n) }
        })
        .unwrap_or("Contracting Officer");

    let sol_ref = if ctx.solicitation.number.is_empty() {
        "Solicitation".to_string()
    } else {
        format!("Solicitation No. {}", ctx.solicitation.number)
    };

    let title = if ctx.solicitation.title.is_empty() {
        "Services".to_string()
    } else {
        ctx.solicitation.title.clone()
    };

    let mut rects: Vec<Rect> = Vec::new();
    let mut hrules: Vec<HRule> = Vec::new();
    let mut lines: Vec<TextLine> = Vec::new();

    // Blue header bar
    rects.push(Rect {
        x: 0.0,
        y: PAGE_H - 80.0,
        w: PAGE_W,
        h: 80.0,
        fill: (Color::HOAGS_BLUE.r, Color::HOAGS_BLUE.g, Color::HOAGS_BLUE.b),
    });

    // Company name in header
    lines.push(TextLine::new(
        MARGIN, PAGE_H - 38.0, 20.0, "F2",
        (Color::WHITE.r, Color::WHITE.g, Color::WHITE.b),
        &ctx.company.name,
    ));
    // CAGE / UEI sub-line
    let cage_line = format!("CAGE: {} | UEI: {}", ctx.company.cage, ctx.company.uei);
    lines.push(TextLine::new(
        MARGIN, PAGE_H - 60.0, 10.0, "F1",
        (0.85, 0.92, 1.0),
        cage_line,
    ));

    let mut y = PAGE_H - 110.0;

    // Date
    push_text(&mut lines, MARGIN, y, 11.0, "F1", Color::BLACK, &today);
    y -= 24.0;

    // Addressee block
    let addr_to = format!("{}", co_name);
    push_text(&mut lines, MARGIN, y, 11.0, "F2", Color::BLACK, &addr_to);
    y -= 14.0;
    if !ctx.solicitation.agency.is_empty() {
        push_text(&mut lines, MARGIN, y, 11.0, "F1", Color::BLACK, &ctx.solicitation.agency);
        y -= 14.0;
    }
    y -= 10.0;

    // Subject line
    let subject = format!("RE: Offer in Response to {}", sol_ref);
    push_text(&mut lines, MARGIN, y, 11.0, "F2", Color::HOAGS_BLUE, &subject);
    y -= 10.0;

    hrules.push(HRule {
        x: MARGIN,
        y,
        w: PAGE_W - 2.0 * MARGIN,
        width_pts: 0.75,
        color: (Color::HOAGS_BLUE.r, Color::HOAGS_BLUE.g, Color::HOAGS_BLUE.b),
    });
    y -= 18.0;

    // Salutation
    push_text(&mut lines, MARGIN, y, 11.0, "F1", Color::BLACK,
        &format!("Dear {},", co_name));
    y -= 22.0;

    // Body paragraphs
    let body_width = PAGE_W - 2.0 * MARGIN;
    let para1 = format!(
        "{} is pleased to submit this offer in response to {} for {}. \
        We have carefully reviewed all solicitation requirements and are fully capable of \
        meeting and exceeding the performance standards outlined therein.",
        ctx.company.name, sol_ref, title
    );
    y = wrap_text(&mut lines, MARGIN, y, 11.0, body_width, &para1, 16.0);
    y -= 12.0;

    let para2 = format!(
        "As a verified SAM.gov vendor (CAGE: {}, UEI: {}), {} brings demonstrated \
        experience in federal service delivery. Our team is committed to quality, \
        compliance, and timely performance.",
        ctx.company.cage, ctx.company.uei, ctx.company.name
    );
    y = wrap_text(&mut lines, MARGIN, y, 11.0, body_width, &para2, 16.0);
    y -= 12.0;

    // Past performance summary
    if !ctx.past_performance.is_empty() {
        let pp_intro = format!(
            "Our past performance includes {} federal contract(s) demonstrating relevant \
            experience in similar scope and complexity:",
            ctx.past_performance.len()
        );
        y = wrap_text(&mut lines, MARGIN, y, 11.0, body_width, &pp_intro, 16.0);
        y -= 8.0;
        for pp in &ctx.past_performance {
            let line = format!(
                "  - {} ({}): ${:.0} | {}",
                pp.title, pp.contract, pp.value, pp.period
            );
            y = wrap_text(&mut lines, MARGIN + 12.0, y, 10.0, body_width - 12.0, &line, 14.0);
        }
        y -= 12.0;
    }

    let para3 = "We understand the importance of this requirement to your mission and take our \
        performance obligations seriously. We welcome the opportunity to discuss our offer \
        and are available at your convenience for any clarifications.";
    y = wrap_text(&mut lines, MARGIN, y, 11.0, body_width, para3, 16.0);
    y -= 20.0;

    // Closing
    push_text(&mut lines, MARGIN, y, 11.0, "F1", Color::BLACK, "Respectfully submitted,");
    y -= 40.0;

    // Signature block
    push_text(&mut lines, MARGIN, y, 11.0, "F2", Color::BLACK, &ctx.signer.name);
    y -= 14.0;
    push_text(&mut lines, MARGIN, y, 11.0, "F1", Color::DARK_GRAY, &ctx.signer.title);
    y -= 14.0;
    push_text(&mut lines, MARGIN, y, 11.0, "F1", Color::DARK_GRAY, &ctx.company.name);
    y -= 14.0;
    if !ctx.signer.phone.is_empty() {
        push_text(&mut lines, MARGIN, y, 11.0, "F1", Color::DARK_GRAY, &ctx.signer.phone);
        y -= 14.0;
    }
    if !ctx.signer.email.is_empty() {
        push_text(&mut lines, MARGIN, y, 11.0, "F1", Color::DARK_GRAY, &ctx.signer.email);
    }

    // Footer
    hrules.push(HRule {
        x: MARGIN,
        y: 50.0,
        w: PAGE_W - 2.0 * MARGIN,
        width_pts: 0.5,
        color: (Color::LIGHT_GRAY.r, Color::LIGHT_GRAY.r, Color::LIGHT_GRAY.r),
    });
    let footer = format!(
        "{} | CAGE {} | UEI {} | Page 1 of 1",
        ctx.company.name, ctx.company.cage, ctx.company.uei
    );
    push_text(&mut lines, MARGIN, 36.0, 8.0, "F1", Color::DARK_GRAY, &footer);

    let stream = templates::build_stream(&rects, &hrules, &lines);
    let stream_id = doc.add_object(Object::Stream(stream));
    let page_id = templates::add_page(&mut doc, stream_id);
    pages_kids.push(Object::Reference(page_id));

    attach_pages(&mut doc, pages_kids)
}

// ─── Past Performance volume ──────────────────────────────────────────────────

pub fn build_past_performance(ctx: &ProposalContext) -> Document {
    let mut doc = Document::with_version("1.4");
    let mut pages_kids: Vec<Object> = Vec::new();

    let mut rects: Vec<Rect> = Vec::new();
    let mut hrules: Vec<HRule> = Vec::new();
    let mut lines: Vec<TextLine> = Vec::new();

    // Blue header
    rects.push(Rect {
        x: 0.0,
        y: PAGE_H - 70.0,
        w: PAGE_W,
        h: 70.0,
        fill: (Color::HOAGS_BLUE.r, Color::HOAGS_BLUE.g, Color::HOAGS_BLUE.b),
    });
    lines.push(TextLine::new(
        MARGIN, PAGE_H - 30.0, 18.0, "F2",
        (Color::WHITE.r, Color::WHITE.g, Color::WHITE.b),
        "Past Performance Volume",
    ));
    lines.push(TextLine::new(
        MARGIN, PAGE_H - 52.0, 10.0, "F1",
        (0.85, 0.92, 1.0),
        &ctx.company.name,
    ));

    let mut y = PAGE_H - 110.0;

    // Introductory text
    let intro = format!(
        "{} submits the following past performance references demonstrating relevant \
        experience with federal contracts of similar scope, complexity, and value. \
        All references are available for verification upon request.",
        ctx.company.name
    );
    let body_width = PAGE_W - 2.0 * MARGIN;
    y = wrap_text(&mut lines, MARGIN, y, 11.0, body_width, &intro, 16.0);
    y -= 20.0;

    if ctx.past_performance.is_empty() {
        push_text(&mut lines, MARGIN, y, 11.0, "F3", Color::DARK_GRAY,
            "No past performance records provided.");
    } else {
        for (i, pp) in ctx.past_performance.iter().enumerate() {
            if y < 150.0 {
                // Push current page, start new page
                let stream = templates::build_stream(&rects, &hrules, &lines);
                let stream_id = doc.add_object(Object::Stream(stream));
                let page_id = templates::add_page(&mut doc, stream_id);
                pages_kids.push(Object::Reference(page_id));
                rects = Vec::new();
                hrules = Vec::new();
                lines = Vec::new();
                y = PAGE_H - MARGIN;
            }

            // Record header bar (light blue-gray)
            rects.push(Rect {
                x: MARGIN,
                y: y - 4.0,
                w: PAGE_W - 2.0 * MARGIN,
                h: 20.0,
                fill: (0.88, 0.92, 0.97),
            });
            let record_header = format!("{}. {}", i + 1, pp.title);
            push_text(&mut lines, MARGIN + 4.0, y + 8.0, 12.0, "F2", Color::HOAGS_BLUE, &record_header);
            y -= 28.0;

            // Fields
            let fields = [
                ("Contract Number:", &pp.contract),
                ("Agency:", &pp.agency),
                ("Period of Performance:", &pp.period),
            ];
            for (label, value) in &fields {
                if !value.is_empty() {
                    push_text(&mut lines, MARGIN + 8.0, y, 10.0, "F2", Color::BLACK, label);
                    push_text(&mut lines, MARGIN + 165.0, y, 10.0, "F1", Color::BLACK, value);
                    y -= 14.0;
                }
            }

            // Contract value
            push_text(&mut lines, MARGIN + 8.0, y, 10.0, "F2", Color::BLACK, "Contract Value:");
            push_text(&mut lines, MARGIN + 165.0, y, 10.0, "F1", Color::BLACK,
                &format!("${:.2}", pp.value));
            y -= 14.0;

            // Description
            if !pp.description.is_empty() {
                push_text(&mut lines, MARGIN + 8.0, y, 10.0, "F2", Color::BLACK, "Scope:");
                y -= 13.0;
                y = wrap_text(&mut lines, MARGIN + 16.0, y, 10.0, body_width - 24.0, &pp.description, 14.0);
            }

            // POC
            if !pp.poc_name.is_empty() {
                push_text(&mut lines, MARGIN + 8.0, y, 10.0, "F2", Color::BLACK, "POC:");
                let poc = if pp.poc_phone.is_empty() {
                    pp.poc_name.clone()
                } else {
                    format!("{} — {}", pp.poc_name, pp.poc_phone)
                };
                push_text(&mut lines, MARGIN + 165.0, y, 10.0, "F1", Color::BLACK, &poc);
                y -= 14.0;
            }

            // Separator
            hrules.push(HRule {
                x: MARGIN,
                y: y - 4.0,
                w: PAGE_W - 2.0 * MARGIN,
                width_pts: 0.5,
                color: (0.75, 0.75, 0.85),
            });
            y -= 18.0;
        }
    }

    // Footer
    footer_line(&mut hrules, &mut lines, ctx, 1);

    let stream = templates::build_stream(&rects, &hrules, &lines);
    let stream_id = doc.add_object(Object::Stream(stream));
    let page_id = templates::add_page(&mut doc, stream_id);
    pages_kids.push(Object::Reference(page_id));

    attach_pages(&mut doc, pages_kids)
}

// ─── Price schedule PDF ───────────────────────────────────────────────────────

pub fn build_price_schedule(ctx: &ProposalContext) -> Document {
    let mut doc = Document::with_version("1.4");
    let mut pages_kids: Vec<Object> = Vec::new();

    let mut rects: Vec<Rect> = Vec::new();
    let mut hrules: Vec<HRule> = Vec::new();
    let mut lines: Vec<TextLine> = Vec::new();

    // Header
    rects.push(Rect {
        x: 0.0,
        y: PAGE_H - 70.0,
        w: PAGE_W,
        h: 70.0,
        fill: (Color::HOAGS_BLUE.r, Color::HOAGS_BLUE.g, Color::HOAGS_BLUE.b),
    });
    lines.push(TextLine::new(
        MARGIN, PAGE_H - 30.0, 18.0, "F2",
        (Color::WHITE.r, Color::WHITE.g, Color::WHITE.b),
        "Price Schedule",
    ));
    let sol_ref = if ctx.solicitation.number.is_empty() {
        ctx.company.name.clone()
    } else {
        format!("{} | Sol. {}", ctx.company.name, ctx.solicitation.number)
    };
    lines.push(TextLine::new(
        MARGIN, PAGE_H - 52.0, 10.0, "F1",
        (0.85, 0.92, 1.0),
        &sol_ref,
    ));

    let mut y = PAGE_H - 100.0;

    // Table column positions
    let col_clin  = MARGIN;
    let col_desc  = MARGIN + 50.0;
    let col_qty   = MARGIN + 260.0;
    let col_unit  = MARGIN + 310.0;
    let col_up    = MARGIN + 370.0;
    let col_total = MARGIN + 445.0;
    let table_w   = PAGE_W - 2.0 * MARGIN;

    // Table header row
    rects.push(Rect {
        x: MARGIN,
        y: y - 4.0,
        w: table_w,
        h: 20.0,
        fill: (Color::HOAGS_BLUE.r, Color::HOAGS_BLUE.g, Color::HOAGS_BLUE.b),
    });
    let header_cols = [
        (col_clin,  "CLIN"),
        (col_desc,  "Description"),
        (col_qty,   "Qty"),
        (col_unit,  "Unit"),
        (col_up,    "Unit Price"),
        (col_total, "Total"),
    ];
    for (cx, label) in &header_cols {
        lines.push(TextLine::new(*cx + 2.0, y + 8.0, 9.0, "F2",
            (Color::WHITE.r, Color::WHITE.g, Color::WHITE.b), *label));
    }
    y -= 26.0;

    // CLIN rows — use provided CLINs or generate a default single-CLIN from rates
    let clins: Vec<Clin> = if ctx.pricing.clins.is_empty() {
        // Generate a placeholder CLIN from labor rate
        let hourly = ctx.pricing.labor_rate;
        let burdened = hourly * (1.0 + ctx.pricing.overhead) * (1.0 + ctx.pricing.profit);
        vec![Clin {
            number: "0001".into(),
            description: "Base Year Services".into(),
            quantity: 2080.0,
            unit: "Hour".into(),
            unit_price: (burdened * 100.0).round() / 100.0,
        }]
    } else {
        ctx.pricing.clins.clone()
    };

    let mut grand_total = 0.0_f64;
    let mut row_odd = false;

    for clin in &clins {
        if row_odd {
            rects.push(Rect {
                x: MARGIN,
                y: y - 4.0,
                w: table_w,
                h: 18.0,
                fill: (0.95, 0.96, 0.99),
            });
        }
        row_odd = !row_odd;

        let total = clin.total();
        grand_total += total;

        let text_cols: &[(&str, f64)] = &[
            ("F2", col_clin),
        ];
        let _ = text_cols;
        lines.push(TextLine::new(col_clin + 2.0, y + 6.0, 9.0, "F2", (0.0, 0.0, 0.0), &clin.number));

        // Truncate description if too long
        let desc_display = if clin.description.len() > 35 {
            format!("{}...", &clin.description[..32])
        } else {
            clin.description.clone()
        };
        lines.push(TextLine::new(col_desc + 2.0, y + 6.0, 9.0, "F1", (0.0, 0.0, 0.0), &desc_display));
        lines.push(TextLine::new(col_qty + 2.0, y + 6.0, 9.0, "F1", (0.0, 0.0, 0.0), &format!("{:.1}", clin.quantity)));
        lines.push(TextLine::new(col_unit + 2.0, y + 6.0, 9.0, "F1", (0.0, 0.0, 0.0), &clin.unit));
        lines.push(TextLine::new(col_up + 2.0, y + 6.0, 9.0, "F1", (0.0, 0.0, 0.0), &format!("${:.2}", clin.unit_price)));
        lines.push(TextLine::new(col_total + 2.0, y + 6.0, 9.0, "F1", (0.0, 0.0, 0.0), &format!("${:.2}", total)));
        y -= 18.0;
    }

    // Grand total row
    hrules.push(HRule {
        x: MARGIN,
        y,
        w: table_w,
        width_pts: 1.0,
        color: (Color::HOAGS_BLUE.r, Color::HOAGS_BLUE.g, Color::HOAGS_BLUE.b),
    });
    y -= 6.0;
    rects.push(Rect {
        x: MARGIN,
        y: y - 4.0,
        w: table_w,
        h: 22.0,
        fill: (0.87, 0.91, 0.97),
    });
    let blue = (Color::HOAGS_BLUE.r, Color::HOAGS_BLUE.g, Color::HOAGS_BLUE.b);
    lines.push(TextLine::new(col_desc + 2.0, y + 8.0, 10.0, "F2", blue, "GRAND TOTAL"));
    lines.push(TextLine::new(col_total + 2.0, y + 8.0, 10.0, "F2", blue,
        &format!("${:.2}", grand_total)));
    y -= 30.0;

    // Pricing notes
    y -= 10.0;
    push_text(&mut lines, MARGIN, y, 9.0, "F3", Color::DARK_GRAY,
        "Pricing Notes:");
    y -= 13.0;
    let note = format!(
        "Base labor rate: ${:.2}/hr | Overhead: {:.0}% | Profit: {:.0}% | \
        All prices are fully burdened.",
        ctx.pricing.labor_rate,
        ctx.pricing.overhead * 100.0,
        ctx.pricing.profit * 100.0,
    );
    wrap_text(&mut lines, MARGIN, y, 9.0, PAGE_W - 2.0 * MARGIN, &note, 13.0);

    footer_line(&mut hrules, &mut lines, ctx, 1);

    let stream = templates::build_stream(&rects, &hrules, &lines);
    let stream_id = doc.add_object(Object::Stream(stream));
    let page_id = templates::add_page(&mut doc, stream_id);
    pages_kids.push(Object::Reference(page_id));

    attach_pages(&mut doc, pages_kids)
}

// ─── Technical Approach volume ────────────────────────────────────────────────

pub fn build_technical_approach(ctx: &ProposalContext) -> Document {
    let mut doc = Document::with_version("1.4");
    let mut pages_kids: Vec<Object> = Vec::new();

    let mut rects: Vec<Rect> = Vec::new();
    let mut hrules: Vec<HRule> = Vec::new();
    let mut lines: Vec<TextLine> = Vec::new();

    // Header
    rects.push(Rect {
        x: 0.0,
        y: PAGE_H - 70.0,
        w: PAGE_W,
        h: 70.0,
        fill: (Color::HOAGS_BLUE.r, Color::HOAGS_BLUE.g, Color::HOAGS_BLUE.b),
    });
    lines.push(TextLine::new(
        MARGIN, PAGE_H - 30.0, 18.0, "F2",
        (Color::WHITE.r, Color::WHITE.g, Color::WHITE.b),
        "Technical Approach",
    ));
    lines.push(TextLine::new(
        MARGIN, PAGE_H - 52.0, 10.0, "F1",
        (0.85, 0.92, 1.0),
        &ctx.company.name,
    ));

    let mut y = PAGE_H - 110.0;
    let body_width = PAGE_W - 2.0 * MARGIN;

    let sections = [
        ("1. Understanding of Requirements",
         "We have thoroughly reviewed all solicitation documents and possess a clear \
          understanding of the performance requirements, deliverables, and quality standards. \
          Our approach is tailored to meet the specific needs of the Government while \
          ensuring full compliance with all terms and conditions."),
        ("2. Management Approach",
         "Our management structure ensures direct accountability from contract award \
          through final performance. A dedicated point of contact will be available \
          during all business hours to respond to Government inquiries, handle scheduling \
          coordination, and address any performance issues promptly."),
        ("3. Staffing Plan",
         "We will assign fully trained, vetted personnel who meet or exceed all required \
          qualifications. All staff will hold appropriate certifications and background \
          clearances as required by the solicitation. We maintain a qualified bench of \
          trained alternates to ensure continuity of performance."),
        ("4. Quality Control",
         "Our Quality Control Plan (QCP) includes daily site inspections, corrective action \
          procedures, and documented tracking of all performance metrics. We will provide \
          monthly QC reports and maintain open communication with the Contracting Officer's \
          Representative (COR) throughout the period of performance."),
        ("5. Transition Plan",
         "We are prepared to assume full performance responsibilities on the contract start \
          date with no gap in service. Our transition plan includes early staffing, \
          equipment pre-positioning, and a kick-off meeting with Government personnel within \
          five business days of award."),
    ];

    for (heading, body) in &sections {
        if y < 120.0 {
            let stream = templates::build_stream(&rects, &hrules, &lines);
            let stream_id = doc.add_object(Object::Stream(stream));
            let page_id = templates::add_page(&mut doc, stream_id);
            pages_kids.push(Object::Reference(page_id));
            rects = Vec::new();
            hrules = Vec::new();
            lines = Vec::new();
            y = PAGE_H - MARGIN;
        }

        // Section heading
        push_text(&mut lines, MARGIN, y, 12.0, "F2", Color::HOAGS_BLUE, heading);
        y -= 6.0;
        hrules.push(HRule {
            x: MARGIN,
            y,
            w: body_width,
            width_pts: 0.5,
            color: (Color::HOAGS_BLUE.r, Color::HOAGS_BLUE.g, Color::HOAGS_BLUE.b),
        });
        y -= 14.0;
        y = wrap_text(&mut lines, MARGIN, y, 11.0, body_width, body, 16.0);
        y -= 18.0;
    }

    footer_line(&mut hrules, &mut lines, ctx, 1);

    let stream = templates::build_stream(&rects, &hrules, &lines);
    let stream_id = doc.add_object(Object::Stream(stream));
    let page_id = templates::add_page(&mut doc, stream_id);
    pages_kids.push(Object::Reference(page_id));

    attach_pages(&mut doc, pages_kids)
}

// ─── Full package generator ───────────────────────────────────────────────────

/// Write all four documents to `output_dir` and return the list of written file paths.
pub fn generate_full_package(
    ctx: &ProposalContext,
    output_dir: &Path,
) -> Result<Vec<std::path::PathBuf>, Box<dyn std::error::Error>> {
    std::fs::create_dir_all(output_dir)?;
    let mut written = Vec::new();

    let mut cl_doc = build_cover_letter(ctx, None);
    let cl_path = output_dir.join("cover_letter.pdf");
    cl_doc.save(&cl_path)?;
    written.push(cl_path);

    let mut ta_doc = build_technical_approach(ctx);
    let ta_path = output_dir.join("technical_approach.pdf");
    ta_doc.save(&ta_path)?;
    written.push(ta_path);

    let mut pp_doc = build_past_performance(ctx);
    let pp_path = output_dir.join("past_performance.pdf");
    pp_doc.save(&pp_path)?;
    written.push(pp_path);

    let mut ps_doc = build_price_schedule(ctx);
    let ps_path = output_dir.join("price_schedule.pdf");
    ps_doc.save(&ps_path)?;
    written.push(ps_path);

    Ok(written)
}

// ─── Internal helpers ─────────────────────────────────────────────────────────

fn push_text(
    lines: &mut Vec<TextLine>,
    x: f64, y: f64, size: f64,
    font: &'static str,
    color: Color,
    text: &str,
) {
    lines.push(TextLine::new(x, y, size, font, (color.r, color.g, color.b), text));
}

/// Crude word-wrap: splits text on spaces, emits one TextLine per row.
/// Returns the y position after the last emitted line.
fn wrap_text(
    lines: &mut Vec<TextLine>,
    x: f64,
    start_y: f64,
    size: f64,
    max_width: f64,
    text: &str,
    line_height: f64,
) -> f64 {
    // Rough character width estimate: size * 0.55
    let chars_per_line = (max_width / (size * 0.55)).max(1.0) as usize;
    let mut current_line = String::new();
    let mut y = start_y;

    for word in text.split_whitespace() {
        let candidate = if current_line.is_empty() {
            word.to_string()
        } else {
            format!("{} {}", current_line, word)
        };

        if candidate.len() <= chars_per_line {
            current_line = candidate;
        } else {
            if !current_line.is_empty() {
                lines.push(TextLine::new(x, y, size, "F1", (0.0, 0.0, 0.0), &current_line));
                y -= line_height;
            }
            current_line = word.to_string();
        }
    }
    if !current_line.is_empty() {
        lines.push(TextLine::new(x, y, size, "F1", (0.0, 0.0, 0.0), &current_line));
        y -= line_height;
    }
    y
}

fn footer_line(hrules: &mut Vec<HRule>, lines: &mut Vec<TextLine>, ctx: &ProposalContext, page: u32) {
    hrules.push(HRule {
        x: MARGIN,
        y: 50.0,
        w: PAGE_W - 2.0 * MARGIN,
        width_pts: 0.5,
        color: (0.80, 0.80, 0.80),
    });
    let footer = format!(
        "{} | CAGE {} | UEI {} | Page {}",
        ctx.company.name, ctx.company.cage, ctx.company.uei, page
    );
    lines.push(TextLine::new(MARGIN, 36.0, 8.0, "F1",
        (Color::DARK_GRAY.r, Color::DARK_GRAY.g, Color::DARK_GRAY.b), &footer));
}

/// Wire up Pages + Catalog on an owned Document and return it.
fn attach_pages(mut doc: Document, kids: Vec<Object>) -> Document {
    let count = kids.len() as i64;
    let mut pages_dict = lopdf::Dictionary::new();
    pages_dict.set("Type", Object::Name(b"Pages".to_vec()));
    pages_dict.set("Kids", Object::Array(kids));
    pages_dict.set("Count", Object::Integer(count));
    let pages_id = doc.add_object(Object::Dictionary(pages_dict));

    // Point every child page back to Pages
    let page_ids: Vec<ObjectId> = doc.objects.keys().copied().collect();
    for id in page_ids {
        if let Ok(obj) = doc.get_object_mut(id) {
            if let Ok(dict) = obj.as_dict_mut() {
                if dict.get(b"Type").ok()
                    .and_then(|o| o.as_name().ok())
                    == Some(b"Page")
                {
                    dict.set("Parent", Object::Reference(pages_id));
                }
            }
        }
    }

    let mut catalog = lopdf::Dictionary::new();
    catalog.set("Type", Object::Name(b"Catalog".to_vec()));
    catalog.set("Pages", Object::Reference(pages_id));
    let catalog_id = doc.add_object(Object::Dictionary(catalog));
    doc.trailer.set("Root", Object::Reference(catalog_id));
    doc.trailer.set("Size", Object::Integer(doc.objects.len() as i64 + 1));

    doc
}

// ─── tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{CompanyInfo, PricingInfo, SignerInfo};
    use tempfile::TempDir;

    fn sample_ctx() -> ProposalContext {
        ProposalContext {
            company: CompanyInfo {
                name: "Hoags Inc.".into(),
                cage: "15XV5".into(),
                uei: "DUHWVUXFNPV5".into(),
                address: "123 Forest Rd".into(),
                phone: "(458) 239-3215".into(),
                email: "collin@hoagsinc.com".into(),
            },
            signer: SignerInfo {
                name: "Collin Hoag".into(),
                title: "President".into(),
                phone: "(458) 239-3215".into(),
                email: "collin@hoagsinc.com".into(),
            },
            past_performance: vec![
                crate::context::PastPerformance {
                    contract: "12444626P0025".into(),
                    title: "Ottawa NF Janitorial".into(),
                    value: 42000.0,
                    period: "2026-2027".into(),
                    agency: "USDA Forest Service".into(),
                    description: "Janitorial across 3 ranger districts.".into(),
                    poc_name: "Jane Smith".into(),
                    poc_phone: "(503) 555-0100".into(),
                },
            ],
            pricing: PricingInfo {
                labor_rate: 28.0,
                overhead: 0.10,
                profit: 0.08,
                clins: vec![
                    Clin {
                        number: "0001".into(),
                        description: "Base Year Janitorial Services".into(),
                        quantity: 12.0,
                        unit: "Month".into(),
                        unit_price: 3500.0,
                    },
                ],
            },
            solicitation: crate::context::SolicitationMeta {
                number: "12444626P0025".into(),
                title: "Ottawa National Forest Janitorial".into(),
                due_date: "2026-05-01".into(),
                co_name: "Ashley Stokes".into(),
                co_email: "ashley.stokes@fs.usda.gov".into(),
                agency: "USDA Forest Service".into(),
                issue_date: "2026-04-01".into(),
            },
        }
    }

    #[test]
    fn test_build_cover_letter_produces_pdf() {
        let ctx = sample_ctx();
        let mut doc = build_cover_letter(&ctx, None);
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("cover.pdf");
        doc.save(&path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn test_cover_letter_co_override() {
        let ctx = sample_ctx();
        let mut doc = build_cover_letter(&ctx, Some("John Doe"));
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("cover_override.pdf");
        doc.save(&path).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn test_build_past_performance_produces_pdf() {
        let ctx = sample_ctx();
        let mut doc = build_past_performance(&ctx);
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("pp.pdf");
        doc.save(&path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn test_build_price_schedule_produces_pdf() {
        let ctx = sample_ctx();
        let mut doc = build_price_schedule(&ctx);
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("price.pdf");
        doc.save(&path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn test_build_technical_approach_produces_pdf() {
        let ctx = sample_ctx();
        let mut doc = build_technical_approach(&ctx);
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("ta.pdf");
        doc.save(&path).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn test_generate_full_package_creates_four_files() {
        let ctx = sample_ctx();
        let tmp = TempDir::new().unwrap();
        let files = generate_full_package(&ctx, tmp.path()).unwrap();
        assert_eq!(files.len(), 4);
        for f in &files {
            assert!(f.exists(), "Expected file to exist: {:?}", f);
        }
    }

    #[test]
    fn test_price_schedule_no_clins_uses_default() {
        let mut ctx = sample_ctx();
        ctx.pricing.clins = vec![];
        let mut doc = build_price_schedule(&ctx);
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("price_default.pdf");
        doc.save(&path).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn test_looks_like_sol_number_positive() {
        assert!(looks_like_sol_number("12444626P0025"));
        assert!(looks_like_sol_number("SOL-2026-001"));
        assert!(looks_like_sol_number("W912DR24R0012"));
    }

    #[test]
    fn test_looks_like_sol_number_negative() {
        assert!(!looks_like_sol_number("hello"));        // too short, no digit/alpha combo
        assert!(!looks_like_sol_number("12345678901234567890123456")); // too long
    }

    #[test]
    fn test_grand_total_calculation() {
        let clins = vec![
            Clin { number: "0001".into(), description: "A".into(), quantity: 12.0, unit: "Mo".into(), unit_price: 3500.0 },
            Clin { number: "0002".into(), description: "B".into(), quantity: 1.0, unit: "EA".into(), unit_price: 500.0 },
        ];
        let total: f64 = clins.iter().map(|c| c.total()).sum();
        assert!((total - 42500.0).abs() < 0.01);
    }
}
