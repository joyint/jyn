// Copyright (c) 2026 Joydev GmbH (joydev.com)
// SPDX-License-Identifier: MIT

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

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

    /// Project this task belongs to
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,

    /// Source reference for dispatched tasks (e.g. "joy:acme/product:JOY-002A")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
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
            project: None,
            source: None,
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
}
