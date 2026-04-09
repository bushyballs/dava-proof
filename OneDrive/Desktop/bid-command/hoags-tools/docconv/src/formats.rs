/// Format detection and routing for docconv.

use std::path::Path;

/// Recognized document formats.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Format {
    Pdf,
    Csv,
    Json,
    Text,
    Unknown(String),
}

impl std::fmt::Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Format::Pdf => write!(f, "PDF"),
            Format::Csv => write!(f, "CSV"),
            Format::Json => write!(f, "JSON"),
            Format::Text => write!(f, "Text"),
            Format::Unknown(ext) => write!(f, "Unknown({})", ext),
        }
    }
}

impl Format {
    /// Parse a format name string (as supplied via `--to`).
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().trim_start_matches('.') {
            "pdf" => Format::Pdf,
            "csv" => Format::Csv,
            "json" => Format::Json,
            "txt" | "text" => Format::Text,
            other => Format::Unknown(other.to_string()),
        }
    }

    /// Canonical file extension for this format.
    pub fn extension(&self) -> &str {
        match self {
            Format::Pdf => "pdf",
            Format::Csv => "csv",
            Format::Json => "json",
            Format::Text => "txt",
            Format::Unknown(ext) => ext.as_str(),
        }
    }
}

/// Detect the format of a file by its path/extension.
pub fn detect_format(path: &Path) -> Format {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "pdf" => Format::Pdf,
        "csv" => Format::Csv,
        "json" => Format::Json,
        "txt" | "text" => Format::Text,
        other => Format::Unknown(other.to_string()),
    }
}

/// Return true if a conversion from `src` to `dst` is supported.
pub fn is_supported(src: &Format, dst: &Format) -> bool {
    matches!(
        (src, dst),
        (Format::Pdf, Format::Text)
            | (Format::Pdf, Format::Json)
            | (Format::Csv, Format::Json)
            | (Format::Json, Format::Csv)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn detect_pdf() {
        assert_eq!(detect_format(&PathBuf::from("foo.pdf")), Format::Pdf);
    }

    #[test]
    fn detect_csv() {
        assert_eq!(detect_format(&PathBuf::from("data.csv")), Format::Csv);
    }

    #[test]
    fn detect_json() {
        assert_eq!(detect_format(&PathBuf::from("out.json")), Format::Json);
    }

    #[test]
    fn detect_txt() {
        assert_eq!(detect_format(&PathBuf::from("notes.txt")), Format::Text);
    }

    #[test]
    fn from_str_case_insensitive() {
        assert_eq!(Format::from_str("PDF"), Format::Pdf);
        assert_eq!(Format::from_str(".csv"), Format::Csv);
        assert_eq!(Format::from_str("JSON"), Format::Json);
        assert_eq!(Format::from_str("txt"), Format::Text);
        assert_eq!(Format::from_str("text"), Format::Text);
    }

    #[test]
    fn supported_paths() {
        assert!(is_supported(&Format::Pdf, &Format::Text));
        assert!(is_supported(&Format::Pdf, &Format::Json));
        assert!(is_supported(&Format::Csv, &Format::Json));
        assert!(is_supported(&Format::Json, &Format::Csv));
    }

    #[test]
    fn unsupported_path() {
        assert!(!is_supported(&Format::Text, &Format::Pdf));
        assert!(!is_supported(&Format::Csv, &Format::Pdf));
    }

    #[test]
    fn extension_round_trip() {
        assert_eq!(Format::Pdf.extension(), "pdf");
        assert_eq!(Format::Csv.extension(), "csv");
        assert_eq!(Format::Json.extension(), "json");
        assert_eq!(Format::Text.extension(), "txt");
    }
}
