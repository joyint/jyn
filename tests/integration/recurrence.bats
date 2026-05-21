#!/usr/bin/env bats
# jyn --recur: set/clear an RRULE on a task. JYN-0009-01.

load setup

@test "add --recur stores a valid RRULE and show displays it" {
    run jyn add "Weekly standup" --recur "FREQ=WEEKLY;BYDAY=MO"
    [ "$status" -eq 0 ]
    [[ "$output" == *"FREQ=WEEKLY;BYDAY=MO"* ]]

    run jyn show "#1"
    [ "$status" -eq 0 ]
    [[ "$output" == *"Recurs:"* ]]
    [[ "$output" == *"FREQ=WEEKLY;BYDAY=MO"* ]]
}

@test "add --recur rejects an invalid RRULE" {
    run jyn add "Bad rule" --recur "FREQ=NONSENSE"
    [ "$status" -ne 0 ]
    [[ "$output" == *"invalid recurrence rule"* ]]
}

@test "edit --recur sets and --no-recur clears the recurrence" {
    jyn add "Standup" >/dev/null

    run jyn edit "#1" --recur "FREQ=DAILY"
    [ "$status" -eq 0 ]
    run jyn show "#1"
    [[ "$output" == *"Recurs:"*"FREQ=DAILY"* ]]

    run jyn edit "#1" --no-recur
    [ "$status" -eq 0 ]
    run jyn show "#1"
    [[ "$output" != *"Recurs:"* ]]
}
