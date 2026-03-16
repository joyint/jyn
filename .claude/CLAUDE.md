# CLAUDE.md

## Project

Jot is a personal todo CLI sharing [joy-core](https://github.com/joyint/joy) as its foundation. It provides fast, Git-native task management with recurring schedules, due dates, and reminders.

This repository (`joyint/jot`) contains:

| Crate | Purpose | License |
|-------|---------|---------|
| `jot-core` | Todo model: extends joy-core::Item with recurrence (RRULE) | MIT |
| `jot-cli` | Personal todo CLI binary (clap) | MIT |

## Required Reading

Before making any changes, read and follow the rules in these documents:

- `CONTRIBUTING.md` -- coding conventions, testing, CI/CD, commit messages
- `docs/dev/Vision.md` -- product vision, simplified model, CLI design, dispatch
- `docs/dev/Architecture.md` -- tech stack, repo structure, Cargo workspace

For cross-project architecture, ADRs, and business docs see the [umbrella repository](https://github.com/joyint/project).

## Rules

- Do not reference Claude, Anthropic, or AI assistants in code comments, git commits, documentation, or any generated content. No exceptions.
- No emoji in documentation, commit messages, or code comments
- Use Mermaid for all diagrams, never ASCII art
- Fix root causes, not symptoms -- no workarounds or temporary feature flags
- No `unwrap()` or `expect()` in library code (jot-core)
- Run `cargo fmt --all` and `cargo clippy --workspace -- -D warnings` before committing
