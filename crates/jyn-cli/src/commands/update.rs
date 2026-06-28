// Copyright (c) 2026 Joydev GmbH (joydev.com)
// SPDX-License-Identifier: MIT

//! `jyn update` -- swap the binary when this build was distributed via
//! cargo-dist. Mirrors the binary self-update half of `joy update`
//! (JYN-0004-E6).
//!
//! Unlike Joy, jyn keeps no managed in-repo state (no embedded files,
//! git hooks, auth artefacts, or AI tool files), so there is no in-repo
//! sync step: this command is purely the binary swap.
//!
//! The swap is receipt-gated by `axoupdater`: only cargo-dist
//! installer-managed binaries carry the install receipt, so brew /
//! cargo-install / distro-package builds skip the swap with a clear
//! message instead of clobbering a foreign-managed binary.

use anyhow::Result;
use axoupdater::AxoUpdater;
use clap::Args;

use crate::color;

/// Cargo-dist installer writes the receipt under the crate package name
/// (`jyn-cli`), not the binary name `jyn`.
const PKG_NAME: &str = "jyn-cli";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Args)]
pub struct UpdateArgs {
    /// Read-only check; exit 2 if an update is available.
    #[arg(long)]
    pub check: bool,
}

pub fn run(args: UpdateArgs) -> Result<()> {
    if args.check {
        return run_check();
    }

    println!("{}", color::label("jyn update"));
    if let Some((manager, cmd)) = foreign_install() {
        // Foreign-managed binary (e.g. winget): never touch it, point the user
        // at the right command instead. jyn update is binary-only, so there is
        // nothing else to do once the upgrade runs.
        println!(
            "  {} {:<8} {}",
            color::inactive("-"),
            "binary",
            color::inactive(&format!("managed by {manager} ({CURRENT_VERSION})"))
        );
        println!("             upgrade with: {cmd}");
    } else {
        let (mark, detail) = swap_binary();
        println!("  {mark} {:<8} {detail}", "binary");
    }
    Ok(())
}

/// When the running binary has no axoupdater receipt it was installed by a
/// foreign package manager, so `jyn update` must not touch it. Infer that
/// manager from the binary's own path and return `(display name, upgrade
/// command)` for an actionable hint. `None` when a receipt is present.
/// jyn never runs the command itself (a failing foreign upgrade must not
/// entangle jyn).
fn foreign_install() -> Option<(&'static str, String)> {
    let mut updater = AxoUpdater::new_for(PKG_NAME);
    if updater.load_receipt().is_ok() {
        return None;
    }
    let path = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let info = if path.contains("microsoft\\winget") || path.contains("microsoft/winget") {
        ("winget", "winget upgrade -s winget joyint.jyn".to_string())
    } else if path.contains("/.cargo/") || path.contains("\\.cargo\\") {
        ("cargo", "cargo install jyn-cli".to_string())
    } else {
        // Unknown manager; winget is by far the most common foreign install.
        (
            "another installer",
            "winget upgrade -s winget joyint.jyn".to_string(),
        )
    };
    Some(info)
}

/// Run the receipt-gated binary self-update and return a status mark plus
/// a human-readable detail string.
fn swap_binary() -> (String, String) {
    let mut updater = AxoUpdater::new_for(PKG_NAME);
    if updater.load_receipt().is_err() {
        return (
            color::inactive("-"),
            color::inactive(&format!("managed by another installer ({CURRENT_VERSION})")),
        );
    }
    match updater.run_sync() {
        Ok(Some(result)) => {
            let old = result
                .old_version
                .as_ref()
                .map(|v| v.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            let new = result.new_version.to_string();
            (
                color::success("ok"),
                color::success(&format!("updated {old} -> {new}")),
            )
        }
        Ok(None) => (
            color::success("ok"),
            color::inactive(&format!("up to date ({CURRENT_VERSION})")),
        ),
        Err(e) => (color::warning("!"), color::warning(&format!("failed: {e}"))),
    }
}

/// Read-only audit: report whether a binary update is available without
/// touching anything. Exits with code 2 when an update is pending so
/// scripts and CI can detect staleness.
fn run_check() -> Result<()> {
    println!("{}", color::label("jyn update check"));

    let mut updater = AxoUpdater::new_for(PKG_NAME);
    if let Some((manager, cmd)) = foreign_install() {
        println!(
            "  {} {:<8} {}",
            color::inactive("-"),
            "binary",
            color::inactive(&format!("managed by {manager} ({CURRENT_VERSION})"))
        );
        println!("             upgrade with: {cmd}");
        return Ok(());
    }

    if updater.is_update_needed_sync().unwrap_or(false) {
        println!(
            "  {} {:<8} {}",
            color::warning("!"),
            "binary",
            color::warning(&format!("update available (current {CURRENT_VERSION})"))
        );
        std::process::exit(2);
    }

    println!(
        "  {} {:<8} {}",
        color::success("ok"),
        "binary",
        color::inactive(&format!("up to date ({CURRENT_VERSION})"))
    );
    Ok(())
}
