// Copyright (c) 2026 Joydev GmbH (joydev.com)
// SPDX-License-Identifier: MIT

//! Minimal color + layout helpers for the jot CLI.
//!
//! Mirrors the shape of `joy-cli::color` (semantic ANSI constants, dim
//! `label()` for structural chrome, colored `id()` for identifiers,
//! `header`/`footer` for the table frame) but carries only what jot
//! currently needs. NO_COLOR and non-TTY auto-disable; `--color` on
//! the root command overrides.

use std::io::IsTerminal;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum ColorChoice {
    Auto,
    Always,
    Never,
}

static ENABLED: OnceLock<bool> = OnceLock::new();

pub fn init(choice: ColorChoice) {
    let enabled = match choice {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => {
            if std::env::var_os("NO_COLOR").is_some() {
                false
            } else {
                std::io::stdout().is_terminal()
            }
        }
    };
    let _ = ENABLED.set(enabled);
}

fn is_enabled() -> bool {
    *ENABLED.get_or_init(|| {
        if std::env::var_os("NO_COLOR").is_some() {
            return false;
        }
        std::io::stdout().is_terminal()
    })
}

// Semantic ANSI constants (same palette as joy-cli for family consistency).
const RESET: &str = "\x1b[0m";
const SECONDARY: &str = "\x1b[32m"; // ID, labels, chrome
const SUCCESS: &str = "\x1b[38;5;10m"; // check marks, done messages
const DANGER: &str = "\x1b[31m"; // errors, removal

fn wrap(code: &str, text: &str) -> String {
    if is_enabled() {
        format!("{code}{text}{RESET}")
    } else {
        text.to_string()
    }
}

fn wrap2(code1: &str, code2: &str, text: &str) -> String {
    if is_enabled() {
        format!("{code1}{code2}{text}{RESET}")
    } else {
        text.to_string()
    }
}

const BOLD: &str = "\x1b[1m";
const INACTIVE: &str = "\x1b[38;5;8m";

/// Inactive (dim grey) -- for low priority and other de-emphasised bits.
pub fn inactive(text: &str) -> String {
    wrap(INACTIVE, text)
}

/// Bold danger red -- critical and extreme priority.
pub fn danger_bold(text: &str) -> String {
    wrap2(BOLD, DANGER, text)
}

/// Secondary color -- used for IDs and structural chrome (header,
/// separators, footer).
pub fn label(text: &str) -> String {
    wrap(SECONDARY, text)
}

/// Secondary color on an ID.
pub fn id(text: &str) -> String {
    wrap(SECONDARY, text)
}

/// Success green -- for "added", "removed" status words.
pub fn success(text: &str) -> String {
    wrap(SUCCESS, text)
}

/// Danger red -- reserved for error contexts and overdue due dates.
pub fn danger(text: &str) -> String {
    wrap(DANGER, text)
}

/// Warning yellow -- for due-today highlights.
pub fn warning(text: &str) -> String {
    wrap("\x1b[33m", text)
}

/// Info cyan -- for tag chips.
pub fn info(text: &str) -> String {
    wrap("\x1b[36m", text)
}

/// Detect terminal width, falling back to 80 columns when the stream
/// is not a TTY or the OS does not report a size.
pub fn terminal_width() -> usize {
    if let Some((terminal_size::Width(w), _)) = terminal_size::terminal_size() {
        return w as usize;
    }
    std::env::var("COLUMNS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(80)
}

/// Separator line spanning `width` columns, secondary-colored.
pub fn separator(width: usize) -> String {
    label(&"-".repeat(width))
}

/// Top frame: separator, header row, separator -- matches joy's table shape.
pub fn header(columns: &[(&str, usize)], width: usize) -> String {
    let sep = separator(width);
    let row = columns
        .iter()
        .map(|(name, w)| {
            let padded = format!("{name:<w$}", w = *w);
            label(&padded)
        })
        .collect::<Vec<_>>()
        .join(" ");
    format!("{sep}\n{row}\n{sep}")
}

/// Bottom frame: separator, summary line.
pub fn footer(message: &str, width: usize) -> String {
    format!("{}\n{}", separator(width), label(message))
}

/// Pluralize: "1 task" / "3 tasks".
pub fn plural(count: usize, singular: &str) -> String {
    if count == 1 {
        format!("{count} {singular}")
    } else {
        format!("{count} {singular}s")
    }
}
