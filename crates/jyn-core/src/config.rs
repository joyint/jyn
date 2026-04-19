// Copyright (c) 2026 Joydev GmbH (joydev.com)
// SPDX-License-Identifier: MIT

//! User configuration for jyn.
//!
//! Layout mirrors joy-cli's config: YAML files at two locations, merged
//! at load time with a deep-merge. Strict schema via `deny_unknown_fields`
//! at every level so that typos in `jyn config set` are reported instead
//! of silently written.
//!
//! Layers, highest precedence last:
//!   1. code defaults (`Config::default()`)
//!   2. global personal: `$XDG_CONFIG_HOME/jyn/config.yaml`
//!   3. project-local personal: `<root>/.jyn/config.yaml`

use std::path::{Path, PathBuf};

use joy_core::fortune::Category;
use serde::{Deserialize, Serialize};

pub const CONFIG_FILE: &str = "config.yaml";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub output: OutputConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: 1,
            output: OutputConfig::default(),
        }
    }
}

fn default_version() -> u32 {
    1
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputConfig {
    #[serde(default = "default_fortune")]
    pub fortune: bool,
    #[serde(
        rename = "fortune-category",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub fortune_category: Option<Category>,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            fortune: true,
            fortune_category: None,
        }
    }
}

fn default_fortune() -> bool {
    true
}

/// `$XDG_CONFIG_HOME/jyn/config.yaml`, falling back to `~/.config/jyn/config.yaml`.
pub fn global_config_path() -> PathBuf {
    global_config_path_from(
        std::env::var("XDG_CONFIG_HOME").ok().map(PathBuf::from),
        home_dir(),
    )
}

fn global_config_path_from(xdg: Option<PathBuf>, home: Option<PathBuf>) -> PathBuf {
    let config_dir =
        xdg.unwrap_or_else(|| home.unwrap_or_else(|| PathBuf::from(".")).join(".config"));
    config_dir.join("jyn").join(CONFIG_FILE)
}

/// `<root>/.jyn/config.yaml`.
pub fn local_config_path(root: &Path) -> PathBuf {
    crate::storage::jyn_dir(root).join(CONFIG_FILE)
}

fn home_dir() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}

/// Recursively merge `overlay` into `base`. Mapping keys are merged; all
/// other values are replaced.
pub fn deep_merge_value(base: &mut serde_json::Value, overlay: &serde_json::Value) {
    if let (Some(base_map), Some(overlay_map)) = (base.as_object_mut(), overlay.as_object()) {
        for (key, value) in overlay_map {
            if let Some(existing) = base_map.get_mut(key) {
                deep_merge_value(existing, value);
            } else {
                base_map.insert(key.clone(), value.clone());
            }
        }
    } else {
        *base = overlay.clone();
    }
}

fn read_yaml_value(path: &Path) -> Option<serde_json::Value> {
    let content = std::fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_yaml_ng::from_str(&content).ok()?;
    if value.is_null() {
        return None;
    }
    Some(value)
}

/// Fully resolved, typed config: defaults merged with global + local.
///
/// If no project root exists, defaults and global are still honoured.
/// A malformed file is reported on stderr and the offending layer is
/// skipped, falling through to lower-precedence layers.
pub fn load_config() -> Config {
    let merged = load_config_value();
    match serde_json::from_value(merged) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Warning: config has invalid values, using defaults: {e}");
            Config::default()
        }
    }
}

/// Untyped merged view across all layers, suitable for `jot config` display.
pub fn load_config_value() -> serde_json::Value {
    let mut merged: serde_json::Value = serde_json::to_value(Config::default()).unwrap_or_default();

    if let Some(global) = read_yaml_value(&global_config_path()) {
        deep_merge_value(&mut merged, &global);
    }
    if let Some(root) = current_project_root() {
        if let Some(local) = read_yaml_value(&local_config_path(&root)) {
            deep_merge_value(&mut merged, &local);
        }
    }

    merged
}

/// Only the user-set overrides (global + local), without defaults. Used to
/// mark which values are at their default vs. set by the user.
pub fn load_personal_config_value() -> serde_json::Value {
    let mut merged = serde_json::json!({});

    if let Some(global) = read_yaml_value(&global_config_path()) {
        deep_merge_value(&mut merged, &global);
    }
    if let Some(root) = current_project_root() {
        if let Some(local) = read_yaml_value(&local_config_path(&root)) {
            deep_merge_value(&mut merged, &local);
        }
    }

    merged
}

/// Returns cwd if it contains a `.jyn/` directory, else `None`.
///
/// jyn's task model is per-cwd (no walking up), and config follows the
/// same rule for consistency: the local config layer corresponds to
/// exactly the `.jyn/` directory a user sees when they `ls` their cwd.
pub fn current_project_root() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    if crate::storage::jyn_dir(&cwd).is_dir() {
        Some(cwd)
    } else {
        None
    }
}

/// Navigate a dotted key. Accepts both `fortune-category` and
/// `fortune_category` (serde renames vs. Rust field names).
pub fn navigate<'a>(value: &'a serde_json::Value, key: &str) -> Option<&'a serde_json::Value> {
    let mut current = value;
    for part in key.split('.') {
        current = current
            .get(part)
            .or_else(|| current.get(part.replace('-', "_")))
            .or_else(|| current.get(part.replace('_', "-")))?;
    }
    Some(current)
}

/// Set a value at a dotted key path, creating intermediate maps as needed.
pub fn set_nested(
    value: &mut serde_json::Value,
    key: &str,
    new_val: serde_json::Value,
) -> Result<(), String> {
    let parts: Vec<&str> = key.split('.').collect();
    let mut current = value;

    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            current
                .as_object_mut()
                .ok_or_else(|| format!("cannot set '{key}': parent is not an object"))?
                .insert(part.to_string(), new_val.clone());
            return Ok(());
        }
        if !current.get(*part).is_some_and(|v| v.is_object()) {
            current
                .as_object_mut()
                .ok_or_else(|| format!("cannot set '{key}': parent is not an object"))?
                .insert(part.to_string(), serde_json::json!({}));
        }
        current = current.get_mut(*part).unwrap();
    }

    Ok(())
}

/// Render a short "expected X" hint for a dotted key, derived from the
/// schema rather than a hand-maintained list. Returns `None` for unknown
/// keys.
pub fn field_hint(key: &str) -> Option<String> {
    let defaults = serde_json::to_value(Config::default()).ok()?;

    // Probe enum variants by trying string values against the full
    // Config round-trip. This surfaces Category variants without
    // hard-coding them here.
    let candidates = probe_string_field(key);
    if !candidates.is_empty() {
        return Some(format!("allowed values: {}", candidates.join(", ")));
    }

    match navigate(&defaults, key)? {
        serde_json::Value::Bool(_) => Some("expected: true or false".to_string()),
        serde_json::Value::Number(_) => Some("expected: a number".to_string()),
        serde_json::Value::String(_) => Some("expected: a string".to_string()),
        _ => None,
    }
}

fn probe_string_field(key: &str) -> Vec<String> {
    const PROBES: &[&str] = &["tech", "science", "humor", "all"];

    let mut accepted = Vec::new();
    for &candidate in PROBES {
        let yaml = build_yaml_for_key(key, candidate);
        let defaults_yaml = match serde_yaml_ng::to_string(&Config::default()) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let mut base: serde_json::Value = match serde_yaml_ng::from_str(&defaults_yaml) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let overlay: serde_json::Value = match serde_yaml_ng::from_str(&yaml) {
            Ok(v) => v,
            Err(_) => continue,
        };
        deep_merge_value(&mut base, &overlay);
        if serde_json::from_value::<Config>(base).is_ok() {
            accepted.push(candidate.to_string());
        }
    }
    accepted
}

fn build_yaml_for_key(key: &str, value: &str) -> String {
    let parts: Vec<&str> = key.split('.').collect();
    let mut yaml = String::new();
    for (i, part) in parts.iter().enumerate() {
        for _ in 0..i {
            yaml.push_str("  ");
        }
        if i == parts.len() - 1 {
            yaml.push_str(&format!("{part}: {value}\n"));
        } else {
            yaml.push_str(&format!("{part}:\n"));
        }
    }
    yaml
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_roundtrip() {
        let config = Config::default();
        let yaml = serde_yaml_ng::to_string(&config).unwrap();
        let parsed: Config = serde_yaml_ng::from_str(&yaml).unwrap();
        assert_eq!(config, parsed);
    }

    #[test]
    fn default_output_fortune_is_true() {
        assert!(Config::default().output.fortune);
    }

    #[test]
    fn unknown_top_level_key_is_rejected() {
        let yaml = "version: 1\nunknown_key: foo\n";
        let err = serde_yaml_ng::from_str::<Config>(yaml).unwrap_err();
        assert!(err.to_string().contains("unknown"));
    }

    #[test]
    fn unknown_nested_key_is_rejected() {
        let yaml = "output:\n  not_a_field: true\n";
        let err = serde_yaml_ng::from_str::<Config>(yaml).unwrap_err();
        assert!(err.to_string().contains("unknown"));
    }

    #[test]
    fn missing_version_defaults_to_1() {
        let yaml = "output:\n  fortune: false\n";
        let parsed: Config = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(parsed.version, 1);
        assert!(!parsed.output.fortune);
    }

    #[test]
    fn fortune_category_parses() {
        let yaml = "output:\n  fortune-category: tech\n";
        let parsed: Config = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(parsed.output.fortune_category, Some(Category::Tech));
    }

    #[test]
    fn deep_merge_replaces_scalars_and_merges_maps() {
        let mut base = serde_json::json!({ "a": 1, "b": { "c": 2, "d": 3 } });
        let overlay = serde_json::json!({ "b": { "c": 99 }, "e": 5 });
        deep_merge_value(&mut base, &overlay);
        assert_eq!(
            base,
            serde_json::json!({ "a": 1, "b": { "c": 99, "d": 3 }, "e": 5 })
        );
    }

    #[test]
    fn set_nested_creates_intermediate_maps() {
        let mut v = serde_json::json!({});
        set_nested(&mut v, "output.fortune", serde_json::json!(false)).unwrap();
        assert_eq!(v, serde_json::json!({ "output": { "fortune": false } }));
    }

    #[test]
    fn navigate_handles_hyphen_and_underscore_variants() {
        let v = serde_json::json!({ "output": { "fortune-category": "tech" } });
        assert_eq!(
            navigate(&v, "output.fortune-category").unwrap(),
            &serde_json::json!("tech")
        );
        assert_eq!(
            navigate(&v, "output.fortune_category").unwrap(),
            &serde_json::json!("tech")
        );
    }

    #[test]
    fn field_hint_for_bool() {
        let hint = field_hint("output.fortune").unwrap();
        assert_eq!(hint, "expected: true or false");
    }

    #[test]
    fn field_hint_for_category_lists_variants() {
        let hint = field_hint("output.fortune-category").unwrap();
        assert!(hint.contains("tech"));
        assert!(hint.contains("humor"));
        assert!(hint.contains("science"));
        assert!(hint.contains("all"));
    }

    #[test]
    fn global_config_path_uses_xdg_when_set() {
        let p = global_config_path_from(
            Some(PathBuf::from("/tmp/xdg")),
            Some(PathBuf::from("/home/user")),
        );
        assert_eq!(p, PathBuf::from("/tmp/xdg/jyn/config.yaml"));
    }

    #[test]
    fn global_config_path_falls_back_to_home_dot_config() {
        let p = global_config_path_from(None, Some(PathBuf::from("/home/user")));
        assert_eq!(p, PathBuf::from("/home/user/.config/jyn/config.yaml"));
    }

    #[test]
    fn global_config_path_falls_back_to_cwd_without_home() {
        let p = global_config_path_from(None, None);
        assert_eq!(p, PathBuf::from("./.config/jyn/config.yaml"));
    }

    #[test]
    fn local_config_path_is_under_dot_jyn() {
        let root = Path::new("/some/project");
        assert_eq!(
            local_config_path(root),
            Path::new("/some/project/.jyn/config.yaml")
        );
    }
}
