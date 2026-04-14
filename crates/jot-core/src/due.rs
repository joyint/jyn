// Copyright (c) 2026 Joydev GmbH (joydev.com)
// SPDX-License-Identifier: MIT

//! Minimal due-date parsing and rendering.
//!
//! Accepts the common forms a personal task manager needs:
//!   - `today`, `tomorrow`
//!   - `YYYY-MM-DD` (ISO 8601 calendar date)
//!   - `MM-DD`      (current year implied)
//!   - `DD.MM`      (German short form, current year implied)
//!   - `DD.MM.YYYY` (German long form)
//!
//! Weekday names (`fri`, `next monday`) and relative offsets (`+3d`) are
//! deferred to JOT-0032-69; the parser returns a structured error for
//! anything it does not recognise so the CLI can surface a useful hint.
//!
//! Rendering produces short human-readable labels for a list view,
//! with a side-channel severity so the CLI can colorise consistently.

use chrono::{Datelike, Duration, NaiveDate};

#[derive(Debug, thiserror::Error)]
#[error("cannot parse due date '{input}': expected 'today', 'tomorrow', YYYY-MM-DD, MM-DD, DD.MM, or DD.MM.YYYY")]
pub struct ParseDueError {
    input: String,
}

/// Parse a `--due` argument against a reference 'today' date.
pub fn parse_due(input: &str, today: NaiveDate) -> Result<NaiveDate, ParseDueError> {
    let trimmed = input.trim();
    let lower = trimmed.to_lowercase();
    if lower == "today" {
        return Ok(today);
    }
    if lower == "tomorrow" {
        return Ok(today + Duration::days(1));
    }

    let year = today.year();
    // Try the accepted explicit formats first, then the current-year shortcuts.
    if let Ok(d) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        return Ok(d);
    }
    if let Ok(d) = NaiveDate::parse_from_str(trimmed, "%d.%m.%Y") {
        return Ok(d);
    }
    if let Ok(d) = NaiveDate::parse_from_str(&format!("{year}-{trimmed}"), "%Y-%m-%d") {
        return Ok(d);
    }
    if let Ok(d) = NaiveDate::parse_from_str(&format!("{trimmed}.{year}"), "%d.%m.%Y") {
        return Ok(d);
    }
    Err(ParseDueError {
        input: input.to_string(),
    })
}

/// Relative severity of a due date compared to 'today'.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DueSeverity {
    Overdue,
    Today,
    Soon,
    Later,
}

/// Label length mode. `Long` is the default reader-friendly form
/// (`today`, `tomorrow`, `overdue 2d`); `Short` compresses to the
/// terminal-friendly abbreviations matching joy-cli short mode
/// (`tod`, `tmw`, `-2d`). Weekday and month-day renderings are the
/// same in both modes because `%a` and `%b %-d` are already compact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelMode {
    Long,
    Short,
}

/// Render a due date as a label. Returns `(label, severity)` so the
/// CLI can apply colors consistently.
///
/// Dates beyond the next-six-days window always include the year so
/// entries from 2026 and 2027 are never mistaken for each other:
/// Long mode uses ISO `YYYY-MM-DD`; Short mode uses `MM-DD` when the
/// year matches today and `YYYY-MM-DD` otherwise.
pub fn render_due(due: NaiveDate, today: NaiveDate, mode: LabelMode) -> (String, DueSeverity) {
    let delta = (due - today).num_days();
    match (delta, mode) {
        (d, LabelMode::Long) if d < 0 => (format!("overdue {}d", d.abs()), DueSeverity::Overdue),
        (d, LabelMode::Short) if d < 0 => (format!("-{}d", d.abs()), DueSeverity::Overdue),
        (0, LabelMode::Long) => ("today".into(), DueSeverity::Today),
        (0, LabelMode::Short) => ("tod".into(), DueSeverity::Today),
        (1, LabelMode::Long) => ("tomorrow".into(), DueSeverity::Soon),
        (1, LabelMode::Short) => ("tmw".into(), DueSeverity::Soon),
        (d, _) if (2..=6).contains(&d) => (due.format("%a").to_string(), DueSeverity::Soon),
        (_, LabelMode::Long) => (due.format("%Y-%m-%d").to_string(), DueSeverity::Later),
        (_, LabelMode::Short) => {
            if due.year() == today.year() {
                (due.format("%m-%d").to_string(), DueSeverity::Later)
            } else {
                (due.format("%Y-%m-%d").to_string(), DueSeverity::Later)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn parses_today_tomorrow_iso() {
        let today = d(2026, 4, 14);
        assert_eq!(parse_due("today", today).unwrap(), today);
        assert_eq!(parse_due("TOMORROW", today).unwrap(), d(2026, 4, 15));
        assert_eq!(parse_due("2026-12-31", today).unwrap(), d(2026, 12, 31));
    }

    #[test]
    fn parses_current_year_shortcuts() {
        let today = d(2026, 4, 14);
        assert_eq!(parse_due("04-25", today).unwrap(), d(2026, 4, 25));
        assert_eq!(parse_due("25.04", today).unwrap(), d(2026, 4, 25));
        assert_eq!(parse_due("25.04.2027", today).unwrap(), d(2027, 4, 25));
    }

    #[test]
    fn rejects_unsupported_forms() {
        let today = d(2026, 4, 14);
        assert!(parse_due("friday", today).is_err());
        assert!(parse_due("+3d", today).is_err());
        assert!(parse_due("", today).is_err());
    }

    #[test]
    fn renders_relative_labels_long() {
        let today = d(2026, 4, 14); // Tuesday
        let long = LabelMode::Long;
        assert_eq!(render_due(today, today, long).0, "today");
        assert_eq!(render_due(d(2026, 4, 15), today, long).0, "tomorrow");
        assert_eq!(render_due(d(2026, 4, 17), today, long).0, "Fri");
        assert_eq!(render_due(d(2026, 4, 25), today, long).0, "2026-04-25");
        assert_eq!(render_due(d(2027, 4, 25), today, long).0, "2027-04-25");
        assert_eq!(render_due(d(2026, 4, 13), today, long).0, "overdue 1d");
    }

    #[test]
    fn renders_relative_labels_short() {
        let today = d(2026, 4, 14);
        let short = LabelMode::Short;
        assert_eq!(render_due(today, today, short).0, "tod");
        assert_eq!(render_due(d(2026, 4, 15), today, short).0, "tmw");
        assert_eq!(render_due(d(2026, 4, 17), today, short).0, "Fri");
        assert_eq!(render_due(d(2026, 4, 25), today, short).0, "04-25");
        assert_eq!(render_due(d(2027, 4, 25), today, short).0, "2027-04-25");
        assert_eq!(render_due(d(2026, 4, 13), today, short).0, "-1d");
        assert_eq!(render_due(d(2026, 4, 1), today, short).0, "-13d");
    }

    #[test]
    fn renders_severity_independent_of_mode() {
        let today = d(2026, 4, 14);
        for mode in [LabelMode::Long, LabelMode::Short] {
            assert_eq!(render_due(today, today, mode).1, DueSeverity::Today);
            assert_eq!(render_due(d(2026, 4, 15), today, mode).1, DueSeverity::Soon);
            assert_eq!(
                render_due(d(2026, 4, 13), today, mode).1,
                DueSeverity::Overdue
            );
            assert_eq!(
                render_due(d(2026, 5, 30), today, mode).1,
                DueSeverity::Later
            );
        }
    }
}
