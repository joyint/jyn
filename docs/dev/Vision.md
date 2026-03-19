# Jot -- Vision

Jot is a personal task manager sharing [joy-core](https://github.com/joyint/joy) as its foundation. It provides a fast, Git-native CLI for managing personal tasks with recurring schedules, due dates, and reminders. Mobile access works through CalDAV (Apple Reminders, Google Calendar) without requiring a native app.

Jot is licensed under MIT. The CLI and data format are always free and open.

For the platform and sync layer see the [platform repository](https://github.com/joyint/platform). For Joy (product management) see the [Joy repository](https://github.com/joyint/joy).

---

## Target Audience

- Developers who want a terminal-native Todoist replacement
- Anyone who prefers owning their data in a Git repo over trusting a SaaS database
- Teams using Joy for PM who want personal task management on the same platform

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

This means Jot items are valid Joy items with extra fields. The server transports YAML without needing to understand all fields -- unknown fields are accepted and passed through.

### Recurrence

Recurring tasks use an RRULE-compatible model:

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

Jot uses the `TODO` prefix for item IDs (configurable via project acronym, like Joy).

---

## CLI Commands

```sh
jot add <TITLE> [OPTIONS]               # Create a new todo
  jot add "Buy groceries" --due tomorrow --priority high
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

All titles, descriptions, and comments are in the same YAML format as Joy items.

---

## Dispatch Integration

Jot tasks can be created by the joyint.com dispatch service when Joy items reach status gates. These dispatched tasks carry a `source` field linking them back to their Joy origin:

```yaml
# .jot/items/TODO-000A-review-payment-integration.yaml
id: TODO-000A
title: "Review: Payment Integration"
type: task
status: new
source: "joy:JOY-002A"          # created by dispatch from Joy
assignee: orchidee@joyint.com
due_date: 2026-04-16
created: 2026-04-15T14:00:00Z
updated: 2026-04-15T14:00:00Z
```

When this task is completed via `jot done`, joyint.com signals back to the Joy project that the review gate for JOY-002A is satisfied. See [Joy Vision](https://github.com/joyint/joy/blob/main/docs/dev/Vision.md#dispatch-bridging-joy-and-jot) for the full dispatch flow.

AI agents receive dispatched todos the same way. An agent configured as `agent:implementer@joy` picks up assigned tasks via `jot ls --mine`, executes the work, and marks them done.

---

## Sync

Jot syncs via Git, identically to Joy:

- **CLI users:** `git push` / `git pull`
- **CalDAV users:** joyint.com maps `.jot/` items to VTODO resources (see [platform docs](https://github.com/joyint/platform/blob/main/docs/dev/Vision.md))
- **WebUI users:** joyint.com provides a web interface for Jot alongside Joy

---

## Related

For roadmap, milestones, and timeline see the [umbrella repository](https://github.com/joyint/project). For business context (pricing, licensing, competitive landscape) see [BusinessModel.md](https://github.com/joyint/project/blob/main/docs/biz/BusinessModel.md) and [Competition.md](https://github.com/joyint/project/blob/main/docs/biz/Competition.md). These documents are part of the internal planning for the Joyint product ecosystem at Joydev GmbH.
