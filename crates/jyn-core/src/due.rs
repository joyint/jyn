// Copyright (c) 2026 Joydev GmbH (joydev.com)
// SPDX-License-Identifier: MIT

//! Minimal due-date parsing and rendering.
//!
//! Accepts the common forms a personal task manager needs:
//!   - `today`, `tomorrow`
//!   - weekday names: `fri`, `friday`, `next monday` (next future occurrence)
//!   - relative offsets: `+3d`, `3d`, `+1w`, `2w` (added to today)
//!   - `YYYY-MM-DD` (ISO 8601 calendar date)
//!   - `YYYY-MM-DD HH:MM` or `YYYY-MM-DDTHH:MM` (date + time-of-day,
//!     interpreted in the display tz and stored as UTC)
//!   - `MM-DD`      (current year implied)
//!   - `DD.MM`      (German short form, current year implied)
//!   - `DD.MM.YYYY` (German long form)
//!
//! Weekday names always resolve to the next matching day in the future,
//! never today (so `friday` on a Friday means the following Friday); the
//! optional `next ` prefix is accepted as a synonym. The parser returns a
//! structured error for anything it does not recognise so the CLI can
//! surface a useful hint. See JOT-0032-69 (weekdays/offsets) and
//! JYN-000C-B5 (optional time-of-day).
//!
//! Rendering produces short human-readable labels for a list view,
//! with a side-channel severity so the CLI can colorise consistently.

use chrono::{
    DateTime, Datelike, Duration, Local, NaiveDate, NaiveDateTime, TimeZone, Utc, Weekday,
};

use crate::model::Due;

#[derive(Debug, thiserror::Error)]
#[error("cannot parse due date '{input}': expected 'today', 'tomorrow', a weekday (e.g. 'fri', 'next monday'), an offset (e.g. '+3d', '2w'), YYYY-MM-DD, MM-DD, DD.MM, or DD.MM.YYYY")]
pub struct ParseDueError {
    input: String,
}

/// Parse a `--due` argument against a reference 'today' date.
pub fn parse_due(input: &str, today: NaiveDate) -> Result<Due, ParseDueError> {
    let trimmed = input.trim();
    let lower = trimmed.to_lowercase();
    if lower == "today" {
        return Ok(Due::Date(today));
    }
    if lower == "tomorrow" {
        return Ok(Due::Date(today + Duration::days(1)));
    }
    if let Some(date) = parse_weekday(&lower, today) {
        return Ok(Due::Date(date));
    }
    if let Some(date) = parse_offset(&lower, today) {
        return Ok(Due::Date(date));
    }
    // Time-bearing forms (date + HH:MM, with or without 'T') before the
    // bare-date attempts, so a substring like "2026-04-13" does not win
    // over "2026-04-13 14:00".
    if let Some(dt) = parse_datetime(trimmed, input)? {
        return Ok(Due::DateTime(dt));
    }

    let year = today.year();
    if let Ok(d) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        return Ok(Due::Date(d));
    }
    if let Ok(d) = NaiveDate::parse_from_str(trimmed, "%d.%m.%Y") {
        return Ok(Due::Date(d));
    }
    if let Ok(d) = NaiveDate::parse_from_str(&format!("{year}-{trimmed}"), "%Y-%m-%d") {
        return Ok(Due::Date(d));
    }
    if let Ok(d) = NaiveDate::parse_from_str(&format!("{trimmed}.{year}"), "%d.%m.%Y") {
        return Ok(Due::Date(d));
    }
    Err(ParseDueError {
        input: input.to_string(),
    })
}

/// Try the time-bearing forms (`YYYY-MM-DD HH:MM[:SS]`, `YYYY-MM-DDTHH:MM[:SS]`,
/// RFC 3339 with `Z`). A naive datetime is interpreted in `chrono::Local` and
/// converted to UTC; an ambiguous local time (DST fold/gap) is rejected so the
/// stored value is unambiguous.
fn parse_datetime(trimmed: &str, original: &str) -> Result<Option<DateTime<Utc>>, ParseDueError> {
    let local_forms = [
        "%Y-%m-%d %H:%M",
        "%Y-%m-%dT%H:%M",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S",
    ];
    for fmt in local_forms {
        if let Ok(ndt) = NaiveDateTime::parse_from_str(trimmed, fmt) {
            return match Local.from_local_datetime(&ndt).single() {
                Some(local) => Ok(Some(local.with_timezone(&Utc))),
                None => Err(ParseDueError {
                    input: original.to_string(),
                }),
            };
        }
    }
    // Already-zoned (e.g. `2026-04-13T14:00:00Z` or with an offset).
    if let Ok(dt) = DateTime::parse_from_rfc3339(trimmed) {
        return Ok(Some(dt.with_timezone(&Utc)));
    }
    Ok(None)
}

/// Map a full or three-letter weekday name to a `chrono::Weekday`.
fn weekday_from_name(s: &str) -> Option<Weekday> {
    Some(match s {
        "monday" | "mon" => Weekday::Mon,
        "tuesday" | "tue" => Weekday::Tue,
        "wednesday" | "wed" => Weekday::Wed,
        "thursday" | "thu" => Weekday::Thu,
        "friday" | "fri" => Weekday::Fri,
        "saturday" | "sat" => Weekday::Sat,
        "sunday" | "sun" => Weekday::Sun,
        _ => return None,
    })
}

/// Parse a weekday name into the next matching date strictly after today.
/// Accepts an optional `next ` prefix as a synonym. Returns `None` when
/// the input is not a weekday name.
fn parse_weekday(lower: &str, today: NaiveDate) -> Option<NaiveDate> {
    let name = lower.strip_prefix("next ").unwrap_or(lower).trim();
    let target = weekday_from_name(name)?;
    let today_idx = today.weekday().num_days_from_monday();
    let target_idx = target.num_days_from_monday();
    // 0 means the weekday is today; map it to a week out so a weekday
    // name is always a future date (1..=7), never today.
    let raw = (target_idx + 7 - today_idx) % 7;
    let ahead = if raw == 0 { 7 } else { raw };
    Some(today + Duration::days(ahead as i64))
}

/// Parse a relative offset like `+3d`, `3d`, `+1w`, or `2w` into a date
/// `n` days/weeks after today. The leading `+` is optional. Returns
/// `None` when the input is not a recognised offset.
fn parse_offset(lower: &str, today: NaiveDate) -> Option<NaiveDate> {
    let body = lower.strip_prefix('+').unwrap_or(lower);
    let (num, unit) = body.split_at(body.len().checked_sub(1)?);
    // u32 rejects empty, negative, and non-digit numerators.
    let n = i64::from(num.parse::<u32>().ok()?);
    match unit {
        "d" => Some(today + Duration::days(n)),
        "w" => Some(today + Duration::weeks(n)),
        _ => None,
    }
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
        assert_eq!(parse_due("today", today).unwrap(), Due::Date(today));
        assert_eq!(
            parse_due("TOMORROW", today).unwrap(),
            Due::Date(d(2026, 4, 15))
        );
        assert_eq!(
            parse_due("2026-12-31", today).unwrap(),
            Due::Date(d(2026, 12, 31))
        );
    }

    #[test]
    fn parses_current_year_shortcuts() {
        let today = d(2026, 4, 14);
        assert_eq!(
            parse_due("04-25", today).unwrap(),
            Due::Date(d(2026, 4, 25))
        );
        assert_eq!(
            parse_due("25.04", today).unwrap(),
            Due::Date(d(2026, 4, 25))
        );
        assert_eq!(
            parse_due("25.04.2027", today).unwrap(),
            Due::Date(d(2027, 4, 25))
        );
    }

    #[test]
    fn parses_weekday_names() {
        let today = d(2026, 4, 14); // Tuesday
                                    // Upcoming weekday within the week.
        assert_eq!(
            parse_due("friday", today).unwrap(),
            Due::Date(d(2026, 4, 17))
        );
        assert_eq!(parse_due("fri", today).unwrap(), Due::Date(d(2026, 4, 17)));
        assert_eq!(
            parse_due("Sunday", today).unwrap(),
            Due::Date(d(2026, 4, 19))
        );
        // Today's own weekday resolves to the following week, never today.
        assert_eq!(parse_due("tue", today).unwrap(), Due::Date(d(2026, 4, 21)));
        // The optional "next " prefix is accepted as a synonym.
        assert_eq!(
            parse_due("next monday", today).unwrap(),
            Due::Date(d(2026, 4, 20))
        );
        assert_eq!(parse_due("mon", today).unwrap(), Due::Date(d(2026, 4, 20)));
    }

    #[test]
    fn parses_relative_offsets() {
        let today = d(2026, 4, 14);
        assert_eq!(parse_due("+3d", today).unwrap(), Due::Date(d(2026, 4, 17)));
        assert_eq!(parse_due("3d", today).unwrap(), Due::Date(d(2026, 4, 17)));
        assert_eq!(parse_due("+1w", today).unwrap(), Due::Date(d(2026, 4, 21)));
        assert_eq!(parse_due("2w", today).unwrap(), Due::Date(d(2026, 4, 28)));
    }

    #[test]
    fn rejects_unsupported_forms() {
        let today = d(2026, 4, 14);
        assert!(parse_due("someday", today).is_err());
        assert!(parse_due("3", today).is_err()); // offset needs a unit
        assert!(parse_due("+3x", today).is_err()); // unknown unit
        assert!(parse_due("+w", today).is_err()); // missing count
        assert!(parse_due("-3d", today).is_err()); // no negative offsets
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
