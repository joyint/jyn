#!/usr/bin/env bats
# Recurring task completion (date-only default case). JOT-000A / JOT-0006.
#
# TDD spec for the default CLI commands. Each assertion targets the new
# behaviour (per-occurrence history + an advancing series), so the whole
# file is red until JOT-000A/JOT-0006 are implemented. Edge cases
# (time-of-day, DST, skip) are out of scope for now per JYN-000A-B1.

load setup

@test "done records each completed occurrence and the series keeps advancing" {
    jyn add "Water plants" --due 2026-04-13 --recur "FREQ=DAILY" >/dev/null

    jyn done "#1" >/dev/null   # completes 2026-04-13, advances to 2026-04-14
    jyn done "#1" >/dev/null   # completes 2026-04-14, advances to 2026-04-15

    run jyn ls --closed
    [ "$status" -eq 0 ]
    [[ "$output" == *"#1@2026-04-13"* ]]
    [[ "$output" == *"#1@2026-04-14"* ]]
}

@test "show lists completed occurrences under a dedicated section" {
    jyn add "Water plants" --due 2026-04-13 --recur "FREQ=DAILY" >/dev/null
    jyn done "#1" >/dev/null

    run jyn show "#1"
    [ "$status" -eq 0 ]
    [[ "$output" == *"Completed occurrences"* ]]
}

@test "reopen addresses and removes a single occurrence by date" {
    jyn add "Water plants" --due 2026-04-13 --recur "FREQ=DAILY" >/dev/null
    jyn done "#1" >/dev/null

    run jyn reopen "#1@2026-04-13"
    [ "$status" -eq 0 ]

    run jyn ls --closed
    [ "$status" -eq 0 ]
    [[ "$output" != *"#1@2026-04-13"* ]]
}

@test "completing the final occurrence closes the series; further done fails" {
    jyn add "One last time" --due 2026-04-13 --recur "FREQ=DAILY;COUNT=1" >/dev/null

    run jyn done "#1"
    [ "$status" -eq 0 ]

    run jyn done "#1"
    [ "$status" -ne 0 ]
}

@test "hourly recurrence with an end advances by the hour, then closes" {
    # A time-of-day on --due makes the series sub-day (hourly); occurrences
    # are addressed with the time. TZ fixed so the displayed time is stable.
    export TZ=UTC
    jyn add "Health check" --due "2026-04-13 14:00" --recur "FREQ=HOURLY;COUNT=3" >/dev/null

    jyn done "#1" >/dev/null   # completes 14:00, advances to 15:00
    jyn done "#1" >/dev/null   # completes 15:00, advances to 16:00

    run jyn ls --closed
    [ "$status" -eq 0 ]
    [[ "$output" == *"#1@2026-04-13T14:00"* ]]
    [[ "$output" == *"#1@2026-04-13T15:00"* ]]

    run jyn done "#1"          # completes 16:00, exhausts COUNT=3, closes
    [ "$status" -eq 0 ]

    run jyn done "#1"          # nothing left to complete
    [ "$status" -ne 0 ]
}
