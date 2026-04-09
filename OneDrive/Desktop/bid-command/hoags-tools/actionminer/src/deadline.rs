/// Deadline parsing: convert human-readable date strings into `NaiveDate`.
///
/// Supported formats:
/// - ISO dates: "2026-04-15", "April 15", "Apr 15", "15 April 2026", "04/15/2026"
/// - Relative: "next Monday", "next Friday", "end of week", "end of month",
///   "by end of week", "by end of month"
/// - Month day only: "April 15" (assumes current or next occurrence)
use chrono::{Datelike, Duration, Local, NaiveDate, Weekday};

/// Try to parse a deadline string into a `NaiveDate`.
/// Returns `None` if the string cannot be interpreted as a date.
pub fn parse_deadline(s: &str) -> Option<NaiveDate> {
    let s = s.trim();
    let today = Local::now().date_naive();

    // --- ISO 8601: YYYY-MM-DD ---
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Some(d);
    }

    // --- US format: MM/DD/YYYY ---
    if let Ok(d) = NaiveDate::parse_from_str(s, "%m/%d/%Y") {
        return Some(d);
    }

    // --- US format: MM/DD/YY ---
    if let Ok(d) = NaiveDate::parse_from_str(s, "%m/%d/%y") {
        return Some(d);
    }

    // --- "Month DD, YYYY" / "Month DD YYYY" ---
    if let Some(d) = try_parse_month_day_year(s) {
        return Some(d);
    }

    // --- "Month DD" (no year — use current or next year) ---
    if let Some(d) = try_parse_month_day(s, today) {
        return Some(d);
    }

    // --- "DD Month YYYY" ---
    if let Ok(d) = NaiveDate::parse_from_str(s, "%d %B %Y") {
        return Some(d);
    }
    if let Ok(d) = NaiveDate::parse_from_str(s, "%d %b %Y") {
        return Some(d);
    }

    // --- Relative phrases ---
    let lower = s.to_ascii_lowercase();
    let lower = lower.trim_start_matches("by").trim();

    if lower.contains("end of week") {
        // End of the current ISO week = Sunday
        // num_days_from_monday: Mon=0, Tue=1, Wed=2, Thu=3, Fri=4, Sat=5, Sun=6
        // Days until Sunday: (6 - weekday) unless already Sunday (0 days to go).
        let dow = today.weekday().num_days_from_monday(); // 0..6
        let days_to_sunday = if dow == 6 { 0u32 } else { 6 - dow };
        return Some(today + Duration::days(days_to_sunday as i64));
    }

    if lower.contains("end of month") {
        return Some(last_day_of_month(today.year(), today.month()));
    }

    if let Some(day) = parse_next_weekday(lower) {
        // "next <weekday>" = the upcoming instance of that weekday (at least 1 day ahead)
        let mut candidate = today + Duration::days(1);
        while candidate.weekday() != day {
            candidate += Duration::days(1);
        }
        // If "next X" is today's weekday, skip to the one 7 days away
        if today.weekday() == day {
            candidate = today + Duration::days(7);
        }
        return Some(candidate);
    }

    // --- Plain weekday name (this week's upcoming occurrence) ---
    if let Some(day) = weekday_from_str(lower) {
        let mut candidate = today + Duration::days(1);
        for _ in 0..7 {
            if candidate.weekday() == day {
                return Some(candidate);
            }
            candidate += Duration::days(1);
        }
    }

    None
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn try_parse_month_day_year(s: &str) -> Option<NaiveDate> {
    // "April 15, 2026" or "Apr 15 2026" or "April 15 2026"
    let s = s.replace(',', "");
    NaiveDate::parse_from_str(s.trim(), "%B %d %Y")
        .or_else(|_| NaiveDate::parse_from_str(s.trim(), "%b %d %Y"))
        .ok()
}

fn try_parse_month_day(s: &str, today: NaiveDate) -> Option<NaiveDate> {
    // "April 15" or "Apr 15"
    let attempt = |fmt: &str| -> Option<NaiveDate> {
        let with_year = format!("{} {}", s, today.year());
        NaiveDate::parse_from_str(&with_year, fmt)
            .ok()
            .map(|d| {
                // If the date already passed this year, use next year
                if d < today {
                    NaiveDate::from_ymd_opt(today.year() + 1, d.month(), d.day()).unwrap_or(d)
                } else {
                    d
                }
            })
    };
    attempt("%B %d %Y").or_else(|| attempt("%b %d %Y"))
}

fn parse_next_weekday(s: &str) -> Option<Weekday> {
    let s = s.trim_start_matches("next").trim();
    weekday_from_str(s)
}

fn weekday_from_str(s: &str) -> Option<Weekday> {
    match s.trim().to_ascii_lowercase().as_str() {
        "monday" | "mon" => Some(Weekday::Mon),
        "tuesday" | "tue" | "tues" => Some(Weekday::Tue),
        "wednesday" | "wed" => Some(Weekday::Wed),
        "thursday" | "thu" | "thurs" => Some(Weekday::Thu),
        "friday" | "fri" => Some(Weekday::Fri),
        "saturday" | "sat" => Some(Weekday::Sat),
        "sunday" | "sun" => Some(Weekday::Sun),
        _ => None,
    }
}

fn last_day_of_month(year: i32, month: u32) -> NaiveDate {
    let next_month = if month == 12 { 1 } else { month + 1 };
    let next_year = if month == 12 { year + 1 } else { year };
    NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .unwrap()
        .pred_opt()
        .unwrap()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iso_date() {
        let d = parse_deadline("2026-04-15").unwrap();
        assert_eq!(d, NaiveDate::from_ymd_opt(2026, 4, 15).unwrap());
    }

    #[test]
    fn test_us_slash_date() {
        let d = parse_deadline("04/15/2026").unwrap();
        assert_eq!(d, NaiveDate::from_ymd_opt(2026, 4, 15).unwrap());
    }

    #[test]
    fn test_month_day_year() {
        let d = parse_deadline("April 15, 2026").unwrap();
        assert_eq!(d, NaiveDate::from_ymd_opt(2026, 4, 15).unwrap());
    }

    #[test]
    fn test_month_day_year_abbrev() {
        let d = parse_deadline("Apr 15 2026").unwrap();
        assert_eq!(d, NaiveDate::from_ymd_opt(2026, 4, 15).unwrap());
    }

    #[test]
    fn test_end_of_week() {
        // Just verify it returns a Sunday within 7 days
        let d = parse_deadline("end of week").unwrap();
        assert_eq!(d.weekday(), Weekday::Sun);
    }

    #[test]
    fn test_end_of_month() {
        let d = parse_deadline("end of month").unwrap();
        let today = Local::now().date_naive();
        assert_eq!(d.month(), today.month());
        // Should be the last day
        assert_eq!(d, last_day_of_month(today.year(), today.month()));
    }

    #[test]
    fn test_by_end_of_week() {
        let d = parse_deadline("by end of week").unwrap();
        assert_eq!(d.weekday(), Weekday::Sun);
    }

    #[test]
    fn test_next_monday() {
        let d = parse_deadline("next Monday").unwrap();
        assert_eq!(d.weekday(), Weekday::Mon);
        assert!(d > Local::now().date_naive());
    }

    #[test]
    fn test_next_friday() {
        let d = parse_deadline("next Friday").unwrap();
        assert_eq!(d.weekday(), Weekday::Fri);
    }

    #[test]
    fn test_unknown_returns_none() {
        assert!(parse_deadline("sometime soon").is_none());
        assert!(parse_deadline("ASAP").is_none());
    }

    #[test]
    fn test_march_15() {
        // "March 15" — should resolve to a date
        let d = parse_deadline("March 15");
        assert!(d.is_some());
        let d = d.unwrap();
        assert_eq!(d.month(), 3);
        assert_eq!(d.day(), 15);
    }
}
