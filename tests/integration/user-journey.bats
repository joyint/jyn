#!/usr/bin/env bats
# End-to-end user journey for the minimal jot CLI.
# Covers: capture -> list -> remove, plus the no-args-equals-ls shortcut.

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
    [[ "$output" == *"TODO-0001-"* ]]
    [[ "$output" == *"Buy milk"* ]]

    # YAML on disk
    [ -d ".jot/items" ]
    ls .jot/items/*.yaml | grep -q "buy-milk"

    # List (explicit)
    run jot ls
    [ "$status" -eq 0 ]
    [[ "$output" == *"Buy milk"* ]]

    # Default (no subcommand) matches ls
    run jot
    [ "$status" -eq 0 ]
    [[ "$output" == *"Buy milk"* ]]

    # Pick ID and remove it
    id=$(jot ls | awk 'NR==1{print $1}')
    run jot rm "$id"
    [ "$status" -eq 0 ]
    [[ "$output" == *"Removed"* ]]

    # Empty again
    run jot
    [ "$status" -eq 0 ]
    [[ "$output" == *"No open tasks"* ]]
}

@test "multiple tasks: IDs increment and list sorts by ID" {
    jot add "First"    >/dev/null
    jot add "Second"   >/dev/null
    jot add "Third"    >/dev/null

    run jot ls
    [ "$status" -eq 0 ]
    # Expect three lines in add-order (filenames sort by ID)
    first_line=$(echo "$output" | sed -n '1p')
    last_line=$(echo "$output"  | sed -n '3p')
    [[ "$first_line" == *"TODO-0001-"* ]]
    [[ "$first_line" == *"First"*       ]]
    [[ "$last_line"  == *"TODO-0003-"*  ]]
    [[ "$last_line"  == *"Third"*       ]]
}

@test "rm accepts short-form ID" {
    jot add "Short form" >/dev/null
    run jot rm TODO-0001
    [ "$status" -eq 0 ]
    [[ "$output" == *"Removed"* ]]
    [[ "$output" == *"Short form"* ]]
}

@test "rm of missing ID errors out cleanly" {
    run jot rm TODO-9999
    [ "$status" -ne 0 ]
    [[ "$output" == *"task not found"* ]]
}

@test "add auto-creates .jot/items/ on first use" {
    [ ! -d ".jot" ]
    jot add "Bootstrap" >/dev/null
    [ -d ".jot/items" ]
    ls .jot/items/*.yaml | grep -q "bootstrap"
}

@test "IDs survive deletion gaps (no reuse)" {
    jot add "Alpha" >/dev/null
    jot add "Bravo" >/dev/null
    jot rm TODO-0001 >/dev/null
    run jot add "Charlie"
    [ "$status" -eq 0 ]
    # Highest existing is 0002, so next is 0003 (never reuses 0001).
    [[ "$output" == *"TODO-0003-"* ]]
}
