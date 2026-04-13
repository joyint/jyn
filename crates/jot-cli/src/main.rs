// Copyright (c) 2026 Joydev GmbH (joydev.com)
// SPDX-License-Identifier: MIT

use std::io::IsTerminal;

use anyhow::{Context, Result};
use clap::Parser;

use jot_core::model::Task;
use jot_core::storage;

#[derive(Parser)]
#[command(name = "jot", version, about = "Personal task manager")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Add a new task
    Add(AddArgs),
    /// List open tasks
    Ls,
    /// Remove a task
    Rm(RmArgs),
}

#[derive(clap::Args)]
struct AddArgs {
    /// Task title
    title: String,
}

#[derive(clap::Args)]
struct RmArgs {
    /// Task ID (full or short form)
    id: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = std::env::current_dir().context("cannot read current directory")?;

    match cli.command {
        Some(Commands::Add(args)) => run_add(&root, &args.title)?,
        Some(Commands::Ls) | None => run_ls(&root)?,
        Some(Commands::Rm(args)) => run_rm(&root, &args.id)?,
    }

    if std::io::stdout().is_terminal() {
        if let Some(text) = joy_core::fortune::fortune(None, 0.2) {
            eprintln!("\n\x1b[2m{text}\x1b[0m");
        }
    }

    Ok(())
}

fn run_add(root: &std::path::Path, title: &str) -> Result<()> {
    let id = storage::next_id(root, title).context("generating next ID")?;
    let task = Task::new(id.clone(), title.to_string());
    storage::save_task(root, &task).context("saving task")?;
    println!("{id}  {title}");
    Ok(())
}

fn run_ls(root: &std::path::Path) -> Result<()> {
    let tasks = storage::load_tasks(root).context("loading tasks")?;
    let open: Vec<&Task> = tasks.iter().filter(|t| t.item.is_active()).collect();
    if open.is_empty() {
        println!("No open tasks. Add one with: jot add \"<title>\"");
        return Ok(());
    }
    for task in open {
        println!("{}  {}", task.item.id, task.item.title);
    }
    Ok(())
}

fn run_rm(root: &std::path::Path, id: &str) -> Result<()> {
    let task = storage::delete_task(root, id).context("deleting task")?;
    println!("Removed {}  {}", task.item.id, task.item.title);
    Ok(())
}
