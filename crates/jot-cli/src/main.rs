// Copyright (c) 2026 Joydev GmbH (joydev.com)
// SPDX-License-Identifier: MIT

use std::io::IsTerminal;

use anyhow::{Context, Result};
use clap::Parser;

use jot_core::display;
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
    /// Task ID (short `#A1` or full `TODO-00A1-EA`)
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

    let all: Vec<Task> = storage::load_tasks(root).unwrap_or_default();
    let full_ids: Vec<&str> = all.iter().map(|t| t.item.id.as_str()).collect();
    let labels = display::format_ids(&full_ids);
    let label = all
        .iter()
        .zip(labels.iter())
        .find_map(|(t, l)| (t.item.id == id).then_some(l.as_str()))
        .unwrap_or("");
    println!("{label}  {title}");
    Ok(())
}

fn run_ls(root: &std::path::Path) -> Result<()> {
    let tasks = storage::load_tasks(root).context("loading tasks")?;
    let open: Vec<&Task> = tasks.iter().filter(|t| t.item.is_active()).collect();
    if open.is_empty() {
        println!("No open tasks. Add one with: jot add \"<title>\"");
        return Ok(());
    }
    let full_ids: Vec<&str> = open.iter().map(|t| t.item.id.as_str()).collect();
    let labels = display::format_ids(&full_ids);
    let width = labels.iter().map(|s| s.len()).max().unwrap_or(0);
    for (task, label) in open.iter().zip(labels.iter()) {
        println!("{label:<width$}  {}", task.item.title);
    }
    Ok(())
}

fn run_rm(root: &std::path::Path, id: &str) -> Result<()> {
    let task = storage::delete_task(root, id).context("deleting task")?;
    let remaining: Vec<Task> = storage::load_tasks(root).unwrap_or_default();
    let mut all_ids: Vec<&str> = remaining.iter().map(|t| t.item.id.as_str()).collect();
    all_ids.push(task.item.id.as_str());
    let labels = display::format_ids(&all_ids);
    let label = labels.last().map(String::as_str).unwrap_or("");
    println!("Removed {label}  {}", task.item.title);
    Ok(())
}
