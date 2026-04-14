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
use jot_core::due::{self, DueSeverity, LabelMode};
use jot_core::model::Task;
use jot_core::storage;

#[derive(Parser)]
#[command(name = "jot", version, about = "Personal task manager")]
struct Cli {
    /// Colorize output (auto by default: on when stdout is a TTY and NO_COLOR is unset)
    #[arg(long, value_enum, global = true, default_value_t = color::ColorChoice::Auto)]
    color: color::ColorChoice,

    /// Use compact labels ('ext', 'tod', 'tmw', '-2d') instead of the
    /// full spelling. Also triggered by the JOT_SHORT environment variable.
    #[arg(short = 's', long, global = true)]
    short: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Add a new task
    Add(AddArgs),
    /// List open tasks
    Ls(LsArgs),
    /// Show the full details of a task
    Show(ShowArgs),
    /// Modify a task (title, due, priority, tags, description, assignee)
    Edit(EditArgs),
    /// Assign a task to a member (shorthand for 'jot edit --assign')
    Assign(AssignArgs),
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

    /// Long-form description. Multi-line input requires shell quoting.
    #[arg(long)]
    description: Option<String>,

    /// Assignee (free-form, typically an e-mail address).
    #[arg(long)]
    assign: Option<String>,

    /// Task title. Words are joined with spaces, so quoting is only needed
    /// when the shell would otherwise eat characters (e.g. `!`, `*`, `?`).
    /// Flags may appear before or after the title words.
    #[arg(num_args = 1..)]
    title: Vec<String>,
}

#[derive(clap::Args)]
struct ShowArgs {
    /// Task ID (short `#A1` or full `TODO-00A1-EA`).
    id: String,
}

#[derive(clap::Args)]
struct EditArgs {
    /// Task ID.
    id: String,

    /// Replace the title.
    #[arg(long)]
    title: Option<String>,

    /// Set due date: `today`, `tomorrow`, or `YYYY-MM-DD`.
    #[arg(short, long)]
    due: Option<String>,

    /// Clear the due date.
    #[arg(long, conflicts_with = "due")]
    no_due: bool,

    /// Set priority.
    #[arg(short, long, value_enum)]
    priority: Option<PriorityArg>,

    /// Add a tag (repeatable).
    #[arg(long = "add-tag")]
    add_tags: Vec<String>,

    /// Remove a tag (repeatable).
    #[arg(long = "remove-tag")]
    remove_tags: Vec<String>,

    /// Replace the description text.
    #[arg(long)]
    description: Option<String>,

    /// Clear the description.
    #[arg(long, conflicts_with = "description")]
    no_description: bool,

    /// Add an assignee (repeatable).
    #[arg(long)]
    assign: Vec<String>,

    /// Remove an assignee (repeatable).
    #[arg(long)]
    unassign: Vec<String>,
}

#[derive(clap::Args)]
struct AssignArgs {
    /// Task ID.
    id: String,
    /// Member identifier (e.g. e-mail address).
    member: String,
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

    let mode = if cli.short || std::env::var_os("JOT_SHORT").is_some() {
        LabelMode::Short
    } else {
        LabelMode::Long
    };

    let root = std::env::current_dir().context("cannot read current directory")?;

    match cli.command {
        Some(Commands::Add(args)) => run_add(&root, args, mode)?,
        Some(Commands::Ls(args)) => run_ls(&root, &args, mode)?,
        None => run_ls(&root, &LsArgs::default(), mode)?,
        Some(Commands::Show(args)) => run_show(&root, &args.id, mode)?,
        Some(Commands::Edit(args)) => run_edit(&root, args, mode)?,
        Some(Commands::Assign(args)) => run_assign(&root, &args.id, &args.member)?,
        Some(Commands::Rm(args)) => run_rm(&root, &args.id)?,
    }

    if std::io::stdout().is_terminal() {
        if let Some(text) = joy_core::fortune::fortune(None, 0.2) {
            eprintln!("\n\x1b[2m{text}\x1b[0m");
        }
    }

    Ok(())
}

fn run_add(root: &Path, args: AddArgs, mode: LabelMode) -> Result<()> {
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
    task.item.description = args.description.clone();
    if let Some(member) = args.assign.clone() {
        task.item.assignees.push(joy_core::model::item::Assignee {
            member,
            capabilities: Vec::new(),
        });
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
    if let Some(p_label) = priority_label(&task.item.priority, mode) {
        line.push_str(&format!(
            "  {}",
            colored_priority(&task.item.priority, p_label)
        ));
    }
    if let Some(d) = due_date {
        let (label_due, sev) = due::render_due(d, today, mode);
        line.push_str(&format!("  {}", colored_due(&label_due, sev)));
    }
    if !args.tags.is_empty() {
        line.push_str(&format!("  {}", render_tags(&args.tags)));
    }
    println!("{line}");
    Ok(())
}

fn run_ls(root: &Path, args: &LsArgs, mode: LabelMode) -> Result<()> {
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

    let show_prio = filtered
        .iter()
        .any(|t| priority_label(&t.item.priority, mode).is_some());
    let show_due = filtered.iter().any(|t| t.due_date.is_some());
    let show_tags = filtered.iter().any(|t| !t.item.tags.is_empty());

    let prio_labels: Vec<&'static str> = filtered
        .iter()
        .map(|t| priority_label(&t.item.priority, mode).unwrap_or(""))
        .collect();

    let due_labels: Vec<(String, DueSeverity)> = filtered
        .iter()
        .map(|t| match t.due_date {
            Some(d) => due::render_due(d, today, mode),
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
    let prio_header = match mode {
        LabelMode::Long => "PRIORITY",
        LabelMode::Short => "PRIO",
    };
    let prio_width = if show_prio {
        prio_labels
            .iter()
            .map(|s| s.len())
            .max()
            .unwrap_or(0)
            .max(prio_header.len())
    } else {
        0
    };
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
        + if show_prio { prio_width + 1 } else { 0 }
        + if show_due { due_width + 1 } else { 0 }
        + if show_tags { tags_width + 1 } else { 0 };
    let min_title = 20;
    let title_col = (term_w.saturating_sub(fixed)).max(min_title);
    let frame_w = (fixed + title_col).min(term_w.max(1));

    let mut headers: Vec<(&str, usize)> = vec![("ID", id_width)];
    if show_prio {
        headers.push((prio_header, prio_width));
    }
    if show_due {
        headers.push(("DUE", due_width));
    }
    if show_tags {
        headers.push(("TAGS", tags_width));
    }
    headers.push(("TITLE", title_col));
    println!("{}", color::header(&headers, frame_w));

    for ((((task, label), prio_str), (due_str, due_sev)), plain_tags) in filtered
        .iter()
        .zip(labels.iter())
        .zip(prio_labels.iter())
        .zip(due_labels.iter())
        .zip(tag_labels.iter())
    {
        let id_cell = color::id(&format!("{label:<id_width$}"));
        let mut line = id_cell.clone();
        if show_prio {
            let cell = if prio_str.is_empty() {
                format!("{:<w$}", "", w = prio_width)
            } else {
                let colored = colored_priority(&task.item.priority, prio_str);
                let pad = prio_width.saturating_sub(prio_str.len());
                format!("{colored}{}", " ".repeat(pad))
            };
            line.push_str(&format!(" {cell}"));
        }
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
        line.push_str(&format!(" {}", task.item.title));
        println!("{line}");
    }

    println!(
        "{}",
        color::footer(&color::plural(filtered.len(), "task"), frame_w)
    );
    Ok(())
}

fn run_show(root: &Path, id: &str, mode: LabelMode) -> Result<()> {
    let task = storage::load_task(root, id).context("loading task")?;
    let short = display::short_id(&task.item.id);
    let today = Local::now().date_naive();
    let width = color::terminal_width();

    println!("{}", color::separator(width));
    println!(
        "{}  {}  {}",
        color::id(&short),
        task.item.title,
        color::label(&task.item.id)
    );
    println!("{}", color::separator(width));

    if let Some(p_label) = priority_label(&task.item.priority, mode) {
        println!(
            "{:<10} {}",
            color::label("Priority:"),
            colored_priority(&task.item.priority, p_label)
        );
    }
    if let Some(d) = task.due_date {
        let (label, sev) = due::render_due(d, today, mode);
        println!(
            "{:<10} {} ({})",
            color::label("Due:"),
            colored_due(&label, sev),
            d
        );
    }
    if !task.item.tags.is_empty() {
        println!(
            "{:<10} {}",
            color::label("Tags:"),
            render_tags(&task.item.tags)
        );
    }
    if !task.item.assignees.is_empty() {
        let members: Vec<&str> = task
            .item
            .assignees
            .iter()
            .map(|a| a.member.as_str())
            .collect();
        println!("{:<10} {}", color::label("Assignees:"), members.join(", "));
    }
    if let Some(src) = &task.source {
        println!("{:<10} {}", color::label("Source:"), src);
    }
    println!(
        "{:<10} {}",
        color::label("Created:"),
        task.item.created.format("%Y-%m-%d %H:%M")
    );
    println!(
        "{:<10} {}",
        color::label("Updated:"),
        task.item.updated.format("%Y-%m-%d %H:%M")
    );

    if let Some(desc) = &task.item.description {
        if !desc.is_empty() {
            println!();
            println!("{}", color::label("Description:"));
            for line in desc.lines() {
                println!("  {line}");
            }
        }
    }

    println!("{}", color::separator(width));
    Ok(())
}

fn run_edit(root: &Path, args: EditArgs, mode: LabelMode) -> Result<()> {
    let mut task = storage::load_task(root, &args.id).context("loading task")?;
    let today = Local::now().date_naive();

    if let Some(t) = args.title {
        if t.trim().is_empty() {
            anyhow::bail!("title must not be empty");
        }
        task.item.title = t;
    }
    if args.no_due {
        task.due_date = None;
    }
    if let Some(s) = args.due.as_deref() {
        task.due_date = Some(due::parse_due(s, today).map_err(anyhow::Error::msg)?);
    }
    if let Some(p) = args.priority {
        task.item.priority = p.into_core();
    }
    for tag in &args.add_tags {
        if !task.item.tags.iter().any(|t| t == tag) {
            task.item.tags.push(tag.clone());
        }
    }
    if !args.remove_tags.is_empty() {
        task.item.tags.retain(|t| !args.remove_tags.contains(t));
    }
    if args.no_description {
        task.item.description = None;
    }
    if let Some(desc) = args.description {
        task.item.description = Some(desc);
    }
    for member in &args.assign {
        if !task.item.assignees.iter().any(|a| &a.member == member) {
            task.item.assignees.push(joy_core::model::item::Assignee {
                member: member.clone(),
                capabilities: Vec::new(),
            });
        }
    }
    if !args.unassign.is_empty() {
        task.item
            .assignees
            .retain(|a| !args.unassign.contains(&a.member));
    }

    task.item.updated = chrono::Utc::now();
    storage::update_task(root, &task).context("saving task")?;

    let all = storage::load_tasks(root).unwrap_or_default();
    let full_ids: Vec<&str> = all.iter().map(|t| t.item.id.as_str()).collect();
    let labels = display::format_ids(&full_ids);
    let label = all
        .iter()
        .zip(labels.iter())
        .find_map(|(t, l)| (t.item.id == task.item.id).then_some(l.as_str()))
        .unwrap_or("");
    let mut line = format!(
        "{} {}  {}",
        color::success("updated"),
        color::id(label),
        task.item.title
    );
    if let Some(p_label) = priority_label(&task.item.priority, mode) {
        line.push_str(&format!(
            "  {}",
            colored_priority(&task.item.priority, p_label)
        ));
    }
    if let Some(d) = task.due_date {
        let (lbl, sev) = due::render_due(d, today, mode);
        line.push_str(&format!("  {}", colored_due(&lbl, sev)));
    }
    if !task.item.tags.is_empty() {
        line.push_str(&format!("  {}", render_tags(&task.item.tags)));
    }
    println!("{line}");
    Ok(())
}

fn run_assign(root: &Path, id: &str, member: &str) -> Result<()> {
    let mut task = storage::load_task(root, id).context("loading task")?;
    if !task.item.assignees.iter().any(|a| a.member == member) {
        task.item.assignees.push(joy_core::model::item::Assignee {
            member: member.to_string(),
            capabilities: Vec::new(),
        });
    }
    task.item.updated = chrono::Utc::now();
    storage::update_task(root, &task).context("saving task")?;
    let short = display::short_id(&task.item.id);
    println!(
        "{} {} to {}",
        color::success("assigned"),
        color::id(&short),
        member
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

/// Priority label for display. Returns `None` for the default Medium
/// so callers can hide the column and status cell when nothing is
/// out-of-band. `Long` spells names out ('extreme'), `Short` uses the
/// three-letter abbreviations from joy-cli ('ext').
fn priority_label(p: &Priority, mode: LabelMode) -> Option<&'static str> {
    match (p, mode) {
        (Priority::Low, _) => Some("low"),
        (Priority::Medium, _) => None,
        (Priority::High, LabelMode::Long) => Some("high"),
        (Priority::High, LabelMode::Short) => Some("hig"),
        (Priority::Critical, LabelMode::Long) => Some("critical"),
        (Priority::Critical, LabelMode::Short) => Some("crt"),
        (Priority::Extreme, LabelMode::Long) => Some("extreme"),
        (Priority::Extreme, LabelMode::Short) => Some("ext"),
    }
}

fn colored_priority(p: &Priority, label: &str) -> String {
    match p {
        Priority::Low => color::inactive(label),
        Priority::Medium => label.to_string(),
        Priority::High => color::danger(label),
        Priority::Critical | Priority::Extreme => color::danger_bold(label),
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
