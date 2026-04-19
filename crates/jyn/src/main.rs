// Copyright (c) 2026 Joydev GmbH (joydev.com)
// SPDX-License-Identifier: MIT

// Jyn - Personal task manager
//
// Thin alias crate that exists so `cargo install jyn` installs the `jyn`
// binary directly. All logic lives in jyn-cli.

fn main() -> anyhow::Result<()> {
    jyn_cli::run()
}
