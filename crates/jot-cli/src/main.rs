// Copyright (c) 2026 Joydev GmbH (joydev.com)
// SPDX-License-Identifier: MIT

mod color;

use std::io::IsTerminal;

use anyhow::{Context, Result};
use clap::Parser;

use jot_core::display;
use jot_core::model::Task;
use jot_core::storage;

#[derive(Parser)]
#[command(name = "jot", version, about = "Personal task manager")]
struct Cli {
    /// Colorize output (auto by default: on when stdout is a TTY and NO_COLOR is unset)
    #[arg(long, value_enum, global = true, default_value_t = color::ColorChoice::Auto)]
    color: color::ColorChoice,

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
    /// Task title. Words are joined with spaces, so quoting is only needed
    /// when the shell would otherwise eat characters (e.g. `!`, `*`, `?`).
    #[arg(trailing_var_arg = true, num_args = 1..)]
    title: Vec<String>,
}

#[derive(clap::Args)]
struct RmArgs {
    /// Task ID (short `#A1` or full `TODO-00A1-EA`)
    id: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    color::init(cli.color);

    let root = std::env::current_dir().context("cannot read current directory")?;

    match cli.command {
        Some(Commands::Add(args)) => run_add(&root, &args.title.join(" "))?,
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

    let all = storage::load_tasks(root).unwrap_or_default();
    let full_ids: Vec<&str> = all.iter().map(|t| t.item.id.as_str()).collect();
    let labels = display::format_ids(&full_ids);
    let label = all
        .iter()
        .zip(labels.iter())
        .find_map(|(t, l)| (t.item.id == id).then_some(l.as_str()))
        .unwrap_or("");
    println!(
        "{} {}  {}",
        color::success("added"),
        color::id(label),
        title
    );
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

    let id_width = labels.iter().map(|s| s.len()).max().unwrap_or(2).max(2);

    let term_w = color::terminal_width();
    let id_header = "ID";
    let title_header = "TITLE";
    let fixed = id_width + 1;
    let min_title = 20;
    let title_col = (term_w.saturating_sub(fixed)).max(min_title);
    let frame_w = (fixed + title_col).min(term_w.max(1));

    println!(
        "{}",
        color::header(&[(id_header, id_width), (title_header, title_col)], frame_w)
    );

    for (task, label) in open.iter().zip(labels.iter()) {
        let id_cell = format!("{label:<id_width$}");
        println!("{} {}", color::id(&id_cell), task.item.title);
    }

    println!(
        "{}",
        color::footer(&color::plural(open.len(), "task"), frame_w)
    );
    Ok(())
}

fn run_rm(root: &std::path::Path, id: &str) -> Result<()> {
    let task = storage::delete_task(root, id).context("deleting task")?;
    let remaining = storage::load_tasks(root).unwrap_or_default();
    let mut all_ids: Vec<&str> = remaining.iter().map(|t| t.item.id.as_str()).collect();
    all_ids.push(task.item.id.as_str());
    let labels = display::format_ids(&all_ids);
    let label = labels.last().map(String::as_str).unwrap_or("");
    println!(
        "{} {}  {}",
        color::success("removed"),
        color::id(label),
        task.item.title
    );
    Ok(())
}
