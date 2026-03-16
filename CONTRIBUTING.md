# Contributing

This document covers the coding conventions, testing strategy, CI/CD pipeline, and commit message format for the Jot repository.

For product vision and CLI design see [docs/dev/Vision.md](docs/dev/Vision.md). For technology choices and architecture see [docs/dev/Architecture.md](docs/dev/Architecture.md). For cross-project architecture and ADRs see the [umbrella repository](https://github.com/joyint/project).

---

## Documentation Rules

**No emoji in technical documentation.** Emoji are a runtime feature of the CLI (configurable, deactivatable). They do not belong in technical docs (vision, architecture, ADRs, code comments) or commit messages. README.md and user-facing materials may use emoji sparingly for warmth.

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

**Test-Driven Development (TDD)** is the default workflow.

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

Aim for >80% line coverage on jot-core. No hard enforcement.

---

## CI/CD

Every push and pull request triggers:

1. **Format check** -- `cargo fmt --check`
2. **Lint** -- `cargo clippy -- -D warnings`
3. **Test** -- Full test suite
4. **Build** -- Debug build

Releases are triggered by Git tags.

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
