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

use chrono::{DateTime, NaiveDate, NaiveTime, TimeZone};
use rrule::{RRule, Tz, Unvalidated};

#[derive(Debug, thiserror::Error)]
#[error("invalid recurrence rule '{rule}': {message}")]
pub struct RecurrenceError {
    rule: String,
    message: String,
}

/// Validate an RRULE body string. Returns `Ok(())` when the rule parses
/// and builds, otherwise a [`RecurrenceError`] describing the problem.
pub fn validate(rule: &str) -> Result<(), RecurrenceError> {
    // A reference anchor is only needed to surface build-time validation
    // errors; the specific date does not affect whether a rule is valid.
    let reference =
        NaiveDate::from_ymd_opt(2000, 1, 1).ok_or_else(|| err(rule, "internal reference date"))?;
    build_set(rule, reference).map(|_| ())
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
    let set = build_set(rule, anchor)?;
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

/// Parse an RRULE body and build its set anchored at `anchor`.
fn build_set(rule: &str, anchor: NaiveDate) -> Result<rrule::RRuleSet, RecurrenceError> {
    let parsed: RRule<Unvalidated> = rule.parse().map_err(|e| err(rule, e))?;
    parsed
        .build(at_utc_midnight(anchor))
        .map_err(|e| err(rule, e))
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
}
