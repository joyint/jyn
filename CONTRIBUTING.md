# Contributing

This document covers the coding conventions, testing strategy, CI/CD pipeline, and commit message format for the Jot repository.

For product vision and CLI design see [docs/dev/Vision.md](docs/dev/Vision.md). For technology choices and architecture see [docs/dev/Architecture.md](docs/dev/Architecture.md).

The product backlog lives in `.joy/` and is managed with the `joy` CLI. Run `joy` for a board overview, `joy ls` to list items, `joy show <ID>` for details.

---

## Documentation Rules

**No emoji in technical documentation.** Emoji are a runtime feature of the CLI (configurable, deactivatable). They do not belong in technical docs (vision, architecture, code comments) or commit messages. README.md and user-facing materials may use emoji sparingly for warmth.

**No ASCII diagrams.** Always use Mermaid for diagrams. This applies to architecture diagrams, flowcharts, state machines, sequence diagrams, and any other visual representation.

**No ASCII box-drawing** for architecture or flow visualizations. File tree listings (using standard `tree` output characters) are acceptable.

---

## Coding Conventions

**Fix root causes, not symptoms.** Do not add workarounds, feature flags, or conditional logic for temporary problems.

### Rust

**Edition:** 2021 (or latest stable)

**Formatting:** `rustfmt` with default settings. Always run `cargo fmt --all` before committing.

**Linting:** `clippy` at `warn` level in CI, with `#[deny(clippy::all)]` in library crates. Run `cargo clippy --workspace -- -D warnings` before pushing.

**Naming:**

- Types: `PascalCase`
- Functions/methods: `snake_case`
- Constants: `SCREAMING_SNAKE_CASE`
- Crate names: `jot-core`, `jot-cli` (kebab-case)
- Module names: `snake_case`

**Error handling:**

- `jot-core` uses `thiserror` enums -- every error type is explicit and matchable
- `jot-cli` uses `anyhow` for convenient error propagation to the user
- No `unwrap()` or `expect()` in library code. Allowed in tests and in CLI `main()` only.

**Dependencies:** Minimize. Every new dependency must justify its inclusion. Prefer stdlib and well-maintained crates with few transitive dependencies.

---

## License Headers

Every source file must start with a license header using [SPDX](https://spdx.dev/learn/handling-license-info/) format.

```rust
// Copyright (c) 2026 Joydev GmbH (joydev.com)
// SPDX-License-Identifier: MIT
```

The header goes on the first line of the file, before any `#![...]` attributes, imports, or code. One blank line separates the header from the rest of the file.

---

## Testing Strategy

**Test-Driven Development (TDD)** is the default workflow. Write the test first, watch it fail, implement the minimum to pass, refactor.

### Test Levels

**Unit tests** (Rust `#[cfg(test)]` modules):

- Every public function in jot-core has unit tests
- Data model serialization/deserialization roundtrips
- Recurrence rule logic

**Integration tests** (`tests/` directory):

- CLI command execution against real `.jot/` directories
- Full workflows: add, done, ls, edit, rm

**Snapshot tests** (for CLI output):

- CLI output is snapshot-tested with `insta`
- Both color and no-color variants

### Test Commands

```sh
just test              # Run all tests
just test-unit         # Rust unit tests only
just test-int          # Integration tests only
just test-snap         # Snapshot tests
just test-coverage     # With coverage report
```

### Coverage Target

Aim for >80% line coverage on jot-core. No hard enforcement -- coverage is a signal, not a goal.

---

## CI/CD

Every push and pull request triggers:

1. **Format check** -- `cargo fmt --check`
2. **Lint** -- `cargo clippy -- -D warnings`
3. **Test** -- Full test suite
4. **Build** -- Debug build

Releases are triggered by Git tags (`v0.1.0`, `v1.0.0`, etc.).

**Build matrix:**

| Target | OS | Arch |
|--------|----|------|
| CLI binary | Linux, macOS, Windows | x86_64, aarch64 |

**Artifacts:**

- Standalone binaries (tar.gz, zip)
- Homebrew formula
- Cargo install: `cargo install jot`

---

## Task Runner

Use `just` (justfile) as the project task runner.

---

## Commit Messages

Use conventional commits. Format: `type(scope): description`

Types: `feat`, `fix`, `refactor`, `test`, `docs`, `chore`, `ci`

Scopes: `core`, `cli`, `docs`

Examples:

```
feat(core): add RRULE recurrence support
fix(cli): handle completed recurring todos correctly
test(core): add roundtrip tests for todo serialization
```

No emoji in commit messages.

---

## Versioning

All crates in the workspace share the same version and are bumped together, following [Semantic Versioning](https://semver.org/).

Before 1.0 (0.x releases), breaking changes to commands and flags are permitted. After 1.0:

- **Patch** (1.0.0 to 1.0.1): bugfixes only, no CLI changes
- **Minor** (1.0.x to 1.1.0): new commands/flags allowed, deprecated commands show a warning with a hint to the replacement
- **Major** (1.x to 2.0): deprecated commands may be removed

---

## Backlog Workflow

The product backlog is managed with Joy (the `joy` CLI). Key conventions:

- **Item types:** epic, story, task, bug, rework, decision, idea
- **Priority levels:** critical, high, medium, low
- **ID scheme:** JOT-0001 to JOT-FFFF (items), JOT-MS-01 to JOT-MS-FF (milestones), hex-based
- **Language:** All titles, descriptions, and comments in English
- **Status tracking:** `joy start <ID>` before coding, `joy close <ID>` after committing -- never skip
- **Implementation flow:** Comment planned solution into the item, confirm, implement, update todos
- **Checklist items:** Use "todo" (not "task") to avoid confusion with the task item type

---

## Related

Jot is part of the [Joyint ecosystem](https://github.com/joyint/project). Shared architectural decisions (ADRs) and cross-project conventions are maintained there.
