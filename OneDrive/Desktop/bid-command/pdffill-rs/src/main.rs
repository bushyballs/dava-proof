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
