#!/usr/bin/env bats
# End-to-end user journey for the minimal jyn CLI.
# Covers: capture -> list -> remove, the no-args-equals-ls shortcut,
# and JOT-002F-4D short-form ID display with adaptive disambiguation.

load setup

@test "fresh workspace: 'jyn' with no args shows welcome block" {
    run jyn
    [ "$status" -eq 0 ]
    [[ "$output" == *"No tasks yet"* ]]
    [[ "$output" == *"jyn add"* ]]
}

@test "user journey: add, list, remove one task" {
    # Capture
    run jyn add "Buy milk"
    [ "$status" -eq 0 ]
    [[ "$output" == *"added"*"#1"*"Buy milk"* ]]

    # YAML on disk uses full ADR-027 ID
    [ -d ".jyn/items" ]
    ls .jyn/items/*.yaml | grep -q "TODO-0001-.*-buy-milk"

    # List (explicit) shows short form and table frame
    run jyn ls
    [ "$status" -eq 0 ]
    [[ "$output" == *"ID"*"TITLE"* ]]
    [[ "$output" == *"#1"* ]]
    [[ "$output" == *"Buy milk"* ]]
    [[ "$output" == *"1 task"* ]]
    [[ "$output" != *"TODO-"* ]]

    # Default (no subcommand) matches ls
    run jyn
    [ "$status" -eq 0 ]
    [[ "$output" == *"Buy milk"* ]]

    # Remove by short form
    run jyn rm "#1"
    [ "$status" -eq 0 ]
    [[ "$output" == *"removed"*"#1"*"Buy milk"* ]]

    # Empty again -- last task removed, welcome block returns
    run jyn
    [ "$status" -eq 0 ]
    [[ "$output" == *"No tasks yet"* ]]
}

@test "multiple tasks: counters increment, leading zeros stripped" {
    jyn add "First"    >/dev/null
    jyn add "Second"   >/dev/null
    jyn add "Third"    >/dev/null

    run jyn ls
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
    run jyn add this is a test
    [ "$status" -eq 0 ]
    [[ "$output" == *"#1"*"this is a test"* ]]

    # Quoting still works for titles that need it (e.g. shell metacharacters).
    run jyn add "Review PR 42"
    [ "$status" -eq 0 ]
    [[ "$output" == *"#2"*"Review PR 42"* ]]

    # Listing shows both with their original text intact.
    run jyn
    [[ "$output" == *"this is a test"* ]]
    [[ "$output" == *"Review PR 42"*  ]]
}

@test "rm accepts multiple input forms" {
    jyn add "Short form"   >/dev/null
    jyn add "Bare hex"     >/dev/null
    jyn add "Full ADR-027" >/dev/null

    # Short display form with hash
    run jyn rm "#1"
    [ "$status" -eq 0 ]
    [[ "$output" == *"Short form"* ]]

    # Bare counter (no leading hash)
    run jyn rm 2
    [ "$status" -eq 0 ]
    [[ "$output" == *"Bare hex"* ]]

    # Short ADR-027 form
    run jyn rm TODO-0003
    [ "$status" -eq 0 ]
    [[ "$output" == *"Full ADR-027"* ]]
}

@test "rm of missing ID errors out cleanly" {
    run jyn rm "#999"
    [ "$status" -ne 0 ]
    [[ "$output" == *"task not found"* ]]
}

@test "add auto-creates .jyn/items/ on first use" {
    [ ! -d ".jyn" ]
    jyn add "Bootstrap" >/dev/null
    [ -d ".jyn/items" ]
    ls .jyn/items/*.yaml | grep -q "bootstrap"
}

@test "counters survive deletion gaps (no reuse)" {
    jyn add "Alpha" >/dev/null
    jyn add "Bravo" >/dev/null
    jyn rm "#1" >/dev/null
    run jyn add "Charlie"
    [ "$status" -eq 0 ]
    # Highest existing counter is 2, so next is 3 (never reuses 1).
    [[ "$output" == *"#3"* ]]
}

@test "add: flags may follow the title words" {
    # Flag after bare multi-word title.
    run jyn add this is cool --due today
    [ "$status" -eq 0 ]
    [[ "$output" == *"this is cool"*"today"* ]]
    [[ "$output" != *"--due today"*              ]]

    # Flag after quoted title plus another flag.
    run jyn add "this is also cool" --due today --tag work
    [ "$status" -eq 0 ]
    [[ "$output" == *"this is also cool"*"today"*"#work"* ]]

    # Flag, title, flag.
    run jyn add --due today my chore --tag home
    [ "$status" -eq 0 ]
    [[ "$output" == *"my chore"*"today"*"#home"* ]]

    # Titles land in YAML without the flag tokens.
    ! grep -RlF -- "--due today" .jyn/items
}

@test "add with --due, --priority, --tag shows in ls and on disk" {
    run jyn add --due today --tag work --priority high Review PR 42
    [ "$status" -eq 0 ]
    [[ "$output" == *"added"*"#1"*"Review PR 42"*"today"*"#work"* ]]

    # YAML carries all fields.
    yaml=$(ls .jyn/items/TODO-0001-*.yaml)
    grep -q "priority: high" "$yaml"
    grep -q "due_date:"      "$yaml"
    grep -q "^tags:"         "$yaml"
    grep -qF -- "- work"     "$yaml"

    # ls grows DUE and TAGS columns only when data is present. Tags are
    # rendered without '#' in the table.
    run jyn
    [[ "$output" == *"DUE"*   ]]
    [[ "$output" == *"TAGS"*  ]]
    [[ "$output" == *"today"* ]]
    [[ "$output" == *"work"*  ]]
    [[ "$output" != *"#work"* ]]
}

@test "ls without extras stays a two-column table" {
    jyn add Plain task >/dev/null
    run jyn
    [[ "$output" == *"ID"*"TITLE"* ]]
    [[ "$output" != *"DUE"*  ]]
    [[ "$output" != *"TAGS"* ]]
    [[ "$output" != *"ASSIGNEE"* ]]
    [[ "$output" != *"PRIORITY"* ]]
    [[ "$output" != *"PRIO"* ]]
}

@test "ls columns: TAGS rightmost, ASSIGNEE to its left, no '#' prefix" {
    jyn add --tag work --tag urgent -a claude@joy Review PR >/dev/null
    run jyn
    [ "$status" -eq 0 ]
    # Header lists TITLE before ASSIGNEE before TAGS.
    header=$(echo "$output" | grep -E "^ID")
    [[ "$header" == *"TITLE"*"ASSIGNEE"*"TAGS"* ]]
    # Tags rendered without '#'.
    row=$(echo "$output" | grep "Review PR")
    [[ "$row" == *"claude@joy"*"work urgent"* ]]
    [[ "$row" != *"#work"* ]]
}

@test "due rendering: full year on ISO dates so 2028 and 2029 differ" {
    jyn add --due 2028-06-15 same month 2026 >/dev/null
    jyn add --due 2029-06-15 same month 2027 >/dev/null
    run jyn
    [[ "$output" == *"2028-06-15"* ]]
    [[ "$output" == *"2029-06-15"* ]]
}

@test "due input: MM-DD and DD.MM shortcuts are accepted" {
    run jyn add --due 04-30 near a
    [ "$status" -eq 0 ]
    run jyn add --due 25.04 deutsch b
    [ "$status" -eq 0 ]
    run jyn add --due 25.04.2027 deutsch c
    [ "$status" -eq 0 ]
    run jyn
    [[ "$output" == *"near a"*    ]]
    [[ "$output" == *"deutsch b"* ]]
    [[ "$output" == *"deutsch c"* ]]
}

@test "short flag: -a works as --assign on add and edit" {
    run jyn add -a horst@example.com Buy milk
    [ "$status" -eq 0 ]
    run jyn show 1
    [[ "$output" == *"horst@example.com"* ]]

    run jyn edit 1 -a claude@joy
    [ "$status" -eq 0 ]
    run jyn show 1
    [[ "$output" == *"horst@example.com"* ]]
    [[ "$output" == *"claude@joy"* ]]
}

@test "priority column: long by default, short with --short" {
    jyn add Plain task >/dev/null
    run jyn
    [[ "$output" != *"PRIORITY"* ]]
    [[ "$output" != *"PRIO"* ]]

    # Long default: full spelling, header 'PRIORITY'.
    jyn add -p extreme ohohooo >/dev/null
    run jyn
    [[ "$output" == *"PRIORITY"* ]]
    [[ "$output" == *"extreme"*  ]]

    # --short: three-letter labels, header 'PRIO'.
    run jyn --short
    [[ "$output" == *"PRIO"* ]]
    [[ "$output" != *"PRIORITY"* ]]
    [[ "$output" == *"ext"* ]]
    [[ "$output" != *"extreme"* ]]

    # Status lines mirror the mode.
    run jyn add -p high fix a bug
    [[ "$output" == *"fix a bug"*"high"* ]]
    run jyn --short add -p critical urgent cleanup
    [[ "$output" == *"urgent cleanup"*"crt"* ]]

    # Medium stays silent either way.
    run jyn add plain task
    [[ "$output" != *"medium"* ]]
    run jyn --short add plain task
    [[ "$output" != *"med"* ]]
}

@test "due labels: long by default, short with --short or JYN_SHORT" {
    jyn add --due today      Must now     >/dev/null
    jyn add --due tomorrow   Can wait     >/dev/null
    jyn add --due 2020-01-01 Long overdue >/dev/null

    # Long default
    run jyn
    [[ "$output" == *"today"*    ]]
    [[ "$output" == *"tomorrow"* ]]
    [[ "$output" == *"overdue"*  ]]

    # --short flag
    run jyn --short
    [[ "$output" == *"tod"* ]]
    [[ "$output" == *"tmw"* ]]
    [[ "$output" != *"today"* ]]
    [[ "$output" != *"tomorrow"* ]]

    # JYN_SHORT env var
    JYN_SHORT=1 run jyn
    [[ "$output" == *"tod"* ]]
    [[ "$output" != *"today"* ]]
}

@test "ls --tag filters tasks by tag (AND semantics for multiple flags)" {
    jyn add --tag work --tag urgent Fix prod outage >/dev/null
    jyn add --tag home Wash the car >/dev/null
    jyn add --tag work Review PR >/dev/null

    run jyn ls --tag work
    [[ "$output" == *"Fix prod outage"* ]]
    [[ "$output" == *"Review PR"*       ]]
    [[ "$output" != *"Wash the car"*    ]]

    # AND semantics: --tag work --tag urgent only matches tasks with both.
    run jyn ls --tag work --tag urgent
    [[ "$output" == *"Fix prod outage"* ]]
    [[ "$output" != *"Review PR"*       ]]
}

@test "ls --due today includes overdue, excludes future" {
    jyn add --due today     Must do now      >/dev/null
    jyn add --due tomorrow  Can wait         >/dev/null
    jyn add --due 2020-01-01 Ancient overdue >/dev/null
    jyn add                  Undated         >/dev/null

    run jyn ls --due today
    [[ "$output" == *"Must do now"*      ]]
    [[ "$output" == *"Ancient overdue"*  ]]
    [[ "$output" != *"Can wait"*         ]]
    [[ "$output" != *"Undated"*          ]]
}

@test "add rejects unknown --due values" {
    run jyn add --due friday Book dentist
    [ "$status" -ne 0 ]
    [[ "$output" == *"cannot parse due date"* ]]
}

@test "--color=never produces no ANSI escape codes" {
    jyn add "Buy milk" >/dev/null
    run jyn --color=never
    [ "$status" -eq 0 ]
    # No ESC (0x1b) bytes anywhere in the output.
    [[ "$output" != *$'\x1b'* ]]
}

@test "--color=always emits ANSI codes even when piped" {
    jyn add "Buy milk" >/dev/null
    run jyn --color=always
    [ "$status" -eq 0 ]
    # At least one ESC byte present.
    [[ "$output" == *$'\x1b'* ]]
}

@test "NO_COLOR env var disables colors without --color flag" {
    jyn add "Buy milk" >/dev/null
    NO_COLOR=1 run jyn
    [ "$status" -eq 0 ]
    [[ "$output" != *$'\x1b'* ]]
}

@test "show: detail view renders all fields" {
    jyn add -p high --due today --tag shopping \
        --description "Organic veggies and fresh bread" \
        Buy groceries >/dev/null

    run jyn show 1
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
    jyn add Plain task >/dev/null
    run jyn show 1
    [ "$status" -eq 0 ]
    [[ "$output" == *"Plain task"* ]]
    [[ "$output" != *"Priority:"*    ]]
    [[ "$output" != *"Due:"*         ]]
    [[ "$output" != *"Tags:"*        ]]
    [[ "$output" != *"Description:"* ]]
    [[ "$output" != *"Assignees:"*   ]]
}

@test "show: missing ID errors" {
    run jyn show 999
    [ "$status" -ne 0 ]
    [[ "$output" == *"task not found"* ]]
}

@test "edit: changes title, priority, due, tags, description" {
    jyn add --tag initial Buy stuff >/dev/null

    run jyn edit 1 \
        --title "Buy organic stuff" \
        --priority critical \
        --due tomorrow \
        --add-tag urgent \
        --remove-tag initial \
        --description "Re-scoped for quality"
    [ "$status" -eq 0 ]
    [[ "$output" == *"updated"*"#1"*"Buy organic stuff"*"critical"*"tomorrow"* ]]

    run jyn show 1
    [[ "$output" == *"Title:"*"Buy organic stuff"* ]]
    [[ "$output" == *"Prio:"*"critical"*           ]]
    [[ "$output" == *"Due:"*"tomorrow"*            ]]
    [[ "$output" == *"Tags:"*"urgent"*             ]]
    [[ "$output" != *"initial"*                    ]]
    [[ "$output" == *"Description:"*"Re-scoped"*   ]]

    # Filename follows the new title slug, not the old one.
    ls .jyn/items/TODO-0001-*organic-stuff*.yaml >/dev/null
    ! ls .jyn/items/TODO-0001-*-buy-stuff*.yaml 2>/dev/null
}

@test "edit: --no-due and --no-description clear fields" {
    jyn add --due today --description "some text" Buy milk >/dev/null

    run jyn edit 1 --no-due --no-description
    [ "$status" -eq 0 ]

    run jyn show 1
    [[ "$output" != *"Due:"*         ]]
    [[ "$output" != *"Description:"* ]]
}

@test "assign: adds and removes assignees" {
    jyn add Buy milk >/dev/null

    run jyn assign 1 horst@example.com
    [ "$status" -eq 0 ]
    [[ "$output" == *"assigned"*"#1"*"horst@example.com"* ]]

    run jyn show 1
    [[ "$output" == *"Assignee:"*"horst@example.com"* ]]

    # Edit adds a second assignee.
    run jyn edit 1 --assign claude@joy
    [ "$status" -eq 0 ]
    run jyn show 1
    [[ "$output" == *"horst@example.com"* ]]
    [[ "$output" == *"claude@joy"*        ]]

    # Unassign removes one but keeps the other.
    run jyn edit 1 --unassign horst@example.com
    run jyn show 1
    [[ "$output" != *"horst@example.com"* ]]
    [[ "$output" == *"claude@joy"*        ]]
}

@test "description flag: --desc, -d, and --description alias all accepted" {
    run jyn add -d "short form" first
    [ "$status" -eq 0 ]
    run jyn show 1
    [[ "$output" == *"short form"* ]]

    run jyn add --desc "long form" second
    [ "$status" -eq 0 ]
    run jyn show 2
    [[ "$output" == *"long form"* ]]

    run jyn add --description "alias form" third
    [ "$status" -eq 0 ]
    run jyn show 3
    [[ "$output" == *"alias form"* ]]

    yaml=$(ls .jyn/items/TODO-0001-*.yaml)
    grep -qF "description: short form" "$yaml"
}

@test "ls: DESC column shows character count left of ASSIGNEE" {
    jyn add -d "abcde" five >/dev/null
    jyn add -d "0123456789012345" sixteen >/dev/null
    jyn add plain >/dev/null
    run jyn
    [ "$status" -eq 0 ]
    header=$(echo "$output" | grep -E "^ID")
    [[ "$header" == *"TITLE"*"DESC"* ]]
    # Counts appear (chars, not including the quotes).
    [[ "$output" == *"5"*  ]]
    [[ "$output" == *"16"* ]]
}

@test "ls: DESC column hidden when no task has a description" {
    jyn add just a plain task >/dev/null
    run jyn
    [[ "$output" != *"DESC"* ]]
}

@test "close: marks task Closed and records timestamp; done is an alias" {
    jyn add first >/dev/null
    jyn add second >/dev/null

    run jyn close 1
    [ "$status" -eq 0 ]
    [[ "$output" == *"closed"*"#1"*"first"* ]]

    # 'done' alias reaches the same handler.
    run jyn done 2
    [ "$status" -eq 0 ]
    [[ "$output" == *"closed"*"#2"*"second"* ]]

    # Status and timestamp land on disk.
    yaml=$(ls .jyn/items/TODO-0001-*.yaml)
    grep -q "status: closed" "$yaml"
    grep -q "closed_at:"     "$yaml"

    # show footer carries Closed:
    run jyn show 1
    [[ "$output" == *"Closed:"* ]]
}

@test "reopen: clears Closed status and closed_at" {
    jyn add task >/dev/null
    jyn close 1 >/dev/null

    run jyn reopen 1
    [ "$status" -eq 0 ]
    yaml=$(ls .jyn/items/TODO-0001-*.yaml)
    grep -q "status: new" "$yaml"
    ! grep -q "closed_at:" "$yaml"

    run jyn show 1
    [[ "$output" != *"Closed:"* ]]
}

@test "archive: hides by default, --all reveals, --archived isolates" {
    jyn add active one >/dev/null
    jyn add to hide >/dev/null
    jyn archive 2 >/dev/null

    # Default ls hides archived.
    run jyn
    [[ "$output" == *"active one"* ]]
    [[ "$output" != *"to hide"*    ]]
    [[ "$output" == *"1 task"*     ]]

    # --all surfaces archived with strikethrough styling (in color mode).
    run jyn ls --all
    [[ "$output" == *"active one"* ]]
    [[ "$output" == *"to hide"*    ]]

    # --archived shows only archived.
    run jyn ls --archived
    [[ "$output" != *"active one"* ]]
    [[ "$output" == *"to hide"*    ]]

    # show footer carries Archived:
    run jyn show 2
    [[ "$output" == *"Archived:"* ]]
}

@test "unarchive: restores a task to the default list" {
    jyn add revived item >/dev/null
    jyn archive 1 >/dev/null
    run jyn
    [[ "$output" != *"revived item"* ]]
    [[ "$output" == *"No open tasks"* ]]

    run jyn unarchive 1
    [ "$status" -eq 0 ]
    run jyn
    [[ "$output" == *"revived item"* ]]

    # archived_at cleared on disk.
    yaml=$(ls .jyn/items/TODO-0001-*.yaml)
    ! grep -q "archived_at:" "$yaml"
    ! grep -q "^archived: true" "$yaml"
}

@test "closed/archived render with ANSI strikethrough when colors on" {
    jyn add active >/dev/null
    jyn add close this >/dev/null
    jyn add archive this >/dev/null
    jyn close 2 >/dev/null
    jyn archive 3 >/dev/null

    # ESC[9m is the strikethrough code.
    run jyn --color=always ls --all
    [[ "$output" == *$'\x1b[9m'* ]]
}

@test "ls default sort: overdue, today, soon, later, no-date; then priority; then creation" {
    jyn add -p low               a-nodate-low              >/dev/null
    jyn add -p extreme           b-nodate-extreme          >/dev/null
    jyn add --due 2020-01-01     c-overdue                 >/dev/null
    jyn add --due today          d-today-default           >/dev/null
    jyn add --due today -p high  e-today-high              >/dev/null
    jyn add --due tomorrow       f-tomorrow                >/dev/null
    jyn add --due 2030-01-01     g-later                   >/dev/null
    jyn add                      h-will-be-closed          >/dev/null
    jyn close 8                                            >/dev/null

    run jyn
    [ "$status" -eq 0 ]
    rows=$(echo "$output" | grep -E "^#")
    # The eighth and final row must be the closed one.
    last_row=$(echo "$rows" | tail -1)
    [[ "$last_row" == *"h-will-be-closed"* ]]

    # First row = overdue (strongest urgency).
    first_row=$(echo "$rows" | sed -n '1p')
    [[ "$first_row" == *"c-overdue"* ]]

    # Second row = today with higher priority before today with default.
    second=$(echo "$rows" | sed -n '2p')
    third=$(echo "$rows" | sed -n '3p')
    [[ "$second" == *"e-today-high"*    ]]
    [[ "$third"  == *"d-today-default"* ]]

    # Within no-date bucket, extreme > low.
    # Find the position of the two no-date rows.
    ex_pos=$(echo "$rows" | grep -n "b-nodate-extreme" | cut -d: -f1)
    lo_pos=$(echo "$rows" | grep -n "a-nodate-low"     | cut -d: -f1)
    [ "$ex_pos" -lt "$lo_pos" ]
}

@test "ls flags work at top level: jyn -a / jyn --sort / jyn --tag" {
    jyn add -p low    zebra     >/dev/null
    jyn add           alpha     >/dev/null
    jyn add --tag x   beta      >/dev/null
    jyn add to-hide             >/dev/null
    jyn archive 4               >/dev/null

    # jyn -a: archived surfaces.
    run jyn -a
    [[ "$output" == *"to-hide"* ]]

    # jyn --sort title: alphabetical.
    run jyn --sort title
    rows=$(echo "$output" | grep -E "^#")
    [[ $(echo "$rows" | sed -n '1p') == *"alpha"* ]]
    [[ $(echo "$rows" | sed -n '3p') == *"zebra"* ]]

    # jyn --tag x: tag filter.
    run jyn --tag x
    [[ "$output" == *"beta"*  ]]
    [[ "$output" != *"alpha"* ]]
}

@test "--sort: created / priority / due / title / --reverse" {
    jyn add -p low    zebra     >/dev/null
    jyn add           alpha     >/dev/null
    jyn add --due 2026-06-01 beta >/dev/null
    jyn add -p extreme charlie  >/dev/null

    # priority: extreme first, low last
    run jyn --sort priority
    rows=$(echo "$output" | grep -E "^#")
    [[ $(echo "$rows" | sed -n '1p') == *"charlie"* ]]
    [[ $(echo "$rows" | sed -n '4p') == *"zebra"*   ]]

    # title + reverse: z first
    run jyn --sort title -r
    rows=$(echo "$output" | grep -E "^#")
    [[ $(echo "$rows" | sed -n '1p') == *"zebra"* ]]

    # created: insertion order
    run jyn --sort created
    rows=$(echo "$output" | grep -E "^#")
    [[ $(echo "$rows" | sed -n '1p') == *"zebra"*  ]]
    [[ $(echo "$rows" | sed -n '4p') == *"charlie"* ]]

    # due: beta has a date, others fall to the end
    run jyn --sort due
    rows=$(echo "$output" | grep -E "^#")
    [[ $(echo "$rows" | sed -n '1p') == *"beta"* ]]
}

@test "prefix shortcuts: any unambiguous subcommand prefix is accepted" {
    jyn ad "prefix add" >/dev/null
    run jyn l
    [[ "$output" == *"prefix add"* ]]

    # 'c' is an explicit alias for close (close vs config prefix would
    # otherwise be ambiguous).
    run jyn c 1
    [ "$status" -eq 0 ]
    [[ "$output" == *"closed"* ]]

    run jyn re 1
    [ "$status" -eq 0 ]
    [[ "$output" == *"reopened"* ]]

    run jyn ar 1
    [ "$status" -eq 0 ]
    [[ "$output" == *"archived"* ]]

    # 'u' is unambiguous (only 'unarchive' starts with it).
    run jyn u 1
    [ "$status" -eq 0 ]
    [[ "$output" == *"unarchived"* ]]

    # 'r' is ambiguous (reopen vs rm) -- clap errors cleanly.
    run jyn r 1
    [ "$status" -ne 0 ]

    # 'co' is ambiguous (close vs config) -- clap errors cleanly.
    run jyn co 1
    [ "$status" -ne 0 ]
}

@test "ls hugs content width on wide terminals and truncates on narrow ones" {
    jyn add short >/dev/null
    jyn add another quick thing >/dev/null

    # Wide terminal: table should be ~32 chars (ID + TITLE with 5-space
    # buffer after the longest title), not stretched to 200.
    run env COLUMNS=200 jyn --color=never
    # Separator must be far shorter than 200.
    sep_len=$(echo "$output" | grep -m1 "^-" | awk '{print length}')
    [ "$sep_len" -lt 60 ]
    # No truncation with plenty of room.
    [[ "$output" != *"..."* ]]

    # Narrow terminal: title gets '...' plus 2-space buffer before the
    # next column (here no other columns, so just the truncation).
    jyn add "This title is far too long for a narrow 30-column terminal" >/dev/null
    run env COLUMNS=30 jyn --color=never
    [[ "$output" == *"..."* ]]
    # The separator on narrow mode runs the full terminal width.
    sep_len=$(echo "$output" | grep -m1 "^-" | awk '{print length}')
    [ "$sep_len" -eq 30 ]
}

@test "collision: same counter from two sources shows expanded form" {
    # Simulate a post-sync state: two YAML files with the same counter
    # but different title-hash suffixes (what would happen if two
    # devices concurrently created task 1 before sync).
    mkdir -p .jyn/items
    cat > .jyn/items/TODO-0001-EA-review-pr.yaml <<'YAML'
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
    cat > .jyn/items/TODO-0001-7F-call-mom.yaml <<'YAML'
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
    jyn add "Book dentist" >/dev/null

    run jyn ls
    [ "$status" -eq 0 ]
    # Colliding rows must show the suffix; the non-colliding row stays short.
    [[ "$output" == *"#1-EA"* ]]
    [[ "$output" == *"#1-7F"* ]]
    [[ "$output" == *"#2"*   ]]
    [[ "$output" != *"#2-"*  ]]

    # Ambiguous short input is rejected, but with a useful hint.
    run jyn rm "#1"
    [ "$status" -ne 0 ]
    [[ "$output" == *"ambiguous"* ]]

    # Disambiguated input works.
    run jyn rm "#1-EA"
    [ "$status" -eq 0 ]
    [[ "$output" == *"removed"* ]]

    # After removal the remaining #1 is unique again and displays short.
    run jyn ls
    [[ "$output" == *"#1"* ]]
    [[ "$output" != *"#1-"* ]]
}
