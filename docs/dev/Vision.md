# Jot -- Vision

Jot is a personal task manager built on [joy-core](https://github.com/joyint/joy). It provides a fast, Git-native CLI for managing personal tasks with recurring schedules, due dates, and reminders. Mobile access works through CalDAV (Apple Reminders, Google Calendar) without requiring a native app.

Jot is licensed under MIT. The CLI and data format are always free and open.

---

## Target Audience

- Developers who want a terminal-native Todoist replacement
- Anyone who prefers owning their data in a Git repo over trusting a SaaS database
- Teams using Joy for product management who want personal task management on the same foundation

---

## Simplified Model

Jot uses the same `joy-core::Item` struct but with a reduced surface:

| Aspect | Joy | Jot |
|--------|-----|-----|
| Item types | 7 (epic, story, task, bug, rework, decision, idea) | 1 (`task`) |
| Status states | 6 (new, open, in-progress, review, closed, deferred) | 2 (`new`, `closed`) |
| Directory | `.joy/` | `.jot/` |
| Milestones | Yes | No (use tags or parent items instead) |
| Dependencies | Yes | No |
| Status rules | Yes (gates, roles, CI) | No |

### Extension via serde flatten

`jot-core::Todo` extends `joy-core::Item` with Jot-specific fields:

```rust
// jot-core (simplified)
#[derive(Serialize, Deserialize)]
pub struct Todo {
    #[serde(flatten)]
    pub item: Item,           // everything from joy-core
    pub recurrence: Option<Recurrence>,  // RRULE-compatible
}
```

This means Jot items are valid Joy items with extra fields. Any tool that reads Joy items can read Jot items -- unknown fields are accepted and passed through.

### Recurrence

Recurring tasks use an RRULE-compatible model (RFC 5545):

```yaml
# .jot/items/TODO-0003-weekly-review.yaml
id: TODO-0003
title: Weekly review
type: task
status: new
priority: medium
due_date: 2026-03-15
reminder: 2026-03-15T09:00:00Z
recurrence:
  freq: weekly
  interval: 1
  by_day: [fri]
created: 2026-03-09T10:00:00Z
updated: 2026-03-09T10:00:00Z
```

When a recurring todo is completed (`jot done`), the next occurrence is created automatically based on the recurrence rule.

---

## Directory Structure

```
.jot/
├── config.yaml
├── items/
│   ├── TODO-0001-buy-groceries.yaml
│   ├── TODO-0002-dentist-appointment.yaml
│   └── ...
└── log/
```

Jot uses the `TODO` prefix for item IDs (configurable via project acronym).

---

## CLI Commands

```sh
jot add <TITLE> [OPTIONS]               # Create a new todo
  jot add "Review pull request JOY-00D3" --tag work
  jot add "Dentist" --due 2026-04-01 --reminder "2026-04-01T08:00:00"
  jot add "Weekly review" --recur "weekly on fri"

jot done <ID>                           # Mark todo as done
  jot done TODO-0003                    # triggers next recurrence if set

jot ls                                  # List open todos
  jot ls --all                          # include completed
  jot ls --due today                    # due today (includes overdue)
  jot ls --mine                         # for dispatch: todos assigned to me
  jot ls --tag shopping                 # by tag

jot show <ID>                           # Detail view

jot edit <ID> [OPTIONS]                 # Modify a todo
  jot edit TODO-0001 --title "Buy organic groceries"
  jot edit TODO-0001 --due 2026-03-20

jot rm <ID>                             # Delete a todo
```

---

## Dispatch Integration

Jot tasks can be created by external services when Joy items reach status gates. These dispatched tasks carry a `source` field linking them back to their origin:

```yaml
# .jot/items/TODO-000A-review-payment-integration.yaml
id: TODO-000A
title: "Review: Payment Integration"
type: task
status: new
source: "joy:JOY-002A"          # created by dispatch from Joy
assignee: orchidee@example.com
due_date: 2026-04-16
created: 2026-04-15T14:00:00Z
updated: 2026-04-15T14:00:00Z
```

When a dispatched task is completed via `jot done`, the originating system is notified that the gate is satisfied.

AI agents receive dispatched todos the same way. An agent configured as a responsible user picks up assigned tasks via `jot ls --mine`, executes the work, and marks them done.

---

## Sync

Jot syncs via Git, identically to Joy:

- **CLI users:** `git push` / `git pull`
- **CalDAV users:** a compatible server maps `.jot/` items to VTODO resources
- **Web users:** a web interface provides browser-based access

---

## Related

Jot is part of the [Joyint ecosystem](https://github.com/joyint/project), which includes Joy (product management), a sync platform, and native apps. For the sync server and CalDAV bridge see the [platform repository](https://github.com/joyint/platform). For Joy (product management) see the [Joy repository](https://github.com/joyint/joy).
