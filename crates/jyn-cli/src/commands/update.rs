// Copyright (c) 2026 Joydev GmbH (joydev.com)
// SPDX-License-Identifier: MIT

//! `jyn update` -- swap the binary in place, downloading a newer release when
//! one is available. Mirrors the binary self-update half of `joy update`
//! (JYN-0004-E6).
//!
//! Binaries installed via the cargo-dist installer carry an axoupdater
//! receipt. A `just install` or otherwise manually placed build in the install
//! dir (e.g. `~/.local/bin`) has no receipt, so we point the updater at the
//! release source and the running binary's directory and swap in place anyway
//! (overwriting an older local build). Only winget- and cargo-managed binaries
//! are left to their own package manager, with a hint.

use anyhow::Result;
use axoupdater::{AxoUpdater, AxoupdateError, ReleaseSource, ReleaseSourceType, Version};
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
    if let Some((manager, cmd)) = package_manager_hint() {
        // winget / cargo manage the binary: never touch it, just point the
        // user at the right upgrade command.
        println!(
            "  {} {:<8} {}",
            color::inactive("-"),
            "binary",
            color::inactive(&format!("managed by {manager} ({CURRENT_VERSION})"))
        );
        println!("             upgrade with: {cmd}");
    } else {
        let (mark, detail) = update_in_place();
        println!("  {mark} {:<8} {detail}", "binary");
    }
    Ok(())
}

/// winget- and cargo-managed binaries are upgraded by their own package
/// manager; infer which one from the running binary's path. `None` means jyn
/// manages the binary itself (the cargo-dist install dir, e.g. `~/.local/bin`),
/// so we update it in place.
fn package_manager_hint() -> Option<(&'static str, String)> {
    let path = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if path.contains("microsoft\\winget") || path.contains("microsoft/winget") {
        Some(("winget", "winget upgrade -s winget joyint.jyn".to_string()))
    } else if path.contains("/.cargo/") || path.contains("\\.cargo\\") {
        Some(("cargo", "cargo install jyn-cli".to_string()))
    } else {
        None
    }
}

/// An updater ready to run. Uses the cargo-dist install receipt when present;
/// otherwise points at the GitHub release source and the running binary's
/// directory, so a receipt-less build (e.g. `just install` into `~/.local/bin`)
/// still checks-and-swaps, overwriting an older local build.
fn configured_updater() -> AxoUpdater {
    let mut updater = AxoUpdater::new_for(PKG_NAME);
    if updater.load_receipt().is_err() {
        updater.set_release_source(ReleaseSource {
            release_type: ReleaseSourceType::GitHub,
            owner: "joyint".to_string(),
            // repo is joyint/jyn, but cargo-dist publishes artifacts under the
            // crate/app name jyn-cli (jyn-cli-installer.sh, jyn-cli-*.tar.xz).
            name: "jyn".to_string(),
            app_name: "jyn-cli".to_string(),
        });
        if let Ok(version) = CURRENT_VERSION.parse::<Version>() {
            let _ = updater.set_current_version(version);
        }
        if let Some(dir) = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|d| d.to_string_lossy().into_owned()))
        {
            updater.set_install_dir(dir);
        }
    }
    updater
}

/// Download-and-swap when a newer release exists; a no-op when up to date.
fn update_in_place() -> (String, String) {
    let mut updater = configured_updater();
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
        // No newer (or no) release to install is not a failure: we already
        // have the latest. Only genuine problems (network, download, install)
        // are reported as failures, and never with axoupdater's raw wording.
        Err(
            AxoupdateError::NoStableReleases { .. }
            | AxoupdateError::ReleaseNotFound { .. }
            | AxoupdateError::VersionNotFound { .. },
        ) => (
            color::success("ok"),
            color::inactive(&format!("up to date ({CURRENT_VERSION})")),
        ),
        Err(e) => (
            color::warning("!"),
            color::warning(&format!("update failed: {e}")),
        ),
    }
}

/// Read-only audit: report whether a binary update is available without
/// touching anything. Exits with code 2 when an update is pending so
/// scripts and CI can detect staleness.
fn run_check() -> Result<()> {
    println!("{}", color::label("jyn update check"));

    if let Some((manager, cmd)) = package_manager_hint() {
        println!(
            "  {} {:<8} {}",
            color::inactive("-"),
            "binary",
            color::inactive(&format!("managed by {manager} ({CURRENT_VERSION})"))
        );
        println!("             upgrade with: {cmd}");
        return Ok(());
    }

    let mut updater = configured_updater();
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
