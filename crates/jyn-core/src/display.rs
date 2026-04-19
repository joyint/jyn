// Copyright (c) 2026 Joydev GmbH (joydev.com)
// SPDX-License-Identifier: MIT

//! Short-form ID rendering and input parsing per JOT-002F-4D.
//!
//! Full IDs follow joy-core's ADR-027 scheme: `TODO-XXXX-YY`. For display
//! the tool shows only the middle counter without the acronym prefix,
//! leading zeros, or the title-hash suffix: `TODO-00A1-EA` -> `#A1`.
//! When two tasks in the same workspace share a counter (rare, caused by
//! concurrent adds on different devices before sync), the affected rows
//! keep the suffix so they remain addressable: `#A1-EA`, `#A1-7F`.

use std::collections::HashMap;

use crate::storage::ACRONYM;

/// Split a full ID `TODO-00A1-EA` into its counter (`00A1`) and optional
/// title-hash suffix (`EA`). Returns `(counter, suffix)`.
fn split_full_id(full: &str) -> (&str, Option<&str>) {
    let without_prefix = full
        .strip_prefix(&format!("{ACRONYM}-"))
        .or_else(|| full.strip_prefix(&format!("{}-", ACRONYM.to_lowercase())))
        .unwrap_or(full);
    match without_prefix.split_once('-') {
        Some((counter, suffix)) => (counter, Some(suffix)),
        None => (without_prefix, None),
    }
}

/// Strip leading zeros from a hex counter, keeping at least one digit.
fn strip_leading_zeros(hex: &str) -> String {
    let trimmed = hex.trim_start_matches('0');
    if trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Render a short display ID for a single full ID, without disambiguation.
/// Use `format_ids` when rendering a list where collisions must be handled.
pub fn short_id(full: &str) -> String {
    let (counter, _) = split_full_id(full);
    format!("#{}", strip_leading_zeros(counter))
}

/// Render short display IDs for a list of full IDs, expanding only the
/// rows whose counters collide within the list. Returns one string per
/// input, in the same order.
pub fn format_ids(full_ids: &[&str]) -> Vec<String> {
    let mut counter_frequency: HashMap<String, usize> = HashMap::new();
    let parsed: Vec<(String, Option<String>)> = full_ids
        .iter()
        .map(|id| {
            let (c, s) = split_full_id(id);
            (c.to_uppercase(), s.map(|s| s.to_uppercase()))
        })
        .collect();

    for (counter, _) in &parsed {
        *counter_frequency.entry(counter.clone()).or_insert(0) += 1;
    }

    parsed
        .into_iter()
        .map(|(counter, suffix)| {
            let short_counter = strip_leading_zeros(&counter);
            let ambiguous = counter_frequency.get(&counter).copied().unwrap_or(1) > 1;
            match (ambiguous, suffix) {
                (true, Some(sfx)) => format!("#{short_counter}-{sfx}"),
                _ => format!("#{short_counter}"),
            }
        })
        .collect()
}

/// Normalize any of `#A1`, `A1`, `a1`, `TODO-00A1`, `TODO-00A1-EA`,
/// `#A1-EA` to a form `find_task_file` understands (uppercase, with
/// `TODO-` prefix and 4-digit counter).
///
/// Pass-through for anything that does not look like a short or full ID;
/// callers get the original string back and downstream lookup fails with
/// the normal "not found" diagnostic.
pub fn normalize_id_input(raw: &str) -> String {
    let trimmed = raw.trim().trim_start_matches('#');

    if trimmed.to_uppercase().starts_with(&format!("{ACRONYM}-")) {
        return trimmed.to_uppercase();
    }

    let (counter, suffix) = match trimmed.split_once('-') {
        Some((c, s)) => (c, Some(s)),
        None => (trimmed, None),
    };

    let counter_valid =
        !counter.is_empty() && counter.len() <= 4 && counter.chars().all(|c| c.is_ascii_hexdigit());
    if !counter_valid {
        return raw.to_string();
    }

    let padded = format!("{:0>4}", counter.to_uppercase());
    match suffix {
        Some(sfx) if sfx.len() == 2 && sfx.chars().all(|c| c.is_ascii_hexdigit()) => {
            format!("{ACRONYM}-{padded}-{}", sfx.to_uppercase())
        }
        _ => format!("{ACRONYM}-{padded}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_id_strips_prefix_suffix_and_leading_zeros() {
        assert_eq!(short_id("TODO-00A1-EA"), "#A1");
        assert_eq!(short_id("TODO-0001-7F"), "#1");
        assert_eq!(short_id("TODO-0110-B3"), "#110");
        assert_eq!(short_id("TODO-FFFF-00"), "#FFFF");
    }

    #[test]
    fn short_id_without_suffix_still_works() {
        assert_eq!(short_id("TODO-0042"), "#42");
    }

    #[test]
    fn format_ids_unique_counters_stay_short() {
        let ids = vec!["TODO-0001-7F", "TODO-00A1-EA", "TODO-0110-B3"];
        let out = format_ids(&ids);
        assert_eq!(out, vec!["#1", "#A1", "#110"]);
    }

    #[test]
    fn format_ids_expands_only_colliding_rows() {
        let ids = vec![
            "TODO-0001-7F",
            "TODO-00A1-EA",
            "TODO-00A1-7F",
            "TODO-0110-B3",
        ];
        let out = format_ids(&ids);
        assert_eq!(out, vec!["#1", "#A1-EA", "#A1-7F", "#110"]);
    }

    #[test]
    fn normalize_accepts_short_form() {
        assert_eq!(normalize_id_input("#A1"), "TODO-00A1");
        assert_eq!(normalize_id_input("A1"), "TODO-00A1");
        assert_eq!(normalize_id_input("a1"), "TODO-00A1");
        assert_eq!(normalize_id_input("1"), "TODO-0001");
        assert_eq!(normalize_id_input("110"), "TODO-0110");
        assert_eq!(normalize_id_input("FFFF"), "TODO-FFFF");
    }

    #[test]
    fn normalize_accepts_short_form_with_suffix() {
        assert_eq!(normalize_id_input("#A1-EA"), "TODO-00A1-EA");
        assert_eq!(normalize_id_input("a1-ea"), "TODO-00A1-EA");
    }

    #[test]
    fn normalize_passes_through_full_form() {
        assert_eq!(normalize_id_input("TODO-00A1"), "TODO-00A1");
        assert_eq!(normalize_id_input("todo-00a1-ea"), "TODO-00A1-EA");
    }

    #[test]
    fn normalize_passes_through_nonsense() {
        // Out-of-range hex / non-hex stays untouched; find_task_file will
        // return a clean "not found" diagnostic.
        assert_eq!(normalize_id_input("GGGGG"), "GGGGG");
        assert_eq!(normalize_id_input("12345"), "12345");
    }
}
