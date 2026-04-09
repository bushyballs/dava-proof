/// Handles email composition: console display and file persistence.
///
/// Output is ALWAYS plain-text only.  This module NEVER sends email.
use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::{Path, PathBuf};

use crate::templates::EmailDraft;

/// Compose the final plain-text representation of a draft.
pub fn compose(draft: &EmailDraft) -> String {
    let mut out = String::new();

    writeln!(out, "TO:      {}", draft.to).unwrap();
    writeln!(out, "SUBJECT: {}", draft.subject).unwrap();
    writeln!(out, "{}", "-".repeat(72)).unwrap();
    writeln!(out).unwrap();
    out.push_str(&draft.body);
    out.push('\n');
    out
}

/// Save draft to `<output_dir>/<sol_or_contract>_<email_type>.txt`.
///
/// Returns the path of the written file.
/// This function NEVER transmits the email; it writes a local file only.
pub fn save_draft(
    draft: &EmailDraft,
    identifier: &str,
    email_type: &str,
    output_dir: &Path,
) -> std::io::Result<PathBuf> {
    let filename = format!("{}_{}.txt", sanitize(identifier), sanitize(email_type));
    let path = output_dir.join(filename);
    fs::write(&path, compose(draft))?;
    Ok(path)
}

/// Strip characters that are unsafe in file names.
fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | ' ' => '_',
            c => c,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::templates::EmailDraft;
    use tempfile::tempdir;

    fn sample_draft() -> EmailDraft {
        EmailDraft {
            to: "co@agency.gov".to_string(),
            subject: "Quote Submission — Solicitation TEST001".to_string(),
            body: "Dear Jane,\n\nPlease find attached our quote.\n\nRespectfully,\nCollin".to_string(),
        }
    }

    #[test]
    fn compose_includes_to_and_subject() {
        let d = sample_draft();
        let text = compose(&d);
        assert!(text.contains("TO:      co@agency.gov"));
        assert!(text.contains("SUBJECT: Quote Submission"));
    }

    #[test]
    fn compose_includes_separator_line() {
        let text = compose(&sample_draft());
        assert!(text.contains("--------"));
    }

    #[test]
    fn compose_body_is_present() {
        let text = compose(&sample_draft());
        assert!(text.contains("Dear Jane"));
    }

    #[test]
    fn save_draft_writes_file() {
        let dir = tempdir().unwrap();
        let d = sample_draft();
        let path = save_draft(&d, "TEST001", "quote-submit", dir.path()).unwrap();
        assert!(path.exists());
        let contents = fs::read_to_string(&path).unwrap();
        assert!(contents.contains("co@agency.gov"));
    }

    #[test]
    fn save_draft_filename_format() {
        let dir = tempdir().unwrap();
        let d = sample_draft();
        let path = save_draft(&d, "W9127S26QA030", "quote-submit", dir.path()).unwrap();
        let name = path.file_name().unwrap().to_string_lossy();
        assert_eq!(name, "W9127S26QA030_quote-submit.txt");
    }

    #[test]
    fn sanitize_replaces_unsafe_chars() {
        assert_eq!(sanitize("a/b\\c:d"), "a_b_c_d");
        assert_eq!(sanitize("normal"), "normal");
    }
}
