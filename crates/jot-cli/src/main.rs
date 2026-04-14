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
#[command(
    name = "jot",
    version,
    about = "Personal task manager",
    infer_subcommands = true
)]
struct Cli {
    /// Colorize output.
    #[arg(
        long,
        value_enum,
        global = true,
        default_value_t = color::ColorChoice::Auto,
        hide_possible_values = true
    )]
    color: color::ColorChoice,

    /// Use compact labels. Also via JOT_SHORT.
    #[arg(long, global = true)]
    short: bool,

    /// Ls-style flags usable without typing 'ls': `jot -a`, `jot --sort
    /// title`, `jot --tag work`, and so on.
    #[command(flatten)]
    ls: LsArgs,

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
    /// Modify a task
    Edit(EditArgs),
    /// Assign a task to a member
    Assign(AssignArgs),
    /// Mark a task as done
    #[command(alias = "done")]
    Close(IdArgs),
    /// Reopen a closed task
    Reopen(IdArgs),
    /// Archive a task (local only)
    Archive(IdArgs),
    /// Restore an archived task
    Unarchive(IdArgs),
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
    /// Due date.
    #[arg(long)]
    due: Option<String>,

    /// Priority.
    #[arg(short, long, value_enum, hide_possible_values = true)]
    priority: Option<PriorityArg>,

    /// Tag (repeatable).
    #[arg(short, long = "tag")]
    tags: Vec<String>,

    /// Description.
    #[arg(short = 'd', long = "desc", alias = "description")]
    description: Option<String>,

    /// Assignee (e-mail).
    #[arg(short = 'a', long, alias = "assignee")]
    assign: Option<String>,

    /// Task title. Quoting is only needed for shell metacharacters.
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

    /// Set due date.
    #[arg(long)]
    due: Option<String>,

    /// Clear the due date.
    #[arg(long, conflicts_with = "due")]
    no_due: bool,

    /// Set priority.
    #[arg(short, long, value_enum, hide_possible_values = true)]
    priority: Option<PriorityArg>,

    /// Add a tag (repeatable).
    #[arg(long = "add-tag")]
    add_tags: Vec<String>,

    /// Remove a tag (repeatable).
    #[arg(long = "remove-tag")]
    remove_tags: Vec<String>,

    /// Replace the description.
    #[arg(short = 'd', long = "desc", alias = "description")]
    description: Option<String>,

    /// Clear the description.
    #[arg(
        long = "no-desc",
        alias = "no-description",
        conflicts_with = "description"
    )]
    no_description: bool,

    /// Add an assignee (repeatable).
    #[arg(short = 'a', long, alias = "assignee")]
    assign: Vec<String>,

    /// Remove an assignee (repeatable).
    #[arg(long, alias = "unassignee")]
    unassign: Vec<String>,
}

#[derive(clap::Args)]
struct AssignArgs {
    /// Task ID.
    id: String,
    /// Member (e-mail).
    member: String,
}

#[derive(clap::Args)]
struct IdArgs {
    /// Task ID (short `#A1` or full `TODO-00A1-EA`).
    id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum, Default)]
enum SortMode {
    /// What-should-I-do-next: urgency then priority (default).
    #[default]
    Smart,
    /// Creation order, oldest first.
    Created,
    /// Last-updated first.
    Updated,
    /// Priority only (extreme first).
    Priority,
    /// Due date ascending, no-date tasks at the end.
    Due,
    /// Alphabetical by title.
    Title,
}

#[derive(clap::Args, Default)]
struct LsArgs {
    /// Include closed and archived tasks.
    #[arg(short, long)]
    all: bool,

    /// Show only closed tasks.
    #[arg(long)]
    closed: bool,

    /// Show only archived tasks (normally hidden).
    #[arg(long)]
    archived: bool,

    /// Filter by due date (today includes overdue).
    #[arg(long)]
    due: Option<String>,

    /// Filter by tag (repeatable, AND).
    #[arg(short, long = "tag")]
    tags: Vec<String>,

    /// Sort order.
    #[arg(
        long,
        value_enum,
        default_value_t = SortMode::Smart,
        hide_possible_values = true
    )]
    sort: SortMode,

    /// Reverse the sort direction.
    #[arg(short = 'r', long)]
    reverse: bool,
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
        None => run_ls(&root, &cli.ls, mode)?,
        Some(Commands::Show(args)) => run_show(&root, &args.id, mode)?,
        Some(Commands::Edit(args)) => run_edit(&root, args, mode)?,
        Some(Commands::Assign(args)) => run_assign(&root, &args.id, &args.member)?,
        Some(Commands::Close(args)) => run_close(&root, &args.id)?,
        Some(Commands::Reopen(args)) => run_reopen(&root, &args.id)?,
        Some(Commands::Archive(args)) => run_archive(&root, &args.id)?,
        Some(Commands::Unarchive(args)) => run_unarchive(&root, &args.id)?,
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
    let mut filtered: Vec<&Task> = tasks
        .iter()
        .filter(|t| {
            // Visibility rules for status/archive:
            //   default:      active + closed, archived hidden
            //   --all:        everything
            //   --closed:     only closed (and not archived)
            //   --archived:   only archived
            if args.archived {
                return t.archived;
            }
            if args.closed {
                return matches!(t.item.status, joy_core::model::item::Status::Closed)
                    && !t.archived;
            }
            if args.all {
                return true;
            }
            !t.archived
        })
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

    // Sort per the user's requested mode. The smart default floats the
    // what-should-I-do-next task to the top; alternative modes sort by
    // a single dimension. --reverse flips the direction.
    apply_sort(&mut filtered, args.sort, args.reverse, today);

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
    let show_desc = filtered
        .iter()
        .any(|t| t.item.description.as_deref().is_some_and(|s| !s.is_empty()));
    let show_assignee = filtered.iter().any(|t| !t.item.assignees.is_empty());
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
    let assignee_labels: Vec<String> = filtered
        .iter()
        .map(|t| {
            t.item
                .assignees
                .iter()
                .map(|a| a.member.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .collect();
    let desc_labels: Vec<String> = filtered
        .iter()
        .map(|t| match t.item.description.as_deref() {
            Some(s) if !s.is_empty() => s.chars().count().to_string(),
            _ => String::new(),
        })
        .collect();
    // Tags: plain space-separated, no '#' - TAGS column sits rightmost.
    let tag_labels: Vec<String> = filtered.iter().map(|t| t.item.tags.join(" ")).collect();

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
    let desc_width = if show_desc {
        desc_labels
            .iter()
            .map(|s| s.len())
            .max()
            .unwrap_or(0)
            .max("DESC".len())
    } else {
        0
    };
    let assignee_width = if show_assignee {
        assignee_labels
            .iter()
            .map(|s| s.len())
            .max()
            .unwrap_or(0)
            .max("ASSIGNEE".len())
    } else {
        0
    };
    let tags_width = if show_tags {
        tag_labels
            .iter()
            .map(|s| s.len())
            .max()
            .unwrap_or(0)
            .max("TAGS".len())
    } else {
        0
    };

    let term_w = color::terminal_width();
    let fixed = id_width
        + 1
        + if show_prio { prio_width + 1 } else { 0 }
        + if show_due { due_width + 1 } else { 0 }
        + if show_desc { desc_width + 1 } else { 0 }
        + if show_assignee { assignee_width + 1 } else { 0 }
        + if show_tags { tags_width + 1 } else { 0 };

    // Prefer hugging content: TITLE column = max(longest title, "TITLE")
    // + 5 right-margin. If that does not fit in the terminal, fall back
    // to Joy-style truncation and let the table span the full width.
    let longest_title = filtered
        .iter()
        .map(|t| t.item.title.len())
        .max()
        .unwrap_or(0);
    let title_natural = longest_title.max("TITLE".len()) + 5;
    let min_title = 20;
    let (title_col, truncate) = if fixed + title_natural <= term_w {
        (title_natural, false)
    } else {
        (term_w.saturating_sub(fixed).max(min_title), true)
    };
    let frame_w = fixed + title_col;

    // Column order: ID [PRIO] [DUE] TITLE [ASSIGNEE] [TAGS]. Tags sit
    // rightmost; assignee is the column immediately to the left of tags.
    let mut headers: Vec<(&str, usize)> = vec![("ID", id_width)];
    if show_prio {
        headers.push((prio_header, prio_width));
    }
    if show_due {
        headers.push(("DUE", due_width));
    }
    headers.push(("TITLE", title_col));
    if show_desc {
        headers.push(("DESC", desc_width));
    }
    if show_assignee {
        headers.push(("ASSIGNEE", assignee_width));
    }
    if show_tags {
        headers.push(("TAGS", tags_width));
    }
    println!("{}", color::header(&headers, frame_w));

    for ((((((task, label), prio_str), (due_str, due_sev)), desc_str), assignee_str), tag_str) in
        filtered
            .iter()
            .zip(labels.iter())
            .zip(prio_labels.iter())
            .zip(due_labels.iter())
            .zip(desc_labels.iter())
            .zip(assignee_labels.iter())
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
        // Strikethrough + dim for closed, fainter for archived. The
        // styling wraps only the title text; trailing column padding
        // stays plain so the line isn't struck through across empty
        // space. In truncated mode we reserve 2 right-margin chars so
        // the `...` ending doesn't touch the next column.
        let title_text = if truncate {
            truncate_title(&task.item.title, title_col.saturating_sub(2))
        } else {
            task.item.title.clone()
        };
        let styled = if task.archived {
            color::strikethrough_faint(&title_text)
        } else if matches!(task.item.status, joy_core::model::item::Status::Closed) {
            color::strikethrough_dim(&title_text)
        } else {
            title_text.clone()
        };
        let pad = title_col.saturating_sub(title_text.len());
        line.push_str(&format!(" {styled}{}", " ".repeat(pad)));
        if show_desc {
            // Right-align the count so the digits line up cleanly.
            let cell = if desc_str.is_empty() {
                format!("{:<w$}", "", w = desc_width)
            } else {
                let padded = format!("{desc_str:>desc_width$}");
                color::inactive(&padded)
            };
            line.push_str(&format!(" {cell}"));
        }
        if show_assignee {
            let cell = format!("{assignee_str:<assignee_width$}");
            line.push_str(&format!(" {cell}"));
        }
        if show_tags {
            let cell = format!("{tag_str:<tags_width$}");
            line.push_str(&format!(" {}", color::info(&cell)));
        }
        // Trim trailing whitespace so the row doesn't paint past the
        // visible content in color mode.
        println!("{}", line.trim_end());
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

    // ---- Top band: identity + classification ----
    // Plain-text chunks power the width math; colored chunks power the
    // print. They must stay in sync.
    let mut left_plain: Vec<String> = vec![format!("{}  {}", short, task.item.id)];
    let mut left_colored: Vec<String> = vec![format!(
        "{}  {}",
        color::id(&short),
        color::label(&task.item.id)
    )];

    if !task.item.assignees.is_empty() {
        let members = task
            .item
            .assignees
            .iter()
            .map(|a| a.member.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        left_plain.push(format!("Assignee: {members}"));
        left_colored.push(format!("{} {members}", color::label("Assignee:")));
    }
    if let Some(p_label) = priority_label(&task.item.priority, mode) {
        left_plain.push(format!("Prio: {p_label}"));
        left_colored.push(format!(
            "{} {}",
            color::label("Prio:"),
            colored_priority(&task.item.priority, p_label)
        ));
    }
    if let Some(src) = &task.source {
        left_plain.push(format!("Source: {src}"));
        left_colored.push(format!("{} {src}", color::label("Source:")));
    }

    let (right_plain, right_colored) = if task.item.tags.is_empty() {
        (String::new(), String::new())
    } else {
        let joined = task.item.tags.join(" ");
        (
            format!("Tags: {joined}"),
            format!("{} {}", color::label("Tags:"), color::info(&joined)),
        )
    };

    let left_joined_plain = left_plain.join(", ");
    let left_joined_colored = left_colored.join(&color::label(", "));

    println!("{}", color::separator(width));
    if right_plain.is_empty() {
        println!("{left_joined_colored}");
    } else {
        let pad = width
            .saturating_sub(left_joined_plain.len() + right_plain.len())
            .max(1);
        println!(
            "{}{}{}",
            left_joined_colored,
            " ".repeat(pad),
            right_colored
        );
    }

    // ---- Middle band: the actual content ----
    println!("{}", color::separator(width));
    let label_w = "Description:".len();
    println!(
        "{:<w$} {}",
        color::label("Title:"),
        task.item.title,
        w = label_w
    );
    if let Some(d) = task.due_date {
        let (label, sev) = due::render_due(d, today, mode);
        let iso = d.format("%Y-%m-%d").to_string();
        let value = if label == iso {
            colored_due(&label, sev)
        } else {
            format!("{} ({})", colored_due(&label, sev), color::label(&iso))
        };
        println!("{:<w$} {value}", color::label("Due:"), w = label_w);
    }
    if let Some(desc) = &task.item.description {
        if !desc.is_empty() {
            let indent = " ".repeat(label_w + 1);
            let wrap_w = width.saturating_sub(label_w + 1).max(20);
            let mut first_line_label_printed = false;
            for para in desc.lines() {
                let wrapped = wrap_text(para, wrap_w);
                let mut wrapped_iter = wrapped.into_iter();
                if let Some(first) = wrapped_iter.next() {
                    if !first_line_label_printed {
                        println!("{:<w$} {first}", color::label("Description:"), w = label_w);
                        first_line_label_printed = true;
                    } else {
                        println!("{indent}{first}");
                    }
                }
                for rest in wrapped_iter {
                    println!("{indent}{rest}");
                }
            }
            // Handle the rare case of a fully empty paragraph array
            if !first_line_label_printed {
                println!("{:<w$}", color::label("Description:"), w = label_w);
            }
        }
    }

    // ---- Bottom band: record timestamps ----
    println!("{}", color::separator(width));
    // Whole footer line in label color - timestamps are record
    // metadata, not content, and should recede visually.
    let mut footer_parts = vec![
        format!("Created: {}", task.item.created.format("%Y-%m-%d %H:%M")),
        format!("Updated: {}", task.item.updated.format("%Y-%m-%d %H:%M")),
    ];
    if let Some(ts) = task.closed_at {
        footer_parts.push(format!("Closed: {}", ts.format("%Y-%m-%d %H:%M")));
    }
    if let Some(ts) = task.archived_at {
        footer_parts.push(format!("Archived: {}", ts.format("%Y-%m-%d %H:%M")));
    }
    println!("{}", color::label(&footer_parts.join(", ")));
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

fn run_close(root: &Path, id: &str) -> Result<()> {
    let mut task = storage::load_task(root, id).context("loading task")?;
    let now = chrono::Utc::now();
    task.item.status = joy_core::model::item::Status::Closed;
    task.closed_at = Some(now);
    task.item.updated = now;
    storage::update_task(root, &task).context("saving task")?;
    let short = display::short_id(&task.item.id);
    println!(
        "{} {}  {}",
        color::success("closed"),
        color::id(&short),
        task.item.title
    );
    Ok(())
}

fn run_reopen(root: &Path, id: &str) -> Result<()> {
    let mut task = storage::load_task(root, id).context("loading task")?;
    task.item.status = joy_core::model::item::Status::New;
    task.closed_at = None;
    task.item.updated = chrono::Utc::now();
    storage::update_task(root, &task).context("saving task")?;
    let short = display::short_id(&task.item.id);
    println!(
        "{} {}  {}",
        color::success("reopened"),
        color::id(&short),
        task.item.title
    );
    Ok(())
}

fn run_archive(root: &Path, id: &str) -> Result<()> {
    let mut task = storage::load_task(root, id).context("loading task")?;
    let now = chrono::Utc::now();
    task.archived = true;
    task.archived_at = Some(now);
    task.item.updated = now;
    storage::update_task(root, &task).context("saving task")?;
    let short = display::short_id(&task.item.id);
    println!(
        "{} {}  {}",
        color::success("archived"),
        color::id(&short),
        task.item.title
    );
    Ok(())
}

fn run_unarchive(root: &Path, id: &str) -> Result<()> {
    let mut task = storage::load_task(root, id).context("loading task")?;
    task.archived = false;
    task.archived_at = None;
    task.item.updated = chrono::Utc::now();
    storage::update_task(root, &task).context("saving task")?;
    let short = display::short_id(&task.item.id);
    println!(
        "{} {}  {}",
        color::success("unarchived"),
        color::id(&short),
        task.item.title
    );
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

/// Color due labels on a traffic-light-plus-dim gradient so dates are
/// always visually distinct from the title: overdue red, today yellow,
/// soon (this week) secondary green, later dim grey.
/// Apply the user-selected sort mode to `tasks`. All modes keep
/// archived/closed at the bottom (archived last, closed after active)
/// so the secondary dimension only reorders within those buckets.
fn apply_sort(tasks: &mut [&Task], mode: SortMode, reverse: bool, today: chrono::NaiveDate) {
    match mode {
        SortMode::Smart => tasks.sort_by_key(|t| sort_key(t, today)),
        SortMode::Created => tasks.sort_by_key(|t| (t.archived, is_closed(t), t.item.created)),
        SortMode::Updated => {
            // Updated: newest first (reverse the created-style tuple).
            tasks.sort_by(|a, b| {
                (a.archived, is_closed(a), std::cmp::Reverse(a.item.updated)).cmp(&(
                    b.archived,
                    is_closed(b),
                    std::cmp::Reverse(b.item.updated),
                ))
            });
        }
        SortMode::Priority => {
            tasks.sort_by_key(|t| {
                (
                    t.archived,
                    is_closed(t),
                    priority_rank(&t.item.priority),
                    t.item.created,
                )
            });
        }
        SortMode::Due => {
            // No-date tasks always land at the end within their bucket.
            tasks.sort_by_key(|t| {
                (
                    t.archived,
                    is_closed(t),
                    t.due_date.is_none(),
                    t.due_date,
                    t.item.created,
                )
            });
        }
        SortMode::Title => {
            tasks.sort_by(|a, b| {
                (a.archived, is_closed(a), a.item.title.to_lowercase()).cmp(&(
                    b.archived,
                    is_closed(b),
                    b.item.title.to_lowercase(),
                ))
            });
        }
    }
    if reverse {
        tasks.reverse();
    }
}

fn is_closed(t: &Task) -> bool {
    matches!(t.item.status, joy_core::model::item::Status::Closed)
}

fn priority_rank(p: &joy_core::model::item::Priority) -> u8 {
    use joy_core::model::item::Priority::*;
    match p {
        Extreme => 0,
        Critical => 1,
        High => 2,
        Medium => 3,
        Low => 4,
    }
}

/// Ordering key for the default ls sort.
///
/// The tuple sorts lexicographically; lower values float to the top.
///   0. archived (false < true)       - active + closed before archived
///   1. closed   (false < true)       - active before closed
///   2. urgency  (u8)                 - overdue, today, soon, later, none
///   3. priority (u8)                 - extreme, critical, high, medium, low
///   4. created  (DateTime<Utc>)      - older first as a stable tiebreak
fn sort_key(
    task: &Task,
    today: chrono::NaiveDate,
) -> (bool, bool, u8, u8, chrono::DateTime<chrono::Utc>) {
    let closed = matches!(task.item.status, joy_core::model::item::Status::Closed);
    let urgency: u8 = match task.due_date {
        Some(d) => {
            let delta = (d - today).num_days();
            if delta < 0 {
                0 // overdue
            } else if delta == 0 {
                1 // today
            } else if delta <= 6 {
                2 // soon (this week)
            } else {
                3 // later
            }
        }
        None => 4, // no date
    };
    let priority: u8 = match task.item.priority {
        joy_core::model::item::Priority::Extreme => 0,
        joy_core::model::item::Priority::Critical => 1,
        joy_core::model::item::Priority::High => 2,
        joy_core::model::item::Priority::Medium => 3,
        joy_core::model::item::Priority::Low => 4,
    };
    (task.archived, closed, urgency, priority, task.item.created)
}

fn colored_due(label: &str, severity: DueSeverity) -> String {
    match severity {
        DueSeverity::Overdue => color::danger(label),
        DueSeverity::Today => color::warning(label),
        DueSeverity::Soon => color::label(label),
        DueSeverity::Later => color::inactive(label),
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

/// Word-wrap text to the given width. Whitespace-only input returns empty.
/// Words longer than the width are emitted on their own line uncut (so the
/// terminal decides how to visually break them).
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
        } else if current.len() + 1 + word.len() <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(std::mem::take(&mut current));
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// Truncate a title to `max_len` characters, appending `...` when the
/// original exceeds the limit. Returns the original string untouched
/// when it already fits.
fn truncate_title(title: &str, max_len: usize) -> String {
    if title.len() <= max_len {
        return title.to_string();
    }
    if max_len <= 3 {
        return ".".repeat(max_len);
    }
    format!("{}...", &title[..max_len - 3])
}

fn render_tags(tags: &[String]) -> String {
    tags.iter()
        .map(|t| color::info(&format!("#{t}")))
        .collect::<Vec<_>>()
        .join(" ")
}
