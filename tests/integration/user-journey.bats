#!/usr/bin/env bats
# End-to-end user journey for the minimal jot CLI.
# Covers: capture -> list -> remove, the no-args-equals-ls shortcut,
# and JOT-002F-4D short-form ID display with adaptive disambiguation.

load setup

@test "fresh workspace: 'jot' with no args greets empty list" {
    run jot
    [ "$status" -eq 0 ]
    [[ "$output" == *"No open tasks"* ]]
}

@test "user journey: add, list, remove one task" {
    # Capture
    run jot add "Buy milk"
    [ "$status" -eq 0 ]
    [[ "$output" == *"added"*"#1"*"Buy milk"* ]]

    # YAML on disk uses full ADR-027 ID
    [ -d ".jot/items" ]
    ls .jot/items/*.yaml | grep -q "TODO-0001-.*-buy-milk"

    # List (explicit) shows short form and table frame
    run jot ls
    [ "$status" -eq 0 ]
    [[ "$output" == *"ID"*"TITLE"* ]]
    [[ "$output" == *"#1"* ]]
    [[ "$output" == *"Buy milk"* ]]
    [[ "$output" == *"1 task"* ]]
    [[ "$output" != *"TODO-"* ]]

    # Default (no subcommand) matches ls
    run jot
    [ "$status" -eq 0 ]
    [[ "$output" == *"Buy milk"* ]]

    # Remove by short form
    run jot rm "#1"
    [ "$status" -eq 0 ]
    [[ "$output" == *"removed"*"#1"*"Buy milk"* ]]

    # Empty again
    run jot
    [ "$status" -eq 0 ]
    [[ "$output" == *"No open tasks"* ]]
}

@test "multiple tasks: counters increment, leading zeros stripped" {
    jot add "First"    >/dev/null
    jot add "Second"   >/dev/null
    jot add "Third"    >/dev/null

    run jot ls
    [ "$status" -eq 0 ]
    # Extract just the task rows (skip separators + header + footer).
    rows=$(echo "$output" | grep -E "^#")
    first_row=$(echo "$rows" | sed -n '1p')
    last_row=$(echo  "$rows" | sed -n '3p')
    [[ "$first_row" == "#1"*"First"* ]]
    [[ "$last_row"  == "#3"*"Third"* ]]

    # Footer reports the count.
    [[ "$output" == *"3 tasks"* ]]
}

@test "add accepts unquoted multi-word titles" {
    run jot add this is a test
    [ "$status" -eq 0 ]
    [[ "$output" == *"#1"*"this is a test"* ]]

    # Quoting still works for titles that need it (e.g. shell metacharacters).
    run jot add "Review PR 42"
    [ "$status" -eq 0 ]
    [[ "$output" == *"#2"*"Review PR 42"* ]]

    # Listing shows both with their original text intact.
    run jot
    [[ "$output" == *"this is a test"* ]]
    [[ "$output" == *"Review PR 42"*  ]]
}

@test "rm accepts multiple input forms" {
    jot add "Short form"   >/dev/null
    jot add "Bare hex"     >/dev/null
    jot add "Full ADR-027" >/dev/null

    # Short display form with hash
    run jot rm "#1"
    [ "$status" -eq 0 ]
    [[ "$output" == *"Short form"* ]]

    # Bare counter (no leading hash)
    run jot rm 2
    [ "$status" -eq 0 ]
    [[ "$output" == *"Bare hex"* ]]

    # Short ADR-027 form
    run jot rm TODO-0003
    [ "$status" -eq 0 ]
    [[ "$output" == *"Full ADR-027"* ]]
}

@test "rm of missing ID errors out cleanly" {
    run jot rm "#999"
    [ "$status" -ne 0 ]
    [[ "$output" == *"task not found"* ]]
}

@test "add auto-creates .jot/items/ on first use" {
    [ ! -d ".jot" ]
    jot add "Bootstrap" >/dev/null
    [ -d ".jot/items" ]
    ls .jot/items/*.yaml | grep -q "bootstrap"
}

@test "counters survive deletion gaps (no reuse)" {
    jot add "Alpha" >/dev/null
    jot add "Bravo" >/dev/null
    jot rm "#1" >/dev/null
    run jot add "Charlie"
    [ "$status" -eq 0 ]
    # Highest existing counter is 2, so next is 3 (never reuses 1).
    [[ "$output" == *"#3"* ]]
}

@test "add: flags may follow the title words" {
    # Flag after bare multi-word title.
    run jot add this is cool -d today
    [ "$status" -eq 0 ]
    [[ "$output" == *"this is cool"*"today"* ]]
    [[ "$output" != *"-d today"*              ]]

    # Flag after quoted title plus another flag.
    run jot add "this is also cool" -d today --tag work
    [ "$status" -eq 0 ]
    [[ "$output" == *"this is also cool"*"today"*"#work"* ]]

    # Flag, title, flag.
    run jot add -d today my chore --tag home
    [ "$status" -eq 0 ]
    [[ "$output" == *"my chore"*"today"*"#home"* ]]

    # Titles land in YAML without the flag tokens.
    ! grep -RlF -- "-d today" .jot/items
}

@test "add with --due, --priority, --tag shows in ls and on disk" {
    run jot add --due today --tag work --priority high Review PR 42
    [ "$status" -eq 0 ]
    [[ "$output" == *"added"*"#1"*"Review PR 42"*"today"*"#work"* ]]

    # YAML carries all fields.
    yaml=$(ls .jot/items/TODO-0001-*.yaml)
    grep -q "priority: high" "$yaml"
    grep -q "due_date:"      "$yaml"
    grep -q "^tags:"         "$yaml"
    grep -qF -- "- work"     "$yaml"

    # ls grows DUE and TAGS columns only when data is present.
    run jot
    [[ "$output" == *"DUE"* ]]
    [[ "$output" == *"TAGS"* ]]
    [[ "$output" == *"today"* ]]
    [[ "$output" == *"#work"* ]]
}

@test "ls without dues/tags stays a two-column table" {
    jot add Plain task >/dev/null
    run jot
    [[ "$output" == *"ID"*"TITLE"* ]]
    [[ "$output" != *"DUE"* ]]
    [[ "$output" != *"TAGS"* ]]
}

@test "ls --tag filters tasks by tag (AND semantics for multiple flags)" {
    jot add --tag work --tag urgent Fix prod outage >/dev/null
    jot add --tag home Wash the car >/dev/null
    jot add --tag work Review PR >/dev/null

    run jot ls --tag work
    [[ "$output" == *"Fix prod outage"* ]]
    [[ "$output" == *"Review PR"*       ]]
    [[ "$output" != *"Wash the car"*    ]]

    # AND semantics: --tag work --tag urgent only matches tasks with both.
    run jot ls --tag work --tag urgent
    [[ "$output" == *"Fix prod outage"* ]]
    [[ "$output" != *"Review PR"*       ]]
}

@test "ls --due today includes overdue, excludes future" {
    jot add --due today     Must do now      >/dev/null
    jot add --due tomorrow  Can wait         >/dev/null
    jot add --due 2020-01-01 Ancient overdue >/dev/null
    jot add                  Undated         >/dev/null

    run jot ls --due today
    [[ "$output" == *"Must do now"*      ]]
    [[ "$output" == *"Ancient overdue"*  ]]
    [[ "$output" != *"Can wait"*         ]]
    [[ "$output" != *"Undated"*          ]]
}

@test "add rejects unknown --due values" {
    run jot add --due friday Book dentist
    [ "$status" -ne 0 ]
    [[ "$output" == *"cannot parse due date"* ]]
}

@test "--color=never produces no ANSI escape codes" {
    jot add "Buy milk" >/dev/null
    run jot --color=never
    [ "$status" -eq 0 ]
    # No ESC (0x1b) bytes anywhere in the output.
    [[ "$output" != *$'\x1b'* ]]
}

@test "--color=always emits ANSI codes even when piped" {
    jot add "Buy milk" >/dev/null
    run jot --color=always
    [ "$status" -eq 0 ]
    # At least one ESC byte present.
    [[ "$output" == *$'\x1b'* ]]
}

@test "NO_COLOR env var disables colors without --color flag" {
    jot add "Buy milk" >/dev/null
    NO_COLOR=1 run jot
    [ "$status" -eq 0 ]
    [[ "$output" != *$'\x1b'* ]]
}

@test "collision: same counter from two sources shows expanded form" {
    # Simulate a post-sync state: two YAML files with the same counter
    # but different title-hash suffixes (what would happen if two
    # devices concurrently created task 1 before sync).
    mkdir -p .jot/items
    cat > .jot/items/TODO-0001-EA-review-pr.yaml <<'YAML'
id: TODO-0001-EA
title: Review PR
type: task
status: new
priority: medium
capabilities:
- implement
created: 2026-04-14T10:00:00Z
updated: 2026-04-14T10:00:00Z
YAML
    cat > .jot/items/TODO-0001-7F-call-mom.yaml <<'YAML'
id: TODO-0001-7F
title: Call Mom
type: task
status: new
priority: medium
capabilities:
- implement
created: 2026-04-14T10:00:00Z
updated: 2026-04-14T10:00:00Z
YAML
    jot add "Book dentist" >/dev/null

    run jot ls
    [ "$status" -eq 0 ]
    # Colliding rows must show the suffix; the non-colliding row stays short.
    [[ "$output" == *"#1-EA"* ]]
    [[ "$output" == *"#1-7F"* ]]
    [[ "$output" == *"#2"*   ]]
    [[ "$output" != *"#2-"*  ]]

    # Ambiguous short input is rejected, but with a useful hint.
    run jot rm "#1"
    [ "$status" -ne 0 ]
    [[ "$output" == *"ambiguous"* ]]

    # Disambiguated input works.
    run jot rm "#1-EA"
    [ "$status" -eq 0 ]
    [[ "$output" == *"removed"* ]]

    # After removal the remaining #1 is unique again and displays short.
    run jot ls
    [[ "$output" == *"#1"* ]]
    [[ "$output" != *"#1-"* ]]
}
