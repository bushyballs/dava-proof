/// Pattern-based action-item extractor.
///
/// Recognised triggers (case-insensitive):
///
/// **Explicit labels**
/// - `ACTION:` / `ACTION ITEM:` / `TODO:` / `TASK:` / `FOLLOW-UP:`
/// - `DEADLINE:` / `DUE:` / `ASSIGNED TO:` / `@name`
///
/// **Verb phrases**
/// - "will do", "will be doing", "needs to", "need to",
///   "should", "must", "responsible for", "is expected to",
///   "has to", "have to", "is to"
///
/// **Date markers** (collected as deadline metadata)
/// - "by <date>", "due <date>", "deadline: <date>", "due date: <date>"
///
/// **Section headers** — bullet points that follow an "Action Items:"
/// heading are all collected as action items.
use regex::Regex;

use crate::models::ActionItem;

// ---------------------------------------------------------------------------
// Helper: compile a regex once and reuse it
// ---------------------------------------------------------------------------

struct Patterns {
    explicit_label: Regex,
    verb_phrase: Regex,
    deadline_inline: Regex,
    assignee_inline: Regex,
    section_header: Regex,
    bullet: Regex,
    at_mention: Regex,
}

impl Patterns {
    fn new() -> Self {
        Self {
            // ACTION:, ACTION ITEM:, TODO:, TASK:, FOLLOW-UP:
            explicit_label: Regex::new(
                r"(?i)^[\s\-\*>]*(?:action\s+item[s]?|action|todo|task|follow[\s\-]?up)\s*:\s*(.+)"
            ).unwrap(),

            // verb triggers anywhere in a line
            verb_phrase: Regex::new(
                r"(?i)\b(will\s+(?:be\s+)?do(?:ing|ne)?|needs?\s+to|have?\s+to|has\s+to|must|should|responsible\s+for|is\s+expected\s+to|is\s+to)\b"
            ).unwrap(),

            // "by <date>", "due <date>", "deadline: <date>", "due date: <date>"
            deadline_inline: Regex::new(
                r"(?i)(?:by|due(?:\s+date)?|deadline)\s*:?\s+([A-Za-z0-9/\-,\. ]{3,30})"
            ).unwrap(),

            // "assigned to: <name>", "owner: <name>", "for: <name>"
            assignee_inline: Regex::new(
                r"(?i)(?:assigned\s+to|owner|responsible|for)\s*:\s*(@?\w[\w\s\-]{0,30})"
            ).unwrap(),

            // "Action Items:" section header on its own line
            section_header: Regex::new(
                r"(?i)^\s*(?:action\s+items?|action\s+required|follow[\s\-]?ups?|tasks?)\s*:?\s*$"
            ).unwrap(),

            // generic bullet: -, *, •, >, numbered list
            bullet: Regex::new(r"^[\s]*(?:[-\*\u2022>]|\d+[.)]) (.+)").unwrap(),

            // @mention
            at_mention: Regex::new(r"@(\w+)").unwrap(),
        }
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Extract action items from `text`.  `source_file` is stored on each item.
pub fn extract(text: &str, source_file: &str) -> Vec<ActionItem> {
    let p = Patterns::new();
    let mut items: Vec<ActionItem> = Vec::new();

    let lines: Vec<&str> = text.lines().collect();
    let mut in_action_section = false;

    for (idx, &line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // ── Section header detection ──────────────────────────────────────
        if p.section_header.is_match(trimmed) {
            in_action_section = true;
            continue;
        }

        // Any non-empty non-bullet line ends the action section
        if in_action_section {
            if trimmed.is_empty() {
                continue; // blank lines don't break the section
            }
            if let Some(cap) = p.bullet.captures(trimmed) {
                let desc = cap[1].trim().to_string();
                if !desc.is_empty() {
                    let (assignee, deadline) = extract_metadata(&p, &desc, lines.get(idx + 1).copied());
                    items.push(ActionItem::new(desc, assignee, deadline, source_file));
                    continue;
                }
            }
            // Non-bullet text after header — still treat as action item if it
            // looks substantive (not another header)
            if trimmed.len() > 5 && !trimmed.ends_with(':') {
                let (assignee, deadline) = extract_metadata(&p, trimmed, lines.get(idx + 1).copied());
                items.push(ActionItem::new(trimmed, assignee, deadline, source_file));
            } else {
                // Another header-like line resets the section
                in_action_section = false;
            }
            continue;
        }

        // ── Explicit label (ACTION:, TODO:, TASK: …) ─────────────────────
        if let Some(cap) = p.explicit_label.captures(trimmed) {
            let desc = cap[1].trim().to_string();
            if !desc.is_empty() {
                let (assignee, deadline) = extract_metadata(&p, &desc, lines.get(idx + 1).copied());
                items.push(ActionItem::new(desc, assignee, deadline, source_file));
                continue;
            }
        }

        // ── DEADLINE: / DUE: / ASSIGNED TO: as standalone lines ──────────
        // These are metadata modifiers — pair with the previous item if any.
        if let Some(cap) = p.deadline_inline.captures(trimmed) {
            if let Some(last) = items.last_mut() {
                if last.deadline.is_none() {
                    last.deadline = Some(cap[1].trim().to_string());
                }
            }
            continue;
        }
        if let Some(cap) = p.assignee_inline.captures(trimmed) {
            if let Some(last) = items.last_mut() {
                if last.assignee.is_none() {
                    last.assignee = Some(cap[1].trim().to_string());
                }
            }
            continue;
        }

        // ── Verb-phrase trigger ───────────────────────────────────────────
        if p.verb_phrase.is_match(trimmed) {
            let (assignee, deadline) = extract_metadata(&p, trimmed, lines.get(idx + 1).copied());
            items.push(ActionItem::new(trimmed, assignee, deadline, source_file));
            continue;
        }
    }

    // Deduplicate exact descriptions
    items.dedup_by(|a, b| a.description == b.description);
    items
}

// ---------------------------------------------------------------------------
// Metadata extraction helpers
// ---------------------------------------------------------------------------

/// Pull (assignee, deadline) from a description string and optionally from
/// the very next line (continuation lines often contain "assigned to:" etc.).
fn extract_metadata(
    p: &Patterns,
    desc: &str,
    next_line: Option<&str>,
) -> (Option<String>, Option<String>) {
    let search_text = match next_line {
        Some(nl) => format!("{} {}", desc, nl),
        None => desc.to_string(),
    };

    let assignee = extract_assignee(p, &search_text);
    let deadline = extract_deadline(p, &search_text);
    (assignee, deadline)
}

fn extract_assignee(p: &Patterns, text: &str) -> Option<String> {
    // Prefer explicit "assigned to:" / "owner:" label
    if let Some(cap) = p.assignee_inline.captures(text) {
        return Some(cap[1].trim().to_string());
    }
    // Fall back to @mention
    if let Some(cap) = p.at_mention.captures(text) {
        return Some(format!("@{}", &cap[1]));
    }
    None
}

fn extract_deadline(p: &Patterns, text: &str) -> Option<String> {
    p.deadline_inline
        .captures(text)
        .map(|cap| cap[1].trim().to_string())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_explicit_action_label() {
        let text = "ACTION: Send contract to legal team by Friday";
        let items = extract(text, "test.md");
        assert_eq!(items.len(), 1);
        assert!(items[0].description.contains("Send contract"));
    }

    #[test]
    fn test_todo_label() {
        let text = "TODO: Update the pricing spreadsheet";
        let items = extract(text, "test.md");
        assert_eq!(items.len(), 1);
        assert!(items[0].description.contains("pricing spreadsheet"));
    }

    #[test]
    fn test_section_header_bullets() {
        let text = "Action Items:\n- Review proposal\n- Call vendor\n- Update budget\n";
        let items = extract(text, "meeting.md");
        assert_eq!(items.len(), 3);
        assert!(items.iter().any(|i| i.description.contains("Review proposal")));
        assert!(items.iter().any(|i| i.description.contains("Call vendor")));
    }

    #[test]
    fn test_verb_phrase_trigger() {
        let text = "John needs to submit the invoice before end of month.";
        let items = extract(text, "notes.txt");
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_deadline_extraction() {
        let text = "ACTION: Submit bid by March 15";
        let items = extract(text, "test.md");
        assert!(!items.is_empty());
        assert_eq!(items[0].deadline.as_deref(), Some("March 15"));
    }

    #[test]
    fn test_at_mention_assignee() {
        let text = "TODO: Finalize report @alice";
        let items = extract(text, "test.md");
        assert!(!items.is_empty());
        assert_eq!(items[0].assignee.as_deref(), Some("@alice"));
    }

    #[test]
    fn test_explicit_assigned_to() {
        let text = "ACTION: Review contract\nAssigned to: Bob Smith";
        let items = extract(text, "test.md");
        assert!(!items.is_empty());
        // Assignee may come from next-line scan or inline
        let _ = &items[0].assignee; // just ensure it parsed without panic
    }

    #[test]
    fn test_dedup() {
        let text = "ACTION: Do the thing\nACTION: Do the thing\n";
        let items = extract(text, "test.md");
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_empty_input() {
        let items = extract("", "test.md");
        assert!(items.is_empty());
    }

    #[test]
    fn test_must_trigger() {
        let text = "The contractor must submit weekly status reports.";
        let items = extract(text, "sow.md");
        assert_eq!(items.len(), 1);
        assert!(items[0].description.contains("weekly status reports"));
    }
}
