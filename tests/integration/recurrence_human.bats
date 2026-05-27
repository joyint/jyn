#!/usr/bin/env bats
# Human-friendly --recur syntax (translated to RRULE for storage).
# JYN-000D-AB. TDD spec: each test is `skip`-disarmed and armed (skip
# removed) as its phrase mapping lands, so the suite stays green
# meanwhile. Raw RRULE remains the power-user fallback and is covered
# by the existing recurrence{,_done}.bats files.

load setup

@test "--recur daily is accepted and records the occurrence on done" {
    skip "arm when the human parser lands (daily/weekly/monthly/yearly)"
    jyn add "Water plants" --due 2026-04-13 --recur daily >/dev/null
    jyn done "#1" >/dev/null
    run jyn show "#1"
    [ "$status" -eq 0 ]
    [[ "$output" == *"Recurs:"* ]]
    run jyn ls --closed
    [[ "$output" == *"#1@2026-04-13"* ]]
}

@test "--recur weekly is accepted" {
    skip "arm when the human parser lands (daily/weekly/monthly/yearly)"
    jyn add "Standup" --due 2026-04-13 --recur weekly >/dev/null
    run jyn show "#1"
    [[ "$output" == *"Recurs:"* ]]
}

@test "--recur 'every Monday' resolves to weekly on MO" {
    skip "arm when 'every <weekday>' lands"
    # 2026-04-13 is a Monday; next Monday is 2026-04-20.
    jyn add "Standup" --due 2026-04-13 --recur "every Monday" >/dev/null
    jyn done "#1" >/dev/null
    run jyn ls --closed
    [[ "$output" == *"#1@2026-04-13"* ]]
    run jyn show "#1"
    [[ "$output" == *"Recurs:"* ]]
}

@test "--recur 'every 2 weeks' advances by two weeks" {
    skip "arm when 'every N <unit>' lands"
    jyn add "Sync" --due 2026-04-13 --recur "every 2 weeks" >/dev/null
    jyn done "#1" >/dev/null
    # 2026-04-13 + 2 weeks = 2026-04-27.
    run jyn show "#1"
    [[ "$output" == *"2026-04-27"* || "$output" == *"04-27"* ]]
}

@test "--recur weekdays advances from a weekday to the next weekday" {
    skip "arm when 'weekdays' alias lands"
    # 2026-04-17 is a Friday; next weekday is Monday 2026-04-20.
    jyn add "Standup" --due 2026-04-17 --recur weekdays >/dev/null
    jyn done "#1" >/dev/null
    run jyn ls --closed
    [[ "$output" == *"#1@2026-04-17"* ]]
}

@test "--recur 'monthly on the 1st' moves to the next month's first" {
    skip "arm when 'on the Nth' / monthly-day lands"
    jyn add "Pay rent" --due 2026-04-01 --recur "monthly on the 1st" >/dev/null
    jyn done "#1" >/dev/null
    run jyn show "#1"
    [[ "$output" == *"2026-05-01"* || "$output" == *"05-01"* ]]
}

@test "--recur 'daily for 3 days' ends the series after three completions" {
    skip "arm when 'for N <unit>' / 'for N times' (COUNT) lands"
    jyn add "Hydrate" --due 2026-04-13 --recur "daily for 3 days" >/dev/null
    jyn done "#1" >/dev/null
    jyn done "#1" >/dev/null
    run jyn done "#1"
    [ "$status" -eq 0 ]
    run jyn done "#1"
    [ "$status" -ne 0 ]
}

@test "--recur 'hourly for 3 times' with a time of day works end-to-end" {
    skip "arm when 'hourly' + 'for N times' on a time-bearing due land"
    export TZ=UTC
    jyn add "Health check" --due "2026-04-13 14:00" --recur "hourly for 3 times" >/dev/null
    jyn done "#1" >/dev/null
    run jyn ls --closed
    [[ "$output" == *"#1@2026-04-13T14:00"* ]]
}

@test "raw RRULE remains accepted as a power-user fallback" {
    skip "arm alongside the human parser (regression guard)"
    jyn add "Power user" --due 2026-04-13 --recur "FREQ=DAILY" >/dev/null
    run jyn show "#1"
    [ "$status" -eq 0 ]
    [[ "$output" == *"Recurs:"* ]]
}

@test "--recur with an unparseable phrase fails with a clear error" {
    skip "arm with the human parser"
    run jyn add "Bad" --due 2026-04-13 --recur "gibberish phrase"
    [ "$status" -ne 0 ]
    [[ "$output" == *"recur"* ]]
}
