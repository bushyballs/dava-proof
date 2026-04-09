fn main() {
    let path = std::env::args().nth(1).expect("Usage: dump_text <pdf>");
    let doc = lopdf::Document::load(&path).unwrap();
    for (page_num, _) in doc.get_pages() {
        let text = doc.extract_text(&[page_num]).unwrap_or_default();
        println!("=== PAGE {} ===", page_num);
        for line in text.lines() {
            if !line.trim().is_empty() {
                println!("  {:?}", line.trim());
            }
        }
    }
}
