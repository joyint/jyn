// Copyright (c) 2026 Joydev GmbH (joydev.com)
// SPDX-License-Identifier: MIT

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

/// A completed (or otherwise overridden) occurrence of a recurring task,
/// mapping to an iCalendar RECURRENCE-ID override on the series. The
/// occurrence is a calendar date for now; a time-of-day variant follows
/// with the time-capable due model (JYN-000A-B1). `comments` is reserved
/// for per-occurrence comments (JYN-000B-6B).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompletedOccurrence {
    /// The occurrence's original due date (its RECURRENCE-ID).
    pub occurrence: NaiveDate,
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
    /// The series advanced to the next occurrence on this date.
    Advanced { next: NaiveDate },
    /// The recurrence is exhausted; the task is now closed for good.
    Ended,
}

/// A Jot task extends joy-core::Item with recurrence and task-specific fields.
/// Uses serde flatten to inherit all base Item fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Task {
    #[serde(flatten)]
    pub item: joy_core::model::item::Item,

    /// Due date (date only, no time)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_date: Option<NaiveDate>,

    /// Reminder datetime
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reminder: Option<DateTime<Utc>>,

    /// Recurrence rule (RFC 5545 RRULE format)
    /// e.g. "FREQ=WEEKLY;BYDAY=MO,WE,FR" or "FREQ=DAILY;INTERVAL=2"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recurrence: Option<String>,

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
            due_date: None,
            reminder: None,
            recurrence: None,
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
        let (Some(rule), Some(due)) = (self.recurrence.clone(), self.due_date) else {
            return Ok(CompletionResult::NotRecurring);
        };
        self.completed_occurrences.push(CompletedOccurrence {
            occurrence: due,
            completed_at: now,
            comments: Vec::new(),
        });
        self.item.updated = now;
        match crate::recurrence::next_occurrence(&rule, due, due)? {
            Some(next) => {
                self.due_date = Some(next);
                self.item.status = joy_core::model::item::Status::New;
                self.closed_at = None;
                Ok(CompletionResult::Advanced { next })
            }
            None => {
                self.item.status = joy_core::model::item::Status::Closed;
                self.closed_at = Some(now);
                Ok(CompletionResult::Ended)
            }
        }
    }

    /// Reopen a single completed occurrence by its date: drop the override
    /// and make that date the current due again, reactivating the series.
    /// Returns whether a matching occurrence was found.
    pub fn reopen_occurrence(&mut self, occurrence: NaiveDate, now: DateTime<Utc>) -> bool {
        let before = self.completed_occurrences.len();
        self.completed_occurrences
            .retain(|o| o.occurrence != occurrence);
        if self.completed_occurrences.len() == before {
            return false;
        }
        // The reopened occurrence becomes current again when it precedes the
        // current due (e.g. after the series had advanced past or ended).
        if self.due_date.is_none_or(|d| occurrence < d) {
            self.due_date = Some(occurrence);
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
        task.due_date = Some(NaiveDate::from_ymd_opt(2026, 3, 24).unwrap());
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
        task.due_date = Some(day(2026, 4, 13));

        let result = task.complete_occurrence(now).unwrap();
        assert_eq!(
            result,
            CompletionResult::Advanced {
                next: day(2026, 4, 14)
            }
        );
        assert_eq!(task.due_date, Some(day(2026, 4, 14)));
        assert_eq!(task.completed_occurrences.len(), 1);
        assert_eq!(task.completed_occurrences[0].occurrence, day(2026, 4, 13));
        assert_eq!(task.item.status, joy_core::model::item::Status::New);
        assert!(task.closed_at.is_none());
    }

    #[test]
    fn complete_occurrence_ends_an_exhausted_series() {
        let now = Utc::now();
        let mut task = Task::new("JOT-2".into(), "One last time".into());
        task.recurrence = Some("FREQ=DAILY;COUNT=1".into());
        task.due_date = Some(day(2026, 4, 13));

        let result = task.complete_occurrence(now).unwrap();
        assert_eq!(result, CompletionResult::Ended);
        assert_eq!(task.item.status, joy_core::model::item::Status::Closed);
        assert!(task.closed_at.is_some());
    }

    #[test]
    fn complete_occurrence_on_non_recurring_is_a_noop_signal() {
        let mut task = Task::new("JOT-3".into(), "Buy milk".into());
        task.due_date = Some(day(2026, 4, 13));
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
        task.due_date = Some(day(2026, 4, 13));
        task.complete_occurrence(now).unwrap(); // -> due 2026-04-14, override 2026-04-13

        assert!(task.reopen_occurrence(day(2026, 4, 13), now));
        assert!(task.completed_occurrences.is_empty());
        assert_eq!(task.due_date, Some(day(2026, 4, 13)));
        assert_eq!(task.item.status, joy_core::model::item::Status::New);

        // Reopening an unknown occurrence reports not-found.
        assert!(!task.reopen_occurrence(day(2026, 1, 1), now));
    }
}
