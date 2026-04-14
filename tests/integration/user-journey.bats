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
    run jot add this is cool --due today
    [ "$status" -eq 0 ]
    [[ "$output" == *"this is cool"*"today"* ]]
    [[ "$output" != *"--due today"*              ]]

    # Flag after quoted title plus another flag.
    run jot add "this is also cool" --due today --tag work
    [ "$status" -eq 0 ]
    [[ "$output" == *"this is also cool"*"today"*"#work"* ]]

    # Flag, title, flag.
    run jot add --due today my chore --tag home
    [ "$status" -eq 0 ]
    [[ "$output" == *"my chore"*"today"*"#home"* ]]

    # Titles land in YAML without the flag tokens.
    ! grep -RlF -- "--due today" .jot/items
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

    # ls grows DUE and TAGS columns only when data is present. Tags are
    # rendered without '#' in the table.
    run jot
    [[ "$output" == *"DUE"*   ]]
    [[ "$output" == *"TAGS"*  ]]
    [[ "$output" == *"today"* ]]
    [[ "$output" == *"work"*  ]]
    [[ "$output" != *"#work"* ]]
}

@test "ls without extras stays a two-column table" {
    jot add Plain task >/dev/null
    run jot
    [[ "$output" == *"ID"*"TITLE"* ]]
    [[ "$output" != *"DUE"*  ]]
    [[ "$output" != *"TAGS"* ]]
    [[ "$output" != *"ASSIGNEE"* ]]
    [[ "$output" != *"PRIORITY"* ]]
    [[ "$output" != *"PRIO"* ]]
}

@test "ls columns: TAGS rightmost, ASSIGNEE to its left, no '#' prefix" {
    jot add --tag work --tag urgent -a claude@joy Review PR >/dev/null
    run jot
    [ "$status" -eq 0 ]
    # Header lists TITLE before ASSIGNEE before TAGS.
    header=$(echo "$output" | grep -E "^ID")
    [[ "$header" == *"TITLE"*"ASSIGNEE"*"TAGS"* ]]
    # Tags rendered without '#'.
    row=$(echo "$output" | grep "Review PR")
    [[ "$row" == *"claude@joy"*"work urgent"* ]]
    [[ "$row" != *"#work"* ]]
}

@test "due rendering: full year on ISO dates so 2026 and 2027 differ" {
    jot add --due 2026-04-24 same month 2026 >/dev/null
    jot add --due 2027-04-24 same month 2027 >/dev/null
    run jot
    [[ "$output" == *"2026-04-24"* ]]
    [[ "$output" == *"2027-04-24"* ]]
}

@test "due input: MM-DD and DD.MM shortcuts are accepted" {
    run jot add --due 04-30 near a
    [ "$status" -eq 0 ]
    run jot add --due 25.04 deutsch b
    [ "$status" -eq 0 ]
    run jot add --due 25.04.2027 deutsch c
    [ "$status" -eq 0 ]
    run jot
    [[ "$output" == *"near a"*    ]]
    [[ "$output" == *"deutsch b"* ]]
    [[ "$output" == *"deutsch c"* ]]
}

@test "short flag: -a works as --assign on add and edit" {
    run jot add -a horst@example.com Buy milk
    [ "$status" -eq 0 ]
    run jot show 1
    [[ "$output" == *"horst@example.com"* ]]

    run jot edit 1 -a claude@joy
    [ "$status" -eq 0 ]
    run jot show 1
    [[ "$output" == *"horst@example.com"* ]]
    [[ "$output" == *"claude@joy"* ]]
}

@test "priority column: long by default, short with --short" {
    jot add Plain task >/dev/null
    run jot
    [[ "$output" != *"PRIORITY"* ]]
    [[ "$output" != *"PRIO"* ]]

    # Long default: full spelling, header 'PRIORITY'.
    jot add -p extreme ohohooo >/dev/null
    run jot
    [[ "$output" == *"PRIORITY"* ]]
    [[ "$output" == *"extreme"*  ]]

    # --short: three-letter labels, header 'PRIO'.
    run jot --short
    [[ "$output" == *"PRIO"* ]]
    [[ "$output" != *"PRIORITY"* ]]
    [[ "$output" == *"ext"* ]]
    [[ "$output" != *"extreme"* ]]

    # Status lines mirror the mode.
    run jot add -p high fix a bug
    [[ "$output" == *"fix a bug"*"high"* ]]
    run jot --short add -p critical urgent cleanup
    [[ "$output" == *"urgent cleanup"*"crt"* ]]

    # Medium stays silent either way.
    run jot add plain task
    [[ "$output" != *"medium"* ]]
    run jot --short add plain task
    [[ "$output" != *"med"* ]]
}

@test "due labels: long by default, short with --short or JOT_SHORT" {
    jot add --due today      Must now     >/dev/null
    jot add --due tomorrow   Can wait     >/dev/null
    jot add --due 2020-01-01 Long overdue >/dev/null

    # Long default
    run jot
    [[ "$output" == *"today"*    ]]
    [[ "$output" == *"tomorrow"* ]]
    [[ "$output" == *"overdue"*  ]]

    # --short flag
    run jot --short
    [[ "$output" == *"tod"* ]]
    [[ "$output" == *"tmw"* ]]
    [[ "$output" != *"today"* ]]
    [[ "$output" != *"tomorrow"* ]]

    # JOT_SHORT env var
    JOT_SHORT=1 run jot
    [[ "$output" == *"tod"* ]]
    [[ "$output" != *"today"* ]]
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

@test "show: detail view renders all fields" {
    jot add -p high --due today --tag shopping \
        --description "Organic veggies and fresh bread" \
        Buy groceries >/dev/null

    run jot show 1
    [ "$status" -eq 0 ]
    # Top band carries both IDs.
    [[ "$output" == *"#1"*"TODO-0001-"* ]]
    # Top band carries classification + tags.
    [[ "$output" == *"Prio:"*"high"*       ]]
    [[ "$output" == *"Tags:"*"shopping"*   ]]
    # Middle band carries title + due + description.
    [[ "$output" == *"Title:"*"Buy groceries"* ]]
    [[ "$output" == *"Due:"*"today"*           ]]
    [[ "$output" == *"Description:"*"Organic"* ]]
    # Bottom band carries timestamps.
    [[ "$output" == *"Created:"* ]]
    [[ "$output" == *"Updated:"* ]]
}

@test "show: minimal task omits empty sections" {
    jot add Plain task >/dev/null
    run jot show 1
    [ "$status" -eq 0 ]
    [[ "$output" == *"Plain task"* ]]
    [[ "$output" != *"Priority:"*    ]]
    [[ "$output" != *"Due:"*         ]]
    [[ "$output" != *"Tags:"*        ]]
    [[ "$output" != *"Description:"* ]]
    [[ "$output" != *"Assignees:"*   ]]
}

@test "show: missing ID errors" {
    run jot show 999
    [ "$status" -ne 0 ]
    [[ "$output" == *"task not found"* ]]
}

@test "edit: changes title, priority, due, tags, description" {
    jot add --tag initial Buy stuff >/dev/null

    run jot edit 1 \
        --title "Buy organic stuff" \
        --priority critical \
        --due tomorrow \
        --add-tag urgent \
        --remove-tag initial \
        --description "Re-scoped for quality"
    [ "$status" -eq 0 ]
    [[ "$output" == *"updated"*"#1"*"Buy organic stuff"*"critical"*"tomorrow"* ]]

    run jot show 1
    [[ "$output" == *"Title:"*"Buy organic stuff"* ]]
    [[ "$output" == *"Prio:"*"critical"*           ]]
    [[ "$output" == *"Due:"*"tomorrow"*            ]]
    [[ "$output" == *"Tags:"*"urgent"*             ]]
    [[ "$output" != *"initial"*                    ]]
    [[ "$output" == *"Description:"*"Re-scoped"*   ]]

    # Filename follows the new title slug, not the old one.
    ls .jot/items/TODO-0001-*organic-stuff*.yaml >/dev/null
    ! ls .jot/items/TODO-0001-*-buy-stuff*.yaml 2>/dev/null
}

@test "edit: --no-due and --no-description clear fields" {
    jot add --due today --description "some text" Buy milk >/dev/null

    run jot edit 1 --no-due --no-description
    [ "$status" -eq 0 ]

    run jot show 1
    [[ "$output" != *"Due:"*         ]]
    [[ "$output" != *"Description:"* ]]
}

@test "assign: adds and removes assignees" {
    jot add Buy milk >/dev/null

    run jot assign 1 horst@example.com
    [ "$status" -eq 0 ]
    [[ "$output" == *"assigned"*"#1"*"horst@example.com"* ]]

    run jot show 1
    [[ "$output" == *"Assignee:"*"horst@example.com"* ]]

    # Edit adds a second assignee.
    run jot edit 1 --assign claude@joy
    [ "$status" -eq 0 ]
    run jot show 1
    [[ "$output" == *"horst@example.com"* ]]
    [[ "$output" == *"claude@joy"*        ]]

    # Unassign removes one but keeps the other.
    run jot edit 1 --unassign horst@example.com
    run jot show 1
    [[ "$output" != *"horst@example.com"* ]]
    [[ "$output" == *"claude@joy"*        ]]
}

@test "description flag: --desc, -d, and --description alias all accepted" {
    run jot add -d "short form" first
    [ "$status" -eq 0 ]
    run jot show 1
    [[ "$output" == *"short form"* ]]

    run jot add --desc "long form" second
    [ "$status" -eq 0 ]
    run jot show 2
    [[ "$output" == *"long form"* ]]

    run jot add --description "alias form" third
    [ "$status" -eq 0 ]
    run jot show 3
    [[ "$output" == *"alias form"* ]]

    yaml=$(ls .jot/items/TODO-0001-*.yaml)
    grep -qF "description: short form" "$yaml"
}

@test "ls: DESC column shows character count left of ASSIGNEE" {
    jot add -d "abcde" five >/dev/null
    jot add -d "0123456789012345" sixteen >/dev/null
    jot add plain >/dev/null
    run jot
    [ "$status" -eq 0 ]
    header=$(echo "$output" | grep -E "^ID")
    [[ "$header" == *"TITLE"*"DESC"* ]]
    # Counts appear (chars, not including the quotes).
    [[ "$output" == *"5"*  ]]
    [[ "$output" == *"16"* ]]
}

@test "ls: DESC column hidden when no task has a description" {
    jot add just a plain task >/dev/null
    run jot
    [[ "$output" != *"DESC"* ]]
}

@test "close: marks task Closed and records timestamp; done is an alias" {
    jot add first >/dev/null
    jot add second >/dev/null

    run jot close 1
    [ "$status" -eq 0 ]
    [[ "$output" == *"closed"*"#1"*"first"* ]]

    # 'done' alias reaches the same handler.
    run jot done 2
    [ "$status" -eq 0 ]
    [[ "$output" == *"closed"*"#2"*"second"* ]]

    # Status and timestamp land on disk.
    yaml=$(ls .jot/items/TODO-0001-*.yaml)
    grep -q "status: closed" "$yaml"
    grep -q "closed_at:"     "$yaml"

    # show footer carries Closed:
    run jot show 1
    [[ "$output" == *"Closed:"* ]]
}

@test "reopen: clears Closed status and closed_at" {
    jot add task >/dev/null
    jot close 1 >/dev/null

    run jot reopen 1
    [ "$status" -eq 0 ]
    yaml=$(ls .jot/items/TODO-0001-*.yaml)
    grep -q "status: new" "$yaml"
    ! grep -q "closed_at:" "$yaml"

    run jot show 1
    [[ "$output" != *"Closed:"* ]]
}

@test "archive: hides by default, --all reveals, --archived isolates" {
    jot add active one >/dev/null
    jot add to hide >/dev/null
    jot archive 2 >/dev/null

    # Default ls hides archived.
    run jot
    [[ "$output" == *"active one"* ]]
    [[ "$output" != *"to hide"*    ]]
    [[ "$output" == *"1 task"*     ]]

    # --all surfaces archived with strikethrough styling (in color mode).
    run jot ls --all
    [[ "$output" == *"active one"* ]]
    [[ "$output" == *"to hide"*    ]]

    # --archived shows only archived.
    run jot ls --archived
    [[ "$output" != *"active one"* ]]
    [[ "$output" == *"to hide"*    ]]

    # show footer carries Archived:
    run jot show 2
    [[ "$output" == *"Archived:"* ]]
}

@test "unarchive: restores a task to the default list" {
    jot add revived item >/dev/null
    jot archive 1 >/dev/null
    run jot
    [[ "$output" != *"revived item"* ]]
    [[ "$output" == *"No open tasks"* ]]

    run jot unarchive 1
    [ "$status" -eq 0 ]
    run jot
    [[ "$output" == *"revived item"* ]]

    # archived_at cleared on disk.
    yaml=$(ls .jot/items/TODO-0001-*.yaml)
    ! grep -q "archived_at:" "$yaml"
    ! grep -q "^archived: true" "$yaml"
}

@test "closed/archived render with ANSI strikethrough when colors on" {
    jot add active >/dev/null
    jot add close this >/dev/null
    jot add archive this >/dev/null
    jot close 2 >/dev/null
    jot archive 3 >/dev/null

    # ESC[9m is the strikethrough code.
    run jot --color=always ls --all
    [[ "$output" == *$'\x1b[9m'* ]]
}

@test "prefix shortcuts: any unambiguous subcommand prefix is accepted" {
    jot ad "prefix add" >/dev/null
    run jot l
    [[ "$output" == *"prefix add"* ]]

    run jot c 1
    [ "$status" -eq 0 ]
    [[ "$output" == *"closed"* ]]

    run jot re 1
    [ "$status" -eq 0 ]
    [[ "$output" == *"reopened"* ]]

    run jot ar 1
    [ "$status" -eq 0 ]
    [[ "$output" == *"archived"* ]]

    # 'u' is unambiguous (only 'unarchive' starts with it).
    run jot u 1
    [ "$status" -eq 0 ]
    [[ "$output" == *"unarchived"* ]]

    # 'r' is ambiguous (reopen vs rm) -- clap errors cleanly.
    run jot r 1
    [ "$status" -ne 0 ]
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
