# Jot -- Architecture

This document defines the technical foundation for the Jot repository. It covers technology choices, repository structure, crate layout, and key design decisions.

For product vision and CLI design see [Vision.md](./Vision.md). For coding conventions, testing, and CI/CD see [CONTRIBUTING.md](../../CONTRIBUTING.md).

---

## Technology Stack

| Component                    | Version              | Rationale                                                         |
| ---------------------------- | -------------------- | ----------------------------------------------------------------- |
| **Rust**                     | 1.85 (latest stable) | Performance, single binary, type safety, memory safety            |
| **clap** (derive API)        | 4.5                  | CLI standard, shell completions                                   |
| **serde** + **serde_yml**    | 1.0 / 0.0.12         | YAML for `.jot/` files                                            |
| **thiserror**                | 2.0                  | Explicit error types in jot-core                                  |
| **anyhow**                   | 1.0                  | Convenient error handling in jot-cli                              |
| **insta**                    | 1.41                 | Snapshot testing                                                  |
| **rrule**                    | latest               | RRULE parsing and next-occurrence computation                     |

---

## Relationship to joy-core

`jot-core` depends on `joy-core` as a crates.io dependency. It extends `joy-core::Item` with recurrence support via `serde(flatten)` while inheriting the full base data model, YAML I/O, status logic, and Git integration.

```mermaid
graph TD
    JOYCORE[joy-core<br/>data model, YAML I/O, status logic,<br/>deps, validation, ID generation, git]

    JOTCORE[jot-core<br/>extends Item with recurrence,<br/>RRULE, todo-specific logic]
    JOTCLI[jot-cli<br/>personal todo CLI]

    JOYCORE --> JOTCORE
    JOTCORE --> JOTCLI
```

### Dependency strategy

`jot-core` declares a crates.io dependency on joy-core with a compatible minor version (e.g., `joy-core = "0.5"`). Any `0.5.x` patch release is picked up automatically. A minor bump (0.6) requires an explicit update in jot-core.

For local development alongside joy-core, a Cargo `paths` override can redirect to a local checkout:

```toml
# .cargo/config.toml (in a parent directory or workspace)
paths = ["../joy/crates/joy-core"]
```

External builders who clone only the jot repo get the crates.io version -- no additional setup required.

---

## Recurrence (RRULE)

Jot supports recurring todos via the iCalendar RRULE standard (RFC 5545). This ensures compatibility with CalDAV clients (Apple Reminders, Google Calendar, Thunderbird) without format conversion.

```yaml
title: Team Standup
due_date: '2026-03-19T09:00:00'
recurrence: 'FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR'
```

The `rrule` Rust crate parses RRULE strings and computes next occurrence dates, handling time zones, DST transitions, and leap years. jot-core uses it to calculate the next `due_date` when a recurring todo is completed.

Common patterns:

| Pattern | RRULE |
|---------|-------|
| Every day | `FREQ=DAILY` |
| Every weekday | `FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR` |
| Every 2 weeks | `FREQ=WEEKLY;INTERVAL=2` |
| First of every month | `FREQ=MONTHLY;BYMONTHDAY=1` |
| Every year | `FREQ=YEARLY` |

---

## Repository Structure

```
jot/
├── Cargo.toml                  # Workspace root
├── Cargo.lock
├── LICENSE                     # MIT license
├── CONTRIBUTING.md             # Coding conventions, testing, CI/CD
├── README.md
├── docs/
│   └── dev/
│       ├── Vision.md           # Product vision, CLI design
│       └── Architecture.md     # This file
├── crates/
│   ├── jot-core/               # Todo extension: recurrence, RRULE (MIT)
│   │   ├── Cargo.toml          # Depends on joy-core
│   │   └── src/
│   └── jot-cli/                # Personal todo CLI binary (MIT)
│       ├── Cargo.toml          # Depends on jot-core
│       └── src/
│           ├── main.rs
│           └── commands/       # One module per command (add, done, ls, show, edit, rm)
├── tests/                      # Integration tests
│   ├── cli/                    # CLI integration tests
│   └── fixtures/               # Test data (.jot/ directories)
├── .joy/                       # Product backlog (managed by joy CLI)
├── .github/
│   └── workflows/              # CI/CD
├── .claude/                    # Claude Code context
│   └── CLAUDE.md
└── justfile                    # Task runner (just)
```

---

## Cargo Workspace

```toml
# Cargo.toml (workspace root)
[workspace]
resolver = "2"
members = [
    "crates/jot-core",
    "crates/jot-cli",
]

[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_yml = "0.0.12"
thiserror = "2.0"
anyhow = "1.0"
clap = { version = "4.5", features = ["derive"] }
```

---

## Data Format

### YAML storage

All data lives in `.jot/items/*.yaml` files, one file per todo. The format follows the same conventions as joy-core items (see [ADR-001](https://github.com/joyint/project/blob/main/docs/dev/adr/ADR-001-yaml-over-sqlite.md) for rationale). Files are human-readable, diffable, and mergeable with standard Git tools.

### YAML schema evolution

The `.jot/config.yaml` contains a `version` field (currently `1`). Schema evolution rules:

- **New fields** are always optional with sensible defaults. Old files work without migration.
- **Fields are never renamed or removed**, only deprecated and ignored.
- **Incompatible schema changes** increment the version. Jot detects the old version, migrates automatically on the next write, and updates the version field.
- **Newer format, older tool**: if Jot encounters a version higher than it understands, it refuses to operate with a clear error message suggesting an update.

---

## Performance Targets

- `jot add`: <100ms (quick capture must feel instant)
- `jot ls`: <30ms for unfiltered list
- `jot done`: <100ms including recurrence calculation
- Recurrence computation: <10ms for 100 recurring todos
- Binary size: <5MB

Performance targets are enforced by timing assertions in CI tests.

---

## Licensing

Both crates (`jot-core`, `jot-cli`) are MIT-licensed. Every source file carries an SPDX license header.

---

## Key Design Decisions

Architectural decisions that affect Jot are documented as ADRs. The most relevant ones:

- [ADR-001: YAML over SQLite for data storage](https://github.com/joyint/project/blob/main/docs/dev/adr/ADR-001-yaml-over-sqlite.md)
- [ADR-008: Open Core Licensing Model](https://github.com/joyint/project/blob/main/docs/dev/adr/ADR-008-open-core-licensing.md)
- [ADR-010: VCS abstraction layer](https://github.com/joyint/project/blob/main/docs/dev/adr/ADR-010-vcs-abstraction.md)
- [ADR-011: YAML-aware merge strategy for conflict resolution](https://github.com/joyint/project/blob/main/docs/dev/adr/ADR-011-yaml-aware-merge-strategy.md)

The full list of ADRs is maintained in the [Joyint project repository](https://github.com/joyint/project/tree/main/docs/dev/adr).
