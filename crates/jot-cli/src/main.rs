// Copyright (c) 2026 Joydev GmbH (joydev.com)
// SPDX-License-Identifier: MIT

mod color;

use std::io::IsTerminal;
use std::path::Path;

use anyhow::{Context, Result};
use chrono::Local;
use clap::Parser;
use joy_core::model::item::Priority;

use jot_core::display;
use jot_core::due::{self, DueSeverity};
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
    Ls(LsArgs),
    /// Remove a task
    Rm(RmArgs),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
enum PriorityArg {
    Low,
    Medium,
    High,
    Critical,
    Extreme,
}

impl PriorityArg {
    fn into_core(self) -> Priority {
        match self {
            PriorityArg::Low => Priority::Low,
            PriorityArg::Medium => Priority::Medium,
            PriorityArg::High => Priority::High,
            PriorityArg::Critical => Priority::Critical,
            PriorityArg::Extreme => Priority::Extreme,
        }
    }
}

#[derive(clap::Args)]
struct AddArgs {
    /// Due date: `today`, `tomorrow`, or `YYYY-MM-DD`.
    #[arg(short, long)]
    due: Option<String>,

    /// Priority: low, medium (default), high, critical, extreme.
    #[arg(short, long, value_enum)]
    priority: Option<PriorityArg>,

    /// Tag. Repeat to attach multiple tags: `--tag work --tag home`.
    #[arg(short, long = "tag")]
    tags: Vec<String>,

    /// Task title. Words are joined with spaces, so quoting is only needed
    /// when the shell would otherwise eat characters (e.g. `!`, `*`, `?`).
    #[arg(trailing_var_arg = true, num_args = 1..)]
    title: Vec<String>,
}

#[derive(clap::Args, Default)]
struct LsArgs {
    /// Include closed tasks as well.
    #[arg(short, long)]
    all: bool,

    /// Filter by due date (`today`, `tomorrow`, `YYYY-MM-DD`). Includes
    /// overdue tasks when `today` is used.
    #[arg(short, long)]
    due: Option<String>,

    /// Filter by tag. Repeat to require multiple tags.
    #[arg(short, long = "tag")]
    tags: Vec<String>,
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
        Some(Commands::Add(args)) => run_add(&root, args)?,
        Some(Commands::Ls(args)) => run_ls(&root, &args)?,
        None => run_ls(&root, &LsArgs::default())?,
        Some(Commands::Rm(args)) => run_rm(&root, &args.id)?,
    }

    if std::io::stdout().is_terminal() {
        if let Some(text) = joy_core::fortune::fortune(None, 0.2) {
            eprintln!("\n\x1b[2m{text}\x1b[0m");
        }
    }

    Ok(())
}

fn run_add(root: &Path, args: AddArgs) -> Result<()> {
    let title = args.title.join(" ");
    if title.trim().is_empty() {
        anyhow::bail!("title must not be empty");
    }

    let today = Local::now().date_naive();
    let due_date = match args.due.as_deref() {
        Some(s) => Some(due::parse_due(s, today).map_err(anyhow::Error::msg)?),
        None => None,
    };

    let id = storage::next_id(root, &title).context("generating next ID")?;
    let mut task = Task::new(id.clone(), title.clone());
    if let Some(p) = args.priority {
        task.item.priority = p.into_core();
    }
    if !args.tags.is_empty() {
        task.item.tags = args.tags.clone();
    }
    task.due_date = due_date;

    storage::save_task(root, &task).context("saving task")?;

    let all = storage::load_tasks(root).unwrap_or_default();
    let full_ids: Vec<&str> = all.iter().map(|t| t.item.id.as_str()).collect();
    let labels = display::format_ids(&full_ids);
    let label = all
        .iter()
        .zip(labels.iter())
        .find_map(|(t, l)| (t.item.id == id).then_some(l.as_str()))
        .unwrap_or("");

    let mut line = format!(
        "{} {}  {}",
        color::success("added"),
        color::id(label),
        title
    );
    if let Some(d) = due_date {
        let (label_due, sev) = due::render_due(d, today);
        line.push_str(&format!("  {}", colored_due(&label_due, sev)));
    }
    if !args.tags.is_empty() {
        line.push_str(&format!("  {}", render_tags(&args.tags)));
    }
    println!("{line}");
    Ok(())
}

fn run_ls(root: &Path, args: &LsArgs) -> Result<()> {
    let today = Local::now().date_naive();
    let due_filter = match args.due.as_deref() {
        Some(s) => Some(due::parse_due(s, today).map_err(anyhow::Error::msg)?),
        None => None,
    };

    let tasks = storage::load_tasks(root).context("loading tasks")?;
    let filtered: Vec<&Task> = tasks
        .iter()
        .filter(|t| args.all || t.item.is_active())
        .filter(|t| match due_filter {
            None => true,
            // 'today' includes overdue, anything else is exact-match.
            Some(d) if d == today => t.due_date.is_some_and(|td| td <= d),
            Some(d) => t.due_date == Some(d),
        })
        .filter(|t| {
            args.tags
                .iter()
                .all(|want| t.item.tags.iter().any(|h| h == want))
        })
        .collect();

    if filtered.is_empty() {
        println!("No open tasks. Add one with: jot add \"<title>\"");
        return Ok(());
    }

    let full_ids: Vec<&str> = filtered.iter().map(|t| t.item.id.as_str()).collect();
    let labels = display::format_ids(&full_ids);

    let show_due = filtered.iter().any(|t| t.due_date.is_some());
    let show_tags = filtered.iter().any(|t| !t.item.tags.is_empty());

    let due_labels: Vec<(String, DueSeverity)> = filtered
        .iter()
        .map(|t| match t.due_date {
            Some(d) => due::render_due(d, today),
            None => (String::new(), DueSeverity::Later),
        })
        .collect();
    let tag_labels: Vec<String> = filtered
        .iter()
        .map(|t| {
            if t.item.tags.is_empty() {
                String::new()
            } else {
                render_tags(&t.item.tags)
            }
        })
        .collect();
    let tag_widths: Vec<usize> = filtered
        .iter()
        .map(|t| {
            t.item
                .tags
                .iter()
                .map(|s| s.len() + 1)
                .sum::<usize>()
                .saturating_sub(1)
                .saturating_add(if t.item.tags.is_empty() {
                    0
                } else {
                    t.item.tags.len()
                })
        })
        .collect();

    let id_width = labels.iter().map(|s| s.len()).max().unwrap_or(2).max(2);
    let due_width = if show_due {
        due_labels
            .iter()
            .map(|(s, _)| s.len())
            .max()
            .unwrap_or(3)
            .max(3)
    } else {
        0
    };
    let tags_width = if show_tags {
        tag_widths.iter().copied().max().unwrap_or(4).max(4)
    } else {
        0
    };

    let term_w = color::terminal_width();
    let fixed = id_width
        + 1
        + if show_due { due_width + 1 } else { 0 }
        + if show_tags { tags_width + 1 } else { 0 };
    let min_title = 20;
    let title_col = (term_w.saturating_sub(fixed)).max(min_title);
    let frame_w = (fixed + title_col).min(term_w.max(1));

    let mut headers: Vec<(&str, usize)> = vec![("ID", id_width), ("TITLE", title_col)];
    if show_due {
        headers.push(("DUE", due_width));
    }
    if show_tags {
        headers.push(("TAGS", tags_width));
    }
    println!("{}", color::header(&headers, frame_w));

    for (((task, label), (due_str, due_sev)), plain_tags) in filtered
        .iter()
        .zip(labels.iter())
        .zip(due_labels.iter())
        .zip(tag_labels.iter())
    {
        let id_cell = color::id(&format!("{label:<id_width$}"));
        let title_cell = format!("{:<w$}", task.item.title, w = title_col);
        let mut line = format!("{id_cell} {title_cell}");
        if show_due {
            let cell = if due_str.is_empty() {
                format!("{:<w$}", "", w = due_width)
            } else {
                let padded = format!("{due_str:<due_width$}");
                colored_due(&padded, *due_sev)
            };
            line.push_str(&format!(" {cell}"));
        }
        if show_tags {
            let padding = tags_width.saturating_sub(tag_plain_width(task));
            let cell = if plain_tags.is_empty() {
                format!("{:<w$}", "", w = tags_width)
            } else {
                format!("{plain_tags}{}", " ".repeat(padding))
            };
            line.push_str(&format!(" {cell}"));
        }
        println!("{line}");
    }

    println!(
        "{}",
        color::footer(&color::plural(filtered.len(), "task"), frame_w)
    );
    Ok(())
}

fn run_rm(root: &Path, id: &str) -> Result<()> {
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

fn colored_due(label: &str, severity: DueSeverity) -> String {
    match severity {
        DueSeverity::Overdue => color::danger(label),
        DueSeverity::Today => color::warning(label),
        DueSeverity::Soon | DueSeverity::Later => label.to_string(),
    }
}

fn render_tags(tags: &[String]) -> String {
    tags.iter()
        .map(|t| color::info(&format!("#{t}")))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Visible width of the tag column for a given task: '#' + tag per tag,
/// joined by single spaces.
fn tag_plain_width(task: &Task) -> usize {
    if task.item.tags.is_empty() {
        0
    } else {
        task.item.tags.iter().map(|t| t.len() + 1).sum::<usize>()
            + task.item.tags.len().saturating_sub(1)
    }
}
