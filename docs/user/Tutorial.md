# Jyn Tutorial

Jyn is a fast personal task manager that lives in your terminal. Tasks are
plain files in a Git repo, so your todo list is versioned alongside your work
and stays fully offline. Anyone comfortable on the command line can use it;
a web portal and desktop and mobile apps are on the way, opening jyn up well
beyond the terminal. Jyn is the personal companion to Joy, the team
product-management tool in the Joyint ecosystem.

This guide covers the command-line tool and walks through everyday use. Run
`jyn tutorial -i` to browse it chapter by chapter instead of reading top to
bottom.

## Capturing Tasks

The fastest path is `jyn add` followed by a title. Quotes are only needed when
the shell would otherwise interpret a character.

```sh
jyn add Review the deploy script
jyn add "Call Lisa about the Q3 numbers"
```

Add structure as you go:

```sh
jyn add "Fix broken pipeline" --priority high --tag work
jyn add "Submit travel expenses" --due 2026-06-01 --tag admin
```

### Priority

Five levels, from lowest to highest: `low`, `medium`, `high`, `critical`,
`extreme`. The flag is `--priority`, with the short form `-p` and the alias
`--prio`.

```sh
jyn add "Rotate the leaked token" --prio critical
```

### Due dates

`--due` takes a calendar date in `YYYY-MM-DD` form.

```sh
jyn add "Renew the TLS certificate" --due 2026-07-15
```

### Tags

Tags are free-text labels for grouping and filtering. Repeat `--tag` (short
`-t`) to attach several.

```sh
jyn add "Plan the offsite" --tag work --tag planning
```

### Description and assignee

`--desc` (short `-d`) attaches a longer note; `--assign` (short `-a`) records
who owns the task by e-mail.

```sh
jyn add "Draft the RFC" --desc "Cover storage layout and migration" --assign me@example.com
```

## Listing Tasks

`jyn` on its own (or `jyn ls`) lists your open tasks. The list flags work
without typing `ls`, so `jyn -t work` is the same as `jyn ls -t work`.

```sh
jyn                  # open tasks, smart order
jyn ls --tag work    # only work tasks
jyn ls --due today   # due today or overdue
```

### What is shown

By default the list hides closed and archived tasks. Widen it when you need
history:

```sh
jyn --all        # include closed and archived
jyn --closed     # only completed tasks
jyn --archived   # only archived tasks
```

### Sorting

`--sort` accepts `smart` (the default), `created`, `updated`, `priority`,
`due`, and `title`. `smart` surfaces what to do next by combining urgency and
priority. Add `--reverse` (short `-r`) to flip the order.

```sh
jyn ls --sort due
jyn ls --sort priority --reverse
```

## Where Jyn Stores Your Tasks

Your tasks are plain YAML files under a `.jyn/` directory, one file per
task in `.jyn/items/`. Because they are ordinary files in a Git repo, you
can read, grep, version, and sync them like anything else in your project.

Jyn finds that directory the same way Git finds a repo: it starts in your
current directory and walks **upward** to the nearest `.jyn/`. So once a
workspace exists, you can run `jyn` from any subdirectory below it and see
the same task list. If no `.jyn/` exists anywhere above you, the first
`jyn add` creates one in the current directory.

The footer of every listing shows which `.jyn/` is currently active:

```
--------------------------------------------------------------------------------
3 tasks  ~/notes/.jyn
```

If a list looks empty when you did not expect it to, check that footer: you
are most likely in a directory that resolves to a different workspace than
the one you added tasks to.

## Viewing and Editing

`jyn show` prints the full detail of one task, including its description and
tags.

```sh
jyn show #A1
```

`jyn edit` changes fields after the fact. Tags and assignees are added and
removed individually; due date and description have explicit clear flags.

```sh
jyn edit #A1 --title "Fix the nightly pipeline" --prio high
jyn edit #A1 --add-tag urgent --remove-tag work
jyn edit #A1 --due 2026-08-01      # set or change the due date
jyn edit #A1 --no-due              # clear the due date
jyn edit #A1 --no-desc             # clear the description
```

## The Task Lifecycle

A task starts open. Complete it with `jyn close` (aliases `jyn done` and
`jyn c`); bring it back with `jyn reopen`.

```sh
jyn done #A1
jyn reopen #A1
```

Archiving hides a task from the normal list without deleting it, which suits
things you want out of the way but kept on record. `jyn rm` deletes a task for
good.

```sh
jyn archive #A1
jyn unarchive #A1
jyn rm #A1
```

## Referring to Tasks

Every command that takes a task accepts either the short form shown in the
list (for example `#A1`) or the full task ID. The short form is the convenient
one for day-to-day work.

## Assigning Tasks

When you share a repo, `jyn assign` records who owns a task by e-mail.

```sh
jyn assign #A1 lisa@example.com
```

## Configuration

Jyn reads layered YAML config: a global file under `~/.config/jyn/config.yaml`
for personal defaults, and an optional project-local `./.jyn/config.yaml` that
only applies inside that directory.

```sh
jyn config                                    # merged view; [default] marks unset values
jyn config get output.fortune                 # read one value
jyn config set --global output.fortune false  # personal default, everywhere
jyn config set --local  output.fortune-category tech  # this project only
```

When no config file exists yet, `jyn config set` requires an explicit
`--global` or `--local` so a `.jyn/` directory never appears by surprise.
Unknown keys are rejected with a hint instead of being silently stored.

## Getting Help

Every command has `--help`. Append it to any subcommand for the full flag
list:

```sh
jyn add --help
jyn ls --help
```

Run `jyn tutorial` any time to reread this guide, or `jyn tutorial -i` to jump
between chapters.
</content>
