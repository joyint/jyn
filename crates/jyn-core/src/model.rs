// Copyright (c) 2026 Joydev GmbH (joydev.com)
// SPDX-License-Identifier: MIT

use chrono::{DateTime, NaiveDate, NaiveTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

/// A task's due value, either a calendar date or a precise UTC datetime.
/// Mirrors iCalendar's `DUE`/`DTSTART` `VALUE=DATE` vs `VALUE=DATE-TIME`
/// distinction and Microsoft To Do's `dueDateTime`. Storage is UTC for
/// the datetime variant per JOY-01A1-3A; display converts to the
/// configured timezone (else machine local). Serde is untagged so the
/// YAML value alone (a `YYYY-MM-DD` string or an RFC 3339 datetime
/// string with `Z`) selects the variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Due {
    /// A UTC datetime (the time-bearing case; enables sub-day recurrence).
    DateTime(DateTime<Utc>),
    /// A calendar date with no time component (the timezone-independent case).
    Date(NaiveDate),
}

impl Due {
    /// The calendar-date component, useful for date-bucket comparisons
    /// (e.g. filtering by today's date independent of time-of-day).
    pub fn date(self) -> NaiveDate {
        match self {
            Due::Date(d) => d,
            Due::DateTime(dt) => dt.date_naive(),
        }
    }

    /// True if this due carries a time-of-day.
    pub fn has_time(self) -> bool {
        matches!(self, Due::DateTime(_))
    }

    /// UTC instant for chronological comparison. Date variants are pinned
    /// at 00:00 UTC; DateTime variants are returned as-is.
    pub fn as_utc_instant(self) -> DateTime<Utc> {
        match self {
            Due::Date(d) => Utc.from_utc_datetime(&d.and_time(NaiveTime::MIN)),
            Due::DateTime(dt) => dt,
        }
    }
}

impl From<NaiveDate> for Due {
    fn from(d: NaiveDate) -> Self {
        Due::Date(d)
    }
}

impl From<DateTime<Utc>> for Due {
    fn from(dt: DateTime<Utc>) -> Self {
        Due::DateTime(dt)
    }
}

impl std::fmt::Display for Due {
    /// Stable string form used in occurrence addressing (`#1@DATE` /
    /// `#1@DATETIME`) and basic output. Date variants render as
    /// `YYYY-MM-DD`; DateTime variants render as `YYYY-MM-DDTHH:MM` in
    /// UTC for now (a display-tz helper in joy-core will localise later
    /// per JOY-01A1-3A).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Due::Date(d) => write!(f, "{d}"),
            Due::DateTime(dt) => write!(f, "{}", dt.format("%Y-%m-%dT%H:%M")),
        }
    }
}

/// A completed (or otherwise overridden) occurrence of a recurring task,
/// mapping to an iCalendar RECURRENCE-ID override on the series. The
/// occurrence is a calendar date for now; a time-of-day variant follows
/// with the time-capable due model (JYN-000A-B1). `comments` is reserved
/// for per-occurrence comments (JYN-000B-6B).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompletedOccurrence {
    /// The occurrence's original due value (its RECURRENCE-ID): either a
    /// date (date-only series) or a UTC datetime (time-bearing series).
    pub occurrence: Due,
    /// When this occurrence was completed (UTC).
    pub completed_at: DateTime<Utc>,
    /// Per-occurrence comments. Reserved; populated by JYN-000B-6B.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<joy_core::model::item::Comment>,
}

/// Outcome of completing one occurrence of a (possibly recurring) task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionResult {
    /// Not a recurring series with a due anchor; the caller should do a
    /// plain close instead.
    NotRecurring,
    /// The series advanced to the next occurrence.
    Advanced { next: Due },
    /// The recurrence is exhausted; the task is now closed for good.
    Ended,
}

/// A Jot task extends joy-core::Item with recurrence and task-specific fields.
/// Uses serde flatten to inherit all base Item fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Task {
    #[serde(flatten)]
    pub item: joy_core::model::item::Item,

    /// Due value: either a calendar date or a UTC datetime. The YAML key
    /// is `due` (the field now carries an optional time too, so the
    /// earlier `due_date` name is no longer accurate); old date-only
    /// files using `due_date:` are still read via the serde alias.
    #[serde(default, alias = "due_date", skip_serializing_if = "Option::is_none")]
    pub due: Option<Due>,

    /// Reminder datetime
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reminder: Option<DateTime<Utc>>,

    /// Recurrence rule (RFC 5545 RRULE format)
    /// e.g. "FREQ=WEEKLY;BYDAY=MO,WE,FR" or "FREQ=DAILY;INTERVAL=2"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recurrence: Option<String>,

    /// IANA timezone anchoring a time-bearing recurrence (e.g. `Europe/Berlin`),
    /// mapping to iCalendar `DTSTART;TZID=...` so a local wall-clock time
    /// stays stable across DST. Omitted for date-only or UTC recurrences.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recurrence_tz: Option<String>,

    /// First occurrence of the series (its `DTSTART`), set lazily on the
    /// first completion. Used as the rule's anchor so `COUNT`/`UNTIL`
    /// bounds count occurrences from the start rather than from a moving
    /// `due`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recurrence_anchor: Option<Due>,

    /// Completed occurrences of a recurring series (RECURRENCE-ID
    /// overrides). Empty for non-recurring tasks.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub completed_occurrences: Vec<CompletedOccurrence>,

    /// Project this task belongs to
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,

    /// Source reference for dispatched tasks (e.g. "joy:acme/product:JOY-002A")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    /// Timestamp the task was closed. Mirrors VTODO's `COMPLETED` and
    /// MS Graph's `completedDateTime`. Written by `jyn close`, cleared
    /// by `jyn reopen`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closed_at: Option<DateTime<Utc>>,

    /// Whether the task has been archived. Git-only concept, never
    /// propagated to CalDAV or MS Graph.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub archived: bool,

    /// Timestamp the task was archived. Written by `jyn archive`,
    /// cleared by `jyn unarchive`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archived_at: Option<DateTime<Utc>>,
}

impl Task {
    pub fn new(id: String, title: String) -> Self {
        Self {
            item: joy_core::model::item::Item::new(
                id,
                title,
                joy_core::model::item::ItemType::Task,
                joy_core::model::item::Priority::Medium,
                vec![joy_core::model::item::Capability::Implement],
            ),
            due: None,
            reminder: None,
            recurrence: None,
            recurrence_tz: None,
            recurrence_anchor: None,
            completed_occurrences: Vec::new(),
            project: None,
            source: None,
            closed_at: None,
            archived: false,
            archived_at: None,
        }
    }

    /// Check if this task is recurring
    pub fn is_recurring(&self) -> bool {
        self.recurrence.is_some()
    }

    /// Check if this task was created by dispatch
    pub fn is_dispatched(&self) -> bool {
        self.source.is_some()
    }

    /// Complete the current occurrence of a recurring task: record it as an
    /// override, then advance the due date to the next occurrence, or close
    /// the series for good if the rule is exhausted. Returns `NotRecurring`
    /// when the task is not a recurring series with a due date, so the
    /// caller does a plain close instead.
    pub fn complete_occurrence(
        &mut self,
        now: DateTime<Utc>,
    ) -> Result<CompletionResult, crate::recurrence::RecurrenceError> {
        let (Some(rule), Some(due)) = (self.recurrence.clone(), self.due) else {
            return Ok(CompletionResult::NotRecurring);
        };
        // Capture the first occurrence as the rule's anchor (DTSTART) so
        // `COUNT`/`UNTIL` bounds stay relative to the start of the series,
        // not the current (advancing) due.
        let anchor = *self.recurrence_anchor.get_or_insert(due);
        self.completed_occurrences.push(CompletedOccurrence {
            occurrence: due,
            completed_at: now,
            comments: Vec::new(),
        });
        self.item.updated = now;
        // Date-only series step in dates; time-bearing series step in UTC
        // datetimes so sub-day FREQ (HOURLY) works without collapsing.
        let next: Option<Due> = match (anchor, due) {
            (Due::Date(a), Due::Date(d)) => {
                crate::recurrence::next_occurrence(&rule, a, d)?.map(Due::Date)
            }
            (Due::DateTime(a), Due::DateTime(d)) => {
                crate::recurrence::next_occurrence_at(&rule, a, d)?.map(Due::DateTime)
            }
            // Anchor and current due disagree on the variant (data drift);
            // fall back to the current due as the anchor to stay safe.
            (_, Due::Date(d)) => crate::recurrence::next_occurrence(&rule, d, d)?.map(Due::Date),
            (_, Due::DateTime(d)) => {
                crate::recurrence::next_occurrence_at(&rule, d, d)?.map(Due::DateTime)
            }
        };
        match next {
            Some(n) => {
                self.due = Some(n);
                self.item.status = joy_core::model::item::Status::New;
                self.closed_at = None;
                Ok(CompletionResult::Advanced { next: n })
            }
            None => {
                self.item.status = joy_core::model::item::Status::Closed;
                self.closed_at = Some(now);
                Ok(CompletionResult::Ended)
            }
        }
    }

    /// Reopen a single completed occurrence: drop the override and make
    /// that occurrence the current due again, reactivating the series.
    /// Returns whether a matching occurrence was found.
    pub fn reopen_occurrence(&mut self, occurrence: Due, now: DateTime<Utc>) -> bool {
        let before = self.completed_occurrences.len();
        self.completed_occurrences
            .retain(|o| o.occurrence != occurrence);
        if self.completed_occurrences.len() == before {
            return false;
        }
        // The reopened occurrence becomes current again when it precedes the
        // current due (e.g. after the series had advanced past or ended).
        if self
            .due
            .is_none_or(|d| occurrence.as_utc_instant() < d.as_utc_instant())
        {
            self.due = Some(occurrence);
        }
        self.item.status = joy_core::model::item::Status::New;
        self.closed_at = None;
        self.item.updated = now;
        true
    }
}

/// A Jot project groups tasks by theme or dispatch source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub title: String,

    /// Source workspace for dispatch projects (e.g. "joy:acme/product")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_new_defaults() {
        let task = Task::new("JOT-0001".into(), "Buy milk".into());
        assert_eq!(task.item.id, "JOT-0001");
        assert_eq!(task.item.title, "Buy milk");
        assert_eq!(task.item.item_type, joy_core::model::item::ItemType::Task);
        assert!(!task.is_recurring());
        assert!(!task.is_dispatched());
    }

    #[test]
    fn task_serialization_roundtrip() {
        let mut task = Task::new("JOT-0001".into(), "Weekly standup".into());
        task.recurrence = Some("FREQ=WEEKLY;BYDAY=MO".into());
        task.due = Some(Due::Date(NaiveDate::from_ymd_opt(2026, 3, 24).unwrap()));
        task.project = Some("JOT-P-01".into());

        let yaml = serde_yaml_ng::to_string(&task).unwrap();
        let parsed: Task = serde_yaml_ng::from_str(&yaml).unwrap();

        assert_eq!(parsed.item.id, "JOT-0001");
        assert_eq!(parsed.recurrence, Some("FREQ=WEEKLY;BYDAY=MO".into()));
        assert!(parsed.is_recurring());
        assert_eq!(parsed.project, Some("JOT-P-01".into()));
    }

    #[test]
    fn dispatched_task() {
        let mut task = Task::new("JOT-0003".into(), "Review JOY-002A".into());
        task.source = Some("joy:acme/product:JOY-002A".into());
        task.project = Some("JOT-P-03".into());

        assert!(task.is_dispatched());
    }

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn complete_occurrence_advances_a_recurring_series() {
        let now = Utc::now();
        let mut task = Task::new("JOT-1".into(), "Water plants".into());
        task.recurrence = Some("FREQ=DAILY".into());
        task.due = Some(Due::Date(day(2026, 4, 13)));

        let result = task.complete_occurrence(now).unwrap();
        assert_eq!(
            result,
            CompletionResult::Advanced {
                next: Due::Date(day(2026, 4, 14))
            }
        );
        assert_eq!(task.due, Some(Due::Date(day(2026, 4, 14))));
        assert_eq!(task.completed_occurrences.len(), 1);
        assert_eq!(
            task.completed_occurrences[0].occurrence,
            Due::Date(day(2026, 4, 13))
        );
        assert_eq!(task.item.status, joy_core::model::item::Status::New);
        assert!(task.closed_at.is_none());
    }

    #[test]
    fn complete_occurrence_ends_an_exhausted_series() {
        let now = Utc::now();
        let mut task = Task::new("JOT-2".into(), "One last time".into());
        task.recurrence = Some("FREQ=DAILY;COUNT=1".into());
        task.due = Some(Due::Date(day(2026, 4, 13)));

        let result = task.complete_occurrence(now).unwrap();
        assert_eq!(result, CompletionResult::Ended);
        assert_eq!(task.item.status, joy_core::model::item::Status::Closed);
        assert!(task.closed_at.is_some());
    }

    #[test]
    fn complete_occurrence_on_non_recurring_is_a_noop_signal() {
        let mut task = Task::new("JOT-3".into(), "Buy milk".into());
        task.due = Some(Due::Date(day(2026, 4, 13)));
        assert_eq!(
            task.complete_occurrence(Utc::now()).unwrap(),
            CompletionResult::NotRecurring
        );
        assert!(task.completed_occurrences.is_empty());
    }

    #[test]
    fn reopen_occurrence_drops_the_override_and_rolls_due_back() {
        let now = Utc::now();
        let mut task = Task::new("JOT-4".into(), "Water plants".into());
        task.recurrence = Some("FREQ=DAILY".into());
        task.due = Some(Due::Date(day(2026, 4, 13)));
        task.complete_occurrence(now).unwrap(); // -> due 2026-04-14, override 2026-04-13

        assert!(task.reopen_occurrence(Due::Date(day(2026, 4, 13)), now));
        assert!(task.completed_occurrences.is_empty());
        assert_eq!(task.due, Some(Due::Date(day(2026, 4, 13))));
        assert_eq!(task.item.status, joy_core::model::item::Status::New);

        // Reopening an unknown occurrence reports not-found.
        assert!(!task.reopen_occurrence(Due::Date(day(2026, 1, 1)), now));
    }

    #[test]
    fn complete_occurrence_advances_an_hourly_time_bearing_series() {
        let now = Utc::now();
        let mut task = Task::new("JOT-5".into(), "Health check".into());
        task.recurrence = Some("FREQ=HOURLY".into());
        let anchor = Utc.with_ymd_and_hms(2026, 4, 13, 14, 0, 0).unwrap();
        task.due = Some(Due::DateTime(anchor));

        let result = task.complete_occurrence(now).unwrap();
        let next = Utc.with_ymd_and_hms(2026, 4, 13, 15, 0, 0).unwrap();
        assert_eq!(
            result,
            CompletionResult::Advanced {
                next: Due::DateTime(next)
            }
        );
        assert_eq!(task.due, Some(Due::DateTime(next)));
        assert_eq!(
            task.completed_occurrences[0].occurrence,
            Due::DateTime(anchor)
        );
    }
}
