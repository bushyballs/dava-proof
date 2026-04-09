use lopdf::{Document, Object, Stream, Dictionary};
use lopdf::content::{Content, Operation};

macro_rules! real {
    ($v:expr) => { Object::Real($v as f32) };
}

/// Page dimensions — US Letter in PDF units (72 pts/inch).
pub const PAGE_W: f64 = 612.0;
pub const PAGE_H: f64 = 792.0;
pub const MARGIN: f64 = 72.0; // 1 inch margins

/// Hex colour helpers (all RGB, values 0-1).
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

impl Color {
    pub const HOAGS_BLUE: Color = Color { r: 0.071, g: 0.290, b: 0.588 };
    pub const BLACK: Color = Color { r: 0.0, g: 0.0, b: 0.0 };
    pub const LIGHT_GRAY: Color = Color { r: 0.90, g: 0.90, b: 0.90 };
    pub const WHITE: Color = Color { r: 1.0, g: 1.0, b: 1.0 };
    pub const DARK_GRAY: Color = Color { r: 0.3, g: 0.3, b: 0.3 };
}

/// A single line of text ready to be emitted into a content stream.
#[derive(Debug, Clone)]
pub struct TextLine {
    pub x: f64,
    pub y: f64,
    pub size: f64,
    pub font: &'static str,
    pub color: (f64, f64, f64),
    pub text: String,
}

impl TextLine {
    pub fn new(x: f64, y: f64, size: f64, font: &'static str, color: (f64, f64, f64), text: impl Into<String>) -> Self {
        Self { x, y, size, font, color, text: text.into() }
    }

    fn ops(&self) -> Vec<Operation> {
        vec![
            Operation::new("rg", vec![real!(self.color.0), real!(self.color.1), real!(self.color.2)]),
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec![Object::Name(self.font.as_bytes().to_vec()), real!(self.size)]),
            Operation::new("Td", vec![real!(self.x), real!(self.y)]),
            Operation::new("Tj", vec![Object::string_literal(safe_pdf_string(&self.text))]),
            Operation::new("ET", vec![]),
        ]
    }
}

/// A filled rectangle (for header bars, table rows, etc.)
#[derive(Debug, Clone)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub fill: (f64, f64, f64),
}

impl Rect {
    fn ops(&self) -> Vec<Operation> {
        vec![
            Operation::new("rg", vec![real!(self.fill.0), real!(self.fill.1), real!(self.fill.2)]),
            Operation::new("re", vec![real!(self.x), real!(self.y), real!(self.w), real!(self.h)]),
            Operation::new("f", vec![]),
        ]
    }
}

/// A horizontal rule line.
#[derive(Debug, Clone)]
pub struct HRule {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub width_pts: f64,
    pub color: (f64, f64, f64),
}

impl HRule {
    fn ops(&self) -> Vec<Operation> {
        vec![
            Operation::new("RG", vec![real!(self.color.0), real!(self.color.1), real!(self.color.2)]),
            Operation::new("w", vec![real!(self.width_pts)]),
            Operation::new("m", vec![real!(self.x), real!(self.y)]),
            Operation::new("l", vec![real!(self.x + self.w), real!(self.y)]),
            Operation::new("S", vec![]),
        ]
    }
}

/// Collect all ops into a single Content stream.
pub fn build_stream(rects: &[Rect], hrules: &[HRule], lines: &[TextLine]) -> Stream {
    let mut ops: Vec<Operation> = Vec::new();
    for r in rects { ops.extend(r.ops()); }
    for h in hrules { ops.extend(h.ops()); }
    for l in lines { ops.extend(l.ops()); }
    let content = Content { operations: ops };
    let encoded = content.encode().expect("content encode");
    Stream::new(Dictionary::new(), encoded)
}

/// Register standard PDF fonts (Helvetica family) on a Resources dictionary.
pub fn standard_fonts() -> Dictionary {
    let mut fonts = Dictionary::new();

    let mut helv = Dictionary::new();
    helv.set("Type", Object::Name(b"Font".to_vec()));
    helv.set("Subtype", Object::Name(b"Type1".to_vec()));
    helv.set("BaseFont", Object::Name(b"Helvetica".to_vec()));
    fonts.set("F1", Object::Dictionary(helv));

    let mut helv_b = Dictionary::new();
    helv_b.set("Type", Object::Name(b"Font".to_vec()));
    helv_b.set("Subtype", Object::Name(b"Type1".to_vec()));
    helv_b.set("BaseFont", Object::Name(b"Helvetica-Bold".to_vec()));
    fonts.set("F2", Object::Dictionary(helv_b));

    let mut helv_o = Dictionary::new();
    helv_o.set("Type", Object::Name(b"Font".to_vec()));
    helv_o.set("Subtype", Object::Name(b"Type1".to_vec()));
    helv_o.set("BaseFont", Object::Name(b"Helvetica-Oblique".to_vec()));
    fonts.set("F3", Object::Dictionary(helv_o));

    fonts
}

/// Build a blank US Letter page and add it to `doc`, returning the page object ID.
pub fn add_page(doc: &mut Document, stream_id: lopdf::ObjectId) -> lopdf::ObjectId {
    let mut resources = Dictionary::new();
    resources.set("Font", Object::Dictionary(standard_fonts()));

    let mut page = Dictionary::new();
    page.set("Type", Object::Name(b"Page".to_vec()));
    page.set("MediaBox", Object::Array(vec![
        Object::Integer(0), Object::Integer(0),
        Object::Integer(PAGE_W as i64), Object::Integer(PAGE_H as i64),
    ]));
    page.set("Resources", Object::Dictionary(resources));
    page.set("Contents", Object::Reference(stream_id));

    doc.add_object(Object::Dictionary(page))
}

/// Sanitize text for PDF literal strings: escape parentheses and backslash,
/// strip non-ASCII to keep byte sequences valid.
pub fn safe_pdf_string(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii())
        .map(|c| match c {
            '(' => "\\(".to_string(),
            ')' => "\\)".to_string(),
            '\\' => "\\\\".to_string(),
            other => other.to_string(),
        })
        .collect()
}

// ─── tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_pdf_string_parens() {
        let s = safe_pdf_string("Hello (world)");
        assert_eq!(s, "Hello \\(world\\)");
    }

    #[test]
    fn test_safe_pdf_string_backslash() {
        let s = safe_pdf_string("C:\\Users\\foo");
        assert_eq!(s, "C:\\\\Users\\\\foo");
    }

    #[test]
    fn test_safe_pdf_string_strips_non_ascii() {
        let s = safe_pdf_string("caf\u{00E9}");
        assert_eq!(s, "caf");
    }

    #[test]
    fn test_build_stream_returns_stream() {
        let rects = vec![Rect {
            x: 0.0, y: 700.0, w: 612.0, h: 40.0,
            fill: (0.071, 0.290, 0.588),
        }];
        let lines = vec![TextLine::new(72.0, 715.0, 14.0, "F2",
            (1.0, 1.0, 1.0), "Test Header")];
        let stream = build_stream(&rects, &[], &lines);
        assert!(!stream.content.is_empty());
    }

    #[test]
    fn test_text_line_ops_count() {
        let tl = TextLine::new(72.0, 600.0, 12.0, "F1", (0.0, 0.0, 0.0), "Hello");
        let ops = tl.ops();
        // rg + BT + Tf + Td + Tj + ET = 6
        assert_eq!(ops.len(), 6);
    }

    #[test]
    fn test_rect_ops_count() {
        let r = Rect { x: 0.0, y: 0.0, w: 100.0, h: 20.0, fill: (0.9, 0.9, 0.9) };
        let ops = r.ops();
        // rg + re + f = 3
        assert_eq!(ops.len(), 3);
    }

    #[test]
    fn test_clin_total_calc() {
        // Verify multiplication used by price schedule: qty * unit_price
        let qty = 12.0_f64;
        let up = 3500.0_f64;
        assert!((qty * up - 42000.0).abs() < 0.01);
    }

    #[test]
    fn test_color_constants_in_range() {
        let c = Color::HOAGS_BLUE;
        assert!(c.r >= 0.0 && c.r <= 1.0);
        assert!(c.g >= 0.0 && c.g <= 1.0);
        assert!(c.b >= 0.0 && c.b <= 1.0);
    }
}
