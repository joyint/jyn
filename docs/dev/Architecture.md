# Jot -- Architecture

This document defines the technical foundation for the Jot repository. It covers technology choices, repository structure, and crate layout.

For product vision and CLI design see [Vision.md](./Vision.md). For coding conventions, testing, and CI/CD see [CONTRIBUTING.md](../../CONTRIBUTING.md). For cross-project architecture and ADRs see the [umbrella repository](https://github.com/joyint/project).

---

## Technology Stack

Jot uses the same Rust toolchain and dependency versions as [Joy](https://github.com/joyint/joy/blob/main/docs/dev/Architecture.md#technology-stack). Key dependencies:

| Component                    | Version              | Rationale                                                         |
| ---------------------------- | -------------------- | ----------------------------------------------------------------- |
| **Rust**                     | 1.85 (latest stable) | Performance, single binary, type safety, memory safety            |
| **clap** (derive API)        | 4.5                  | CLI standard, shell completions                                   |
| **serde** + **serde_yml**    | 1.0 / 0.0.12         | YAML for `.jot/` files                                            |
| **thiserror**                | 2.0                  | Explicit error types in jot-core                                  |
| **anyhow**                   | 1.0                  | Convenient error handling in jot-cli                              |
| **insta**                    | 1.41                 | Snapshot testing                                                  |

---

## Relationship to joy-core

`jot-core` depends on `joy-core` as a crates.io dependency. It extends `joy-core::Item` with recurrence support via `serde(flatten)` while inheriting the full base data model, YAML I/O, status logic, and Git integration (see [ADR-010](https://github.com/joyint/project/blob/main/docs/dev/adr/ADR-010-vcs-abstraction.md)).

```mermaid
graph TD
    JOYCORE[joy-core<br/>joyint/joy repo<br/>data model, YAML I/O, status logic,<br/>deps, validation, ID generation, git]

    JOTCORE[jot-core<br/>extends Item with recurrence,<br/>RRULE, todo-specific logic]
    JOTCLI[jot-cli<br/>personal todo CLI]

    JOYCORE --> JOTCORE
    JOTCORE --> JOTCLI
```

### Dependency strategy

`jot-core` declares a crates.io dependency on joy-core with a compatible minor version (e.g., `joy-core = "0.5"`). Any `0.5.x` patch release is picked up automatically. A minor bump (0.6) requires an explicit update in jot-core.

For internal development in the [umbrella repository](https://github.com/joyint/project), a Cargo `paths` override redirects joy-core to the local checkout:

```toml
# project/.cargo/config.toml
paths = ["joy/crates/joy-core"]
```

Cargo finds this config file automatically when building from any subdirectory of the umbrella. External builders (e.g., AUR packages) who clone only the jot repo get the crates.io version -- no umbrella required.

---

## Recurrence (RRULE)

Jot supports recurring todos via the iCalendar RRULE standard (RFC 5545). This ensures compatibility with CalDAV clients (Apple Reminders, Google Calendar, Thunderbird) without format conversion.

```yaml
title: Team Standup
due_date: '2026-03-19T09:00:00'
recurrence: 'FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR'
```

The `rrule` Rust crate parses RRULE strings and computes the next occurrence dates, handling time zones, DST transitions, and leap years. jot-core uses it to calculate the next `due_date` when a recurring todo is completed.

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

## Performance Targets

- `jot add`: <100ms (quick capture must feel instant)
- `jot ls`: <30ms for unfiltered list
- `jot done`: <100ms including recurrence calculation
- Recurrence computation: <10ms for 100 recurring todos
- Binary size: <5MB

Performance targets are enforced by timing assertions in CI tests. Regressions fail the build.

---

## Licensing

Both crates (`jot-core`, `jot-cli`) are MIT-licensed. See [ADR-008](https://github.com/joyint/project/blob/main/docs/dev/adr/ADR-008-open-core-licensing.md) for the open-core licensing rationale.

---

## Architecture Decision Records

ADRs are maintained in the [umbrella repository](https://github.com/joyint/project/tree/main/docs/dev/adr). Key ADRs relevant to Jot:

- [ADR-001: YAML over SQLite for data storage](https://github.com/joyint/project/blob/main/docs/dev/adr/ADR-001-yaml-over-sqlite.md)
- [ADR-005: Package name `joyint`, binary name `joy`](https://github.com/joyint/project/blob/main/docs/dev/adr/ADR-005-package-name-joyint.md)
- [ADR-008: Open Core Licensing Model](https://github.com/joyint/project/blob/main/docs/dev/adr/ADR-008-open-core-licensing.md)
- [ADR-010: VCS abstraction layer](https://github.com/joyint/project/blob/main/docs/dev/adr/ADR-010-vcs-abstraction.md)
- [ADR-011: YAML-aware merge strategy for conflict resolution](https://github.com/joyint/project/blob/main/docs/dev/adr/ADR-011-yaml-aware-merge-strategy.md)
