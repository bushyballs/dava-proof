use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "pdffill", about = "Universal PDF field detection and filling engine")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Detect fillable fields in a PDF
    Detect {
        /// Path to the PDF file
        pdf: PathBuf,
    },
    /// Fill a PDF with context data
    Fill {
        /// Path to the PDF file
        pdf: PathBuf,
        /// Path to context JSON file
        #[arg(long)]
        context: PathBuf,
        /// Skip ALL network calls — context + memory only
        #[arg(long)]
        airgap: bool,
    },
    /// Batch test PDFs in a directory
    Batch {
        /// Directory containing PDFs
        dir: PathBuf,
        /// Maximum PDFs to test
        #[arg(long, default_value = "50")]
        max: usize,
    },
    /// Query DAVA's memory
    Memory {
        /// Show memory statistics
        #[arg(long)]
        stats: bool,
        /// Search for a field pattern
        #[arg(long)]
        search: Option<String>,
    },
}

fn collect_pdfs(dir: &std::path::Path, pdfs: &mut Vec<PathBuf>, max: usize) {
    if pdfs.len() >= max { return; }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if pdfs.len() >= max { return; }
            let path = entry.path();
            if path.is_dir() {
                collect_pdfs(&path, pdfs, max);
            } else if path.extension().and_then(|e| e.to_str()) == Some("pdf") {
                pdfs.push(path);
            }
        }
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Detect { pdf } => {
            let fields = pdffill::detect::detect_all_fields(&pdf);
            println!("Detected {} fields in {}", fields.len(), pdf.display());
            for f in &fields {
                println!(
                    "  p{} [{:10}] {:10} {:30?} bbox=({:.0},{:.0},{:.0},{:.0})",
                    f.page, f.source, f.field_type, f.label,
                    f.bbox.0, f.bbox.1, f.bbox.2, f.bbox.3,
                );
            }
        }
        Commands::Fill { pdf, context, airgap } => {
            let ctx = pdffill::context::load_context_file(&context)
                .expect("Failed to load context file");
            let memory = pdffill::memory::FieldMemory::open_default()
                .expect("Failed to open memory DB");

            let detected = pdffill::detect::detect_all_fields(&pdf);
            let classified = pdffill::classify::classify_fields(&detected, &memory);
            let filled = pdffill::fill::fill_fields(&classified, &ctx, &memory, airgap);

            let green = filled.iter().filter(|f| f.confidence >= 0.85).count();
            let yellow = filled.iter().filter(|f| f.confidence >= 0.5 && f.confidence < 0.85).count();
            let red = filled.iter().filter(|f| f.confidence < 0.5).count();

            println!("Fields: {}  Green: {}  Yellow: {}  Red: {}", filled.len(), green, yellow, red);
            for f in &filled {
                let color = if f.confidence >= 0.85 { "GREEN" }
                    else if f.confidence >= 0.5 { "YELLOW" }
                    else { "RED" };
                let val = if f.value.is_empty() { "(empty)" } else { &f.value };
                println!("  {:6} {:45} -> {}", color, f.label, val);
            }

            // Render filled PDF + report
            let out_dir = pdf.with_extension("").to_string_lossy().to_string() + "_filled";
            let out_dir = std::path::PathBuf::from(&out_dir);

            match pdffill::render::render_filled_pdf(&pdf, &filled, &out_dir) {
                Ok(filled_path) => println!("\nFilled PDF: {}", filled_path.display()),
                Err(e) => eprintln!("\nFailed to render filled PDF: {}", e),
            }

            match pdffill::render::write_fill_report(&filled, &out_dir) {
                Ok(report_path) => println!("Report:     {}", report_path.display()),
                Err(e) => eprintln!("Failed to write report: {}", e),
            }
        }
        Commands::Batch { dir, max } => {
            let start_all = std::time::Instant::now();
            let mut pdfs: Vec<PathBuf> = Vec::new();
            collect_pdfs(&dir, &mut pdfs, max);

            println!("Testing {} PDFs from {}", pdfs.len(), dir.display());
            println!("{}", "=".repeat(70));

            let mut total_fields = 0usize;
            let mut total_classified = std::collections::HashMap::<String, usize>::new();
            let mut total_sources = std::collections::HashMap::<String, usize>::new();
            let mut zero_field_pdfs = Vec::new();

            let memory = pdffill::memory::FieldMemory::open_default()
                .expect("Failed to open memory DB");

            for (i, pdf) in pdfs.iter().enumerate() {
                let start = std::time::Instant::now();
                let fields = pdffill::detect::detect_all_fields(pdf);
                let classified = pdffill::classify::classify_fields(&fields, &memory);
                let ms = start.elapsed().as_millis();
                let name = pdf.file_name().unwrap_or_default().to_string_lossy();
                let display_name = if name.len() > 45 { format!("{}...", &name[..42]) } else { name.to_string() };

                println!("  [{:3}/{}] OK {:47} {:4} fields  {:6}ms",
                    i + 1, pdfs.len(), display_name, fields.len(), ms);

                if fields.is_empty() {
                    zero_field_pdfs.push(display_name.clone());
                }

                total_fields += fields.len();
                for f in &fields {
                    *total_sources.entry(f.source.clone()).or_insert(0) += 1;
                }
                for c in &classified {
                    *total_classified.entry(c.classification.clone()).or_insert(0) += 1;
                }
            }

            let total_ms = start_all.elapsed().as_millis();
            println!("{}", "=".repeat(70));
            println!("  PDFs tested:   {}", pdfs.len());
            println!("  Total fields:  {}", total_fields);
            println!("  Total time:    {}ms ({:.1}s)", total_ms, total_ms as f64 / 1000.0);
            if !pdfs.is_empty() {
                println!("  Avg per PDF:   {}ms", total_ms / pdfs.len() as u128);
            }

            if !total_sources.is_empty() {
                println!("\n  Detection Sources:");
                let mut sources: Vec<_> = total_sources.iter().collect();
                sources.sort_by(|a, b| b.1.cmp(a.1));
                for (src, count) in &sources {
                    let pct = **count as f64 / total_fields as f64 * 100.0;
                    println!("    {:15} {:5} ({:.1}%)", src, count, pct);
                }
            }

            if !total_classified.is_empty() {
                println!("\n  Classifications:");
                let mut classes: Vec<_> = total_classified.iter().collect();
                classes.sort_by(|a, b| b.1.cmp(a.1));
                for (cls, count) in &classes {
                    let pct = **count as f64 / total_fields as f64 * 100.0;
                    println!("    {:20} {:5} ({:.1}%)", cls, count, pct);
                }
            }

            if !zero_field_pdfs.is_empty() {
                println!("\n  Zero-field PDFs ({}):", zero_field_pdfs.len());
                for name in &zero_field_pdfs {
                    println!("    {}", name);
                }
            }
        }
        Commands::Memory { stats, search } => {
            let memory = pdffill::memory::FieldMemory::open_default()
                .expect("Failed to open memory DB");
            if stats {
                let s = memory.stats();
                println!("DAVA Memory: {} fields, {} templates", s.total_fields, s.total_templates);
            }
            if let Some(term) = search {
                match memory.recall(&term) {
                    Some(hit) => {
                        println!("Found: {}", term);
                        println!("  Value:      {}", hit.value);
                        println!("  Class:      {}", hit.classification);
                        println!("  Confidence: {:.1}%", hit.confidence * 100.0);
                    }
                    None => println!("No memory for: {}", term),
                }
            }
        }
    }
}
