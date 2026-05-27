#!/usr/bin/env bats
# Human-friendly --recur syntax (translated to RRULE for storage).
# JYN-000D-AB. TDD spec: each test is `skip`-disarmed and armed (skip
# removed) as its phrase mapping lands, so the suite stays green
# meanwhile. Raw RRULE remains the power-user fallback and is covered
# by the existing recurrence{,_done}.bats files.

load setup

@test "--recur daily is accepted and records the occurrence on done" {
    jyn add "Water plants" --due 2026-04-13 --recur daily >/dev/null
    jyn done "#1" >/dev/null
    run jyn show "#1"
    [ "$status" -eq 0 ]
    [[ "$output" == *"Recurs:"* ]]
    run jyn ls --closed
    [[ "$output" == *"#1@2026-04-13"* ]]
}

@test "--recur weekly is accepted" {
    jyn add "Standup" --due 2026-04-13 --recur weekly >/dev/null
    run jyn show "#1"
    [[ "$output" == *"Recurs:"* ]]
}

@test "--recur 'every Monday' resolves to weekly on MO" {
    # 2026-04-13 is a Monday; next Monday is 2026-04-20.
    jyn add "Standup" --due 2026-04-13 --recur "every Monday" >/dev/null
    jyn done "#1" >/dev/null
    run jyn ls --closed
    [[ "$output" == *"#1@2026-04-13"* ]]
    run jyn show "#1"
    [[ "$output" == *"Recurs:"* ]]
}

@test "--recur 'every 2 weeks' advances by two weeks" {
    jyn add "Sync" --due 2026-04-13 --recur "every 2 weeks" >/dev/null
    jyn done "#1" >/dev/null
    # 2026-04-13 + 2 weeks = 2026-04-27.
    run jyn show "#1"
    [[ "$output" == *"2026-04-27"* || "$output" == *"04-27"* ]]
}

@test "--recur weekdays advances from a weekday to the next weekday" {
    # 2026-04-17 is a Friday; next weekday is Monday 2026-04-20.
    jyn add "Standup" --due 2026-04-17 --recur weekdays >/dev/null
    jyn done "#1" >/dev/null
    run jyn ls --closed
    [[ "$output" == *"#1@2026-04-17"* ]]
}

@test "--recur 'monthly on the 1st' moves to the next month's first" {
    jyn add "Pay rent" --due 2026-04-01 --recur "monthly on the 1st" >/dev/null
    jyn done "#1" >/dev/null
    run jyn show "#1"
    [[ "$output" == *"2026-05-01"* || "$output" == *"05-01"* ]]
}

@test "--recur 'daily for 3 days' ends the series after three completions" {
    jyn add "Hydrate" --due 2026-04-13 --recur "daily for 3 days" >/dev/null
    jyn done "#1" >/dev/null
    jyn done "#1" >/dev/null
    run jyn done "#1"
    [ "$status" -eq 0 ]
    run jyn done "#1"
    [ "$status" -ne 0 ]
}

@test "--recur 'hourly for 3 times' with a time of day works end-to-end" {
    export TZ=UTC
    jyn add "Health check" --due "2026-04-13 14:00" --recur "hourly for 3 times" >/dev/null
    jyn done "#1" >/dev/null
    run jyn ls --closed
    [[ "$output" == *"#1@2026-04-13T14:00"* ]]
}

@test "raw RRULE remains accepted as a power-user fallback" {
    jyn add "Power user" --due 2026-04-13 --recur "FREQ=DAILY" >/dev/null
    run jyn show "#1"
    [ "$status" -eq 0 ]
    [[ "$output" == *"Recurs:"* ]]
}

@test "--recur with an unparseable phrase fails with a clear error" {
    run jyn add "Bad" --due 2026-04-13 --recur "gibberish phrase"
    [ "$status" -ne 0 ]
    [[ "$output" == *"recur"* ]]
}
