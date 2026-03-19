// Copyright (c) 2026 Joydev GmbH (joydev.com)
// SPDX-License-Identifier: MIT

use std::io::IsTerminal;

use clap::Parser;

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
    /// List tasks
    Ls,
    /// Mark a task as done
    Done(DoneArgs),
}

#[derive(clap::Args)]
struct AddArgs {
    /// Task title
    title: String,
}

#[derive(clap::Args)]
struct DoneArgs {
    /// Task ID
    id: String,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let show_fortune = matches!(
        &cli.command,
        None | Some(Commands::Ls) | Some(Commands::Done(_))
    );

    match cli.command {
        Some(Commands::Add(args)) => {
            println!("Would add task: {}", args.title);
        }
        Some(Commands::Ls) => {
            println!("Would list tasks");
        }
        Some(Commands::Done(args)) => {
            println!("Would complete task: {}", args.id);
        }
        None => {
            println!("jot -- personal task manager");
            println!("Run 'jot --help' for usage");
        }
    }

    if show_fortune && std::io::stdout().is_terminal() {
        if let Some(text) = joy_core::fortune::fortune(None, 0.2) {
            eprintln!("\n\x1b[2m{text}\x1b[0m");
        }
    }

    Ok(())
}
