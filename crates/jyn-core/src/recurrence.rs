// Copyright (c) 2026 Joydev GmbH (joydev.com)
// SPDX-License-Identifier: MIT

//! Recurrence rules (RFC 5545 RRULE) for jyn tasks.
//!
//! A recurring task stores its rule as an RRULE *body* string (the part
//! after `RRULE:`, e.g. `FREQ=WEEKLY;BYDAY=MO`) in `Task.recurrence`.
//! This module validates such strings and computes the next occurrence,
//! delegating the calendar arithmetic to the `rrule` crate.
//!
//! Dates are anchored at UTC midnight: jyn tracks tasks by calendar date
//! (`NaiveDate`), so the time-of-day and zone are irrelevant here and a
//! fixed anchor keeps occurrences stable regardless of the user's clock.

use chrono::{DateTime, NaiveDate, NaiveTime, TimeZone, Utc};
use rrule::{RRule, Tz, Unvalidated};

#[derive(Debug, thiserror::Error)]
#[error("invalid recurrence rule '{rule}': {message}")]
pub struct RecurrenceError {
    rule: String,
    message: String,
}

/// Translate a human-friendly recurrence input into a validated RRULE
/// body, ready to store on a task. Accepts the common phrases users
/// actually type (`daily`, `every Monday`, `every 2 weeks`, `weekdays`,
/// `monthly on the 1st`), with an optional `for N <unit>` / `for N times`
/// suffix that becomes `COUNT`. A raw RRULE body (`FREQ=...`) or an
/// `RRULE:...` line is accepted as a power-user fallback. The returned
/// string is the RRULE body for storage and is already validated.
pub fn parse_input(input: &str) -> Result<String, RecurrenceError> {
    let rule = parse_phrase(input)?;
    validate(&rule)?;
    Ok(rule)
}

/// Translate a human phrase into an RRULE body string (without validation).
fn parse_phrase(input: &str) -> Result<String, RecurrenceError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(err(input, "empty recurrence"));
    }
    let lower_full = trimmed.to_lowercase();
    // Power-user fallback: raw RRULE body, or an `RRULE:...` content line.
    if lower_full.starts_with("freq=") {
        return Ok(trimmed.to_string());
    }
    if lower_full.starts_with("rrule:") {
        return Ok(trimmed["rrule:".len()..].trim().to_string());
    }

    // Split off an optional `for N <unit>` / `for N times` suffix that
    // maps to COUNT, leaving the main phrase to drive FREQ/BYDAY/...
    let (main, count): (&str, Option<u32>) = match lower_full.find(" for ") {
        Some(idx) => {
            let main = trimmed[..idx].trim();
            let after = trimmed[idx + " for ".len()..].trim();
            let n = parse_count_phrase(after)
                .ok_or_else(|| err(input, format!("could not read count from 'for {after}'")))?;
            (main, Some(n))
        }
        None => (trimmed, None),
    };

    let main_lower = main.to_lowercase();
    let tokens: Vec<&str> = main_lower.split_whitespace().collect();

    let (freq, interval, byday, bymonthday): (&str, u32, Option<String>, Option<u32>) = match tokens
        .as_slice()
    {
        ["daily"] => ("DAILY", 1, None, None),
        ["weekly"] => ("WEEKLY", 1, None, None),
        ["monthly"] => ("MONTHLY", 1, None, None),
        ["yearly"] => ("YEARLY", 1, None, None),
        ["hourly"] => ("HOURLY", 1, None, None),
        ["weekdays"] => ("WEEKLY", 1, Some("MO,TU,WE,TH,FR".to_string()), None),
        ["every", weekday] if weekday_short(weekday).is_some() => (
            "WEEKLY",
            1,
            Some(weekday_short(weekday).unwrap().to_string()),
            None,
        ),
        ["every", n, unit] => {
            let n: u32 = n
                .parse()
                .map_err(|_| err(input, format!("expected a number, got '{n}'")))?;
            let f =
                unit_to_freq(unit).ok_or_else(|| err(input, format!("unknown unit '{unit}'")))?;
            (f, n, None, None)
        }
        ["monthly", "on", "the", day] | ["monthly", "on", day] => {
            let d = parse_day_of_month(day)
                .ok_or_else(|| err(input, format!("expected a day of the month, got '{day}'")))?;
            ("MONTHLY", 1, None, Some(d))
        }
        _ => {
            return Err(err(
                input,
                format!("not a recognised recurrence phrase: '{trimmed}'"),
            ))
        }
    };

    let mut rule = format!("FREQ={freq}");
    if interval > 1 {
        rule.push_str(&format!(";INTERVAL={interval}"));
    }
    if let Some(b) = byday {
        rule.push_str(&format!(";BYDAY={b}"));
    }
    if let Some(d) = bymonthday {
        rule.push_str(&format!(";BYMONTHDAY={d}"));
    }
    if let Some(c) = count {
        rule.push_str(&format!(";COUNT={c}"));
    }
    Ok(rule)
}

fn weekday_short(s: &str) -> Option<&'static str> {
    match s {
        "monday" | "mon" => Some("MO"),
        "tuesday" | "tue" | "tues" => Some("TU"),
        "wednesday" | "wed" => Some("WE"),
        "thursday" | "thu" | "thurs" => Some("TH"),
        "friday" | "fri" => Some("FR"),
        "saturday" | "sat" => Some("SA"),
        "sunday" | "sun" => Some("SU"),
        _ => None,
    }
}

fn unit_to_freq(unit: &str) -> Option<&'static str> {
    match unit.trim_end_matches('s') {
        "day" => Some("DAILY"),
        "week" => Some("WEEKLY"),
        "month" => Some("MONTHLY"),
        "year" => Some("YEARLY"),
        "hour" => Some("HOURLY"),
        _ => None,
    }
}

/// Accept `1`, `1st`, `2nd`, ..., `31st` and return the numeric day.
fn parse_day_of_month(s: &str) -> Option<u32> {
    let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    let n: u32 = digits.parse().ok()?;
    if (1..=31).contains(&n) {
        Some(n)
    } else {
        None
    }
}

/// Read the leading number from a `for ...` suffix (`3 times` / `3 days` / `3`).
fn parse_count_phrase(after: &str) -> Option<u32> {
    after.split_whitespace().next()?.parse().ok()
}

/// Validate an RRULE body string. Returns `Ok(())` when the rule parses
/// and builds, otherwise a [`RecurrenceError`] describing the problem.
pub fn validate(rule: &str) -> Result<(), RecurrenceError> {
    // A reference anchor is only needed to surface build-time validation
    // errors; the specific date does not affect whether a rule is valid.
    let reference =
        NaiveDate::from_ymd_opt(2000, 1, 1).ok_or_else(|| err(rule, "internal reference date"))?;
    build_set_date(rule, reference).map(|_| ())
}

/// Compute the next occurrence strictly after `after`, for a rule whose
/// series is anchored at `anchor` (typically the task's current due
/// date). Returns `Ok(None)` when the series has no further occurrence
/// (e.g. an exhausted `COUNT`/`UNTIL`).
pub fn next_occurrence(
    rule: &str,
    anchor: NaiveDate,
    after: NaiveDate,
) -> Result<Option<NaiveDate>, RecurrenceError> {
    let set = build_set_date(rule, anchor)?;
    // `RRuleSet::after` is inclusive of an exact match, so ask for two and
    // take the first occurrence whose date is strictly greater than
    // `after`. Anchor occurrences are at a fixed midnight, so there is at
    // most one per day and two results always cover the equal-plus-next
    // case (and the exhausted case yields none).
    let result = set.after(at_utc_midnight(after)).all(2);
    Ok(result
        .dates
        .iter()
        .map(DateTime::date_naive)
        .find(|d| *d > after))
}

/// Datetime-aware variant for time-bearing series (e.g. `FREQ=HOURLY`).
/// The anchor and bound are explicit UTC datetimes, so sub-day recurrence
/// works without collapsing onto a calendar day.
pub fn next_occurrence_at(
    rule: &str,
    anchor: DateTime<Utc>,
    after: DateTime<Utc>,
) -> Result<Option<DateTime<Utc>>, RecurrenceError> {
    let set = build_set_at(rule, anchor)?;
    let result = set.after(to_rrule_tz(after)).all(2);
    Ok(result
        .dates
        .iter()
        .map(|dt| dt.with_timezone(&Utc))
        .find(|dt| *dt > after))
}

/// Parse an RRULE body and build its set anchored at `anchor` (date-only,
/// UTC midnight).
fn build_set_date(rule: &str, anchor: NaiveDate) -> Result<rrule::RRuleSet, RecurrenceError> {
    let parsed: RRule<Unvalidated> = rule.parse().map_err(|e| err(rule, e))?;
    parsed
        .build(at_utc_midnight(anchor))
        .map_err(|e| err(rule, e))
}

/// Parse an RRULE body and build its set anchored at a specific UTC datetime.
fn build_set_at(rule: &str, anchor: DateTime<Utc>) -> Result<rrule::RRuleSet, RecurrenceError> {
    let parsed: RRule<Unvalidated> = rule.parse().map_err(|e| err(rule, e))?;
    parsed.build(to_rrule_tz(anchor)).map_err(|e| err(rule, e))
}

/// Convert a chrono `DateTime<Utc>` into the `rrule` crate's tz-wrapped
/// `DateTime<Tz>` (UTC).
fn to_rrule_tz(dt: DateTime<Utc>) -> DateTime<Tz> {
    Tz::UTC.from_utc_datetime(&dt.naive_utc())
}

/// Anchor a calendar date at UTC midnight as an `rrule` datetime.
fn at_utc_midnight(date: NaiveDate) -> DateTime<Tz> {
    Tz::UTC.from_utc_datetime(&date.and_time(NaiveTime::MIN))
}

fn err(rule: &str, e: impl std::fmt::Display) -> RecurrenceError {
    RecurrenceError {
        rule: rule.to_string(),
        message: e.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn validates_good_and_bad_rules() {
        assert!(validate("FREQ=WEEKLY").is_ok());
        assert!(validate("FREQ=MONTHLY;BYMONTHDAY=1").is_ok());
        assert!(validate("FREQ=WEEKLY;BYDAY=MO,WE,FR").is_ok());
        assert!(validate("FREQ=NONSENSE").is_err());
        assert!(validate("not a rule").is_err());
        assert!(validate("").is_err());
    }

    #[test]
    fn next_occurrence_basic_frequencies() {
        let anchor = d(2026, 4, 14); // Tuesday
        assert_eq!(
            next_occurrence("FREQ=DAILY", anchor, anchor).unwrap(),
            Some(d(2026, 4, 15))
        );
        assert_eq!(
            next_occurrence("FREQ=WEEKLY", anchor, anchor).unwrap(),
            Some(d(2026, 4, 21))
        );
        assert_eq!(
            next_occurrence("FREQ=MONTHLY", anchor, anchor).unwrap(),
            Some(d(2026, 5, 14))
        );
        assert_eq!(
            next_occurrence("FREQ=YEARLY", anchor, anchor).unwrap(),
            Some(d(2027, 4, 14))
        );
    }

    #[test]
    fn next_occurrence_with_interval() {
        let anchor = d(2026, 4, 14);
        assert_eq!(
            next_occurrence("FREQ=DAILY;INTERVAL=3", anchor, anchor).unwrap(),
            Some(d(2026, 4, 17))
        );
        assert_eq!(
            next_occurrence("FREQ=WEEKLY;INTERVAL=2", anchor, anchor).unwrap(),
            Some(d(2026, 4, 28))
        );
    }

    #[test]
    fn next_occurrence_exhausted_series_is_none() {
        let anchor = d(2026, 4, 14);
        // Only one occurrence (the anchor); nothing strictly after it.
        assert_eq!(
            next_occurrence("FREQ=DAILY;COUNT=1", anchor, anchor).unwrap(),
            None
        );
    }

    #[test]
    fn parse_input_bare_frequencies() {
        assert_eq!(parse_input("daily").unwrap(), "FREQ=DAILY");
        assert_eq!(parse_input("weekly").unwrap(), "FREQ=WEEKLY");
        assert_eq!(parse_input("monthly").unwrap(), "FREQ=MONTHLY");
        assert_eq!(parse_input("yearly").unwrap(), "FREQ=YEARLY");
        assert_eq!(parse_input("hourly").unwrap(), "FREQ=HOURLY");
        // Surrounding whitespace and casing are forgiven.
        assert_eq!(parse_input("  Daily ").unwrap(), "FREQ=DAILY");
    }

    #[test]
    fn parse_input_every_weekday() {
        assert_eq!(parse_input("every Monday").unwrap(), "FREQ=WEEKLY;BYDAY=MO");
        assert_eq!(parse_input("every fri").unwrap(), "FREQ=WEEKLY;BYDAY=FR");
    }

    #[test]
    fn parse_input_every_n_unit() {
        assert_eq!(
            parse_input("every 2 weeks").unwrap(),
            "FREQ=WEEKLY;INTERVAL=2"
        );
        assert_eq!(
            parse_input("every 3 days").unwrap(),
            "FREQ=DAILY;INTERVAL=3"
        );
        assert_eq!(
            parse_input("every 6 hours").unwrap(),
            "FREQ=HOURLY;INTERVAL=6"
        );
    }

    #[test]
    fn parse_input_weekdays_alias() {
        assert_eq!(
            parse_input("weekdays").unwrap(),
            "FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR"
        );
    }

    #[test]
    fn parse_input_monthly_on_the_nth() {
        assert_eq!(
            parse_input("monthly on the 1st").unwrap(),
            "FREQ=MONTHLY;BYMONTHDAY=1"
        );
        assert_eq!(
            parse_input("monthly on the 15th").unwrap(),
            "FREQ=MONTHLY;BYMONTHDAY=15"
        );
        assert_eq!(
            parse_input("monthly on 7").unwrap(),
            "FREQ=MONTHLY;BYMONTHDAY=7"
        );
    }

    #[test]
    fn parse_input_for_count_suffix() {
        assert_eq!(
            parse_input("daily for 3 days").unwrap(),
            "FREQ=DAILY;COUNT=3"
        );
        assert_eq!(
            parse_input("hourly for 3 times").unwrap(),
            "FREQ=HOURLY;COUNT=3"
        );
        assert_eq!(
            parse_input("every Monday for 5 weeks").unwrap(),
            "FREQ=WEEKLY;BYDAY=MO;COUNT=5"
        );
    }

    #[test]
    fn parse_input_raw_rrule_passthrough() {
        assert_eq!(parse_input("FREQ=DAILY").unwrap(), "FREQ=DAILY");
        assert_eq!(
            parse_input("FREQ=WEEKLY;BYDAY=MO,WE,FR").unwrap(),
            "FREQ=WEEKLY;BYDAY=MO,WE,FR"
        );
        assert_eq!(
            parse_input("RRULE:FREQ=DAILY;COUNT=2").unwrap(),
            "FREQ=DAILY;COUNT=2"
        );
    }

    #[test]
    fn parse_input_rejects_gibberish() {
        let err = parse_input("gibberish phrase").unwrap_err();
        assert!(err.message.contains("recognised") || err.message.contains("phrase"));
        assert!(parse_input("").is_err());
        assert!(parse_input("every purple weeks").is_err());
    }
}
