// Copyright (c) 2026 Joydev GmbH (joydev.com)
// SPDX-License-Identifier: MIT

//! `jyn config` subcommand - read, inspect, and write the user config.
//!
//! Layout mirrors joy-cli's config command. Write location resolution is
//! jyn-specific: see [`resolve_write_target`].

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use jyn_core::config as cfg;

use crate::color;

#[derive(clap::Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    command: Option<ConfigCommand>,
}

#[derive(clap::Subcommand)]
enum ConfigCommand {
    /// Get a config value by dotted key (e.g. output.fortune).
    Get(GetArgs),
    /// Set a config value by dotted key (e.g. output.fortune false).
    Set(SetArgs),
}

#[derive(clap::Args)]
struct GetArgs {
    /// Dotted key path (e.g. output.fortune, output.fortune-category).
    key: String,
}

#[derive(clap::Args)]
struct SetArgs {
    /// Dotted key path.
    key: String,
    /// Value to set (string, number, or boolean).
    value: String,
    /// Write to the personal global config (~/.config/jyn/config.yaml).
    #[arg(long, conflicts_with = "local")]
    global: bool,
    /// Write to the project-local config (./.jyn/config.yaml), creating
    /// `.jyn/` if it does not exist yet.
    #[arg(long, conflicts_with = "global")]
    local: bool,
}

pub fn run(args: ConfigArgs) -> Result<()> {
    match args.command {
        None => show_all(),
        Some(ConfigCommand::Get(a)) => get_value(&a.key),
        Some(ConfigCommand::Set(a)) => set_value(&a.key, &a.value, a.global, a.local),
    }
}

// -- list --------------------------------------------------------------------

fn show_all() -> Result<()> {
    let value = cfg::load_config_value();
    let personal = cfg::load_personal_config_value();

    println!("{}", color::label("Configuration"));

    let obj = value.as_object().cloned().unwrap_or_default();
    let entries: Vec<_> = obj.iter().collect();

    for (i, (key, val)) in entries.iter().enumerate() {
        if i > 0 {
            println!();
        }
        if val.is_object() {
            println!("{}", color::label(key));
            print_object(val, &personal, &[key.as_str()], 2);
        } else {
            let is_default = !has_key(&personal, &[key.as_str()]);
            println!("{}", format_kv(key, val, 16, is_default));
        }
    }

    Ok(())
}

fn print_object(
    val: &serde_json::Value,
    personal: &serde_json::Value,
    path: &[&str],
    indent: usize,
) {
    let Some(obj) = val.as_object() else {
        return;
    };
    let pad = " ".repeat(indent);
    for (k, v) in obj {
        if v.is_object() {
            println!("{pad}{}", color::label(k));
            let mut next_path = path.to_vec();
            next_path.push(k.as_str());
            print_object(v, personal, &next_path, indent + 2);
        } else {
            let mut key_path = path.to_vec();
            key_path.push(k.as_str());
            let is_default = !has_key(personal, &key_path);
            println!("{pad}{}", format_kv(k, v, 20 - indent, is_default));
        }
    }
}

fn format_kv(key: &str, val: &serde_json::Value, width: usize, is_default: bool) -> String {
    let formatted = format_value(val);
    let suffix = if is_default {
        format!("  {}", color::inactive("[default]"))
    } else {
        String::new()
    };
    format!("{:<w$} {formatted}{suffix}", color::label(key), w = width)
}

fn format_value(val: &serde_json::Value) -> String {
    match val {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Null => color::inactive("null"),
        other => other.to_string(),
    }
}

fn has_key(value: &serde_json::Value, path: &[&str]) -> bool {
    let mut current = value;
    for part in path {
        match current.get(*part) {
            Some(v) => current = v,
            None => return false,
        }
    }
    true
}

// -- get ---------------------------------------------------------------------

fn get_value(key: &str) -> Result<()> {
    let value = cfg::load_config_value();
    let Some(result) = cfg::navigate(&value, key) else {
        // Silent exit 1 so scripts can branch on presence without parsing stderr.
        std::process::exit(1);
    };

    match result {
        serde_json::Value::String(s) => println!("{s}"),
        serde_json::Value::Bool(b) => println!("{b}"),
        serde_json::Value::Number(n) => println!("{n}"),
        serde_json::Value::Null => println!("null"),
        other => {
            let yaml = serde_yaml_ng::to_string(other)?;
            print!("{yaml}");
        }
    }
    Ok(())
}

// -- set ---------------------------------------------------------------------

enum WriteTarget {
    Local(PathBuf),
    Global(PathBuf),
}

impl WriteTarget {
    fn path(&self) -> &Path {
        match self {
            WriteTarget::Local(p) | WriteTarget::Global(p) => p,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            WriteTarget::Local(_) => "local",
            WriteTarget::Global(_) => "global",
        }
    }
}

/// Where `jyn config set` should write.
///
/// Precedence:
/// 1. Explicit `--global` or `--local` flag.
/// 2. `.jyn/` in cwd -> local.
/// 3. `~/.config/jyn/config.yaml` already exists -> global.
/// 4. Neither exists -> fail with an explicit, actionable error.
fn resolve_write_target(global_flag: bool, local_flag: bool) -> Result<WriteTarget> {
    if global_flag {
        return Ok(WriteTarget::Global(cfg::global_config_path()));
    }
    if local_flag {
        let cwd = std::env::current_dir().context("cannot read current directory")?;
        return Ok(WriteTarget::Local(cfg::local_config_path(&cwd)));
    }

    // No flag: auto-pick.
    if let Some(root) = cfg::current_project_root() {
        return Ok(WriteTarget::Local(cfg::local_config_path(&root)));
    }
    let global = cfg::global_config_path();
    if global.is_file() {
        return Ok(WriteTarget::Global(global));
    }

    bail!(
        "No config exists yet. Choose a location:\n  \
         jyn config set --global <key> <value>   (writes to ~/.config/jyn/config.yaml)\n  \
         jyn config set --local  <key> <value>   (writes to ./.jyn/config.yaml)"
    )
}

fn set_value(key: &str, raw_value: &str, global_flag: bool, local_flag: bool) -> Result<()> {
    let target = resolve_write_target(global_flag, local_flag)?;
    let path = target.path().to_path_buf();

    // Read existing file or start empty.
    let mut value: serde_json::Value = if path.is_file() {
        let content =
            fs::read_to_string(&path).with_context(|| format!("cannot read {}", path.display()))?;
        let parsed: serde_json::Value = serde_yaml_ng::from_str(&content)?;
        if parsed.is_null() {
            serde_json::json!({})
        } else {
            parsed
        }
    } else {
        serde_json::json!({})
    };

    let parsed = parse_value(raw_value);
    cfg::set_nested(&mut value, key, parsed).map_err(anyhow::Error::msg)?;

    // Validate via YAML round-trip through the typed Config so that
    // `deny_unknown_fields` catches typos, and scalar-type mismatches are
    // reported with a schema-derived hint.
    let yaml = serde_yaml_ng::to_string(&value)?;
    let defaults_yaml = serde_yaml_ng::to_string(&cfg::Config::default())?;
    let mut merged: serde_json::Value = serde_yaml_ng::from_str(&defaults_yaml)?;
    let overlay: serde_json::Value = serde_yaml_ng::from_str(&yaml)?;
    cfg::deep_merge_value(&mut merged, &overlay);
    if serde_json::from_value::<cfg::Config>(merged).is_err() {
        if let Some(hint) = cfg::field_hint(key) {
            bail!("'{raw_value}' is not valid for '{key}'\n  {hint}");
        }
        bail!("'{key}' is not a known config key or '{raw_value}' has the wrong type");
    }

    // Ensure parent dir exists (auto-create `.jyn/` or `~/.config/jyn/` on demand).
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("cannot create {}", parent.display()))?;
    }
    fs::write(&path, yaml).with_context(|| format!("cannot write {}", path.display()))?;

    println!("{key} = {raw_value}  [{}]", target.label());
    Ok(())
}

fn parse_value(raw: &str) -> serde_json::Value {
    match raw {
        "true" => serde_json::Value::Bool(true),
        "false" => serde_json::Value::Bool(false),
        "null" | "none" => serde_json::Value::Null,
        _ => {
            if let Ok(n) = raw.parse::<i64>() {
                serde_json::Value::Number(n.into())
            } else if let Ok(f) = raw.parse::<f64>() {
                serde_json::Number::from_f64(f)
                    .map(serde_json::Value::Number)
                    .unwrap_or_else(|| serde_json::Value::String(raw.to_string()))
            } else {
                serde_json::Value::String(raw.to_string())
            }
        }
    }
}
