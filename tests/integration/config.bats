#!/usr/bin/env bats
# Config system: jyn config get/set/list, --global/--local, strict schema.

load setup

setup() {
    TEST_DIR="$(mktemp -d)"
    # Scope XDG_CONFIG_HOME so tests never touch the real ~/.config/jyn.
    export XDG_CONFIG_HOME="$TEST_DIR/xdg"
    cd "$TEST_DIR" || exit 1
}

teardown() {
    cd /
    rm -rf "$TEST_DIR"
    unset XDG_CONFIG_HOME
}

@test "config get: returns default when nothing is set" {
    run jyn config get output.fortune
    [ "$status" -eq 0 ]
    [ "$output" = "true" ]
}

@test "config get: missing key exits 1 silently" {
    run jyn config get no.such.key
    [ "$status" -eq 1 ]
    [ -z "$output" ]
}

@test "config list: bare command shows merged view with [default] markers" {
    run jyn config
    [ "$status" -eq 0 ]
    [[ "$output" == *"fortune"* ]]
    [[ "$output" == *"[default]"* ]]
}

@test "config set: fails loudly when neither local nor global exists" {
    run jyn config set output.fortune false
    [ "$status" -ne 0 ]
    [[ "$output" == *"No config exists yet"* ]]
    [[ "$output" == *"--global"* ]]
    [[ "$output" == *"--local"* ]]
}

@test "config set --global: auto-creates ~/.config/jyn and writes" {
    run jyn config set --global output.fortune false
    [ "$status" -eq 0 ]
    [[ "$output" == *"[global]"* ]]
    [ -f "$XDG_CONFIG_HOME/jyn/config.yaml" ]

    run jyn config get output.fortune
    [ "$status" -eq 0 ]
    [ "$output" = "false" ]
}

@test "config set --local: auto-creates .jyn/ and writes" {
    run jyn config set --local output.fortune false
    [ "$status" -eq 0 ]
    [[ "$output" == *"[local]"* ]]
    [ -f "$PWD/.jyn/config.yaml" ]

    run jyn config get output.fortune
    [ "$status" -eq 0 ]
    [ "$output" = "false" ]
}

@test "config set: default picks local when .jyn/ exists in cwd" {
    # Seed a project by creating .jyn via any jyn write (add creates it).
    jyn add "seed" >/dev/null
    run jyn config set output.fortune false
    [ "$status" -eq 0 ]
    [[ "$output" == *"[local]"* ]]
    [ -f "$PWD/.jyn/config.yaml" ]
}

@test "config set: default picks global when .jyn/ absent but global exists" {
    # Prime the global file.
    jyn config set --global output.fortune true >/dev/null
    # Now implicit set: no .jyn/ here, global exists, should target global.
    run jyn config set output.fortune false
    [ "$status" -eq 0 ]
    [[ "$output" == *"[global]"* ]]
}

@test "config set: local overrides global" {
    jyn config set --global output.fortune true >/dev/null
    jyn add "seed" >/dev/null
    jyn config set --local output.fortune false >/dev/null

    run jyn config get output.fortune
    [ "$status" -eq 0 ]
    [ "$output" = "false" ]
}

@test "config set: strict schema rejects unknown key" {
    run jyn config set --global outpt.fortune false
    [ "$status" -ne 0 ]
    [[ "$output" == *"not a known config key"* || "$output" == *"wrong type"* ]]
}

@test "config set: type error carries a schema-derived hint" {
    run jyn config set --global output.fortune notabool
    [ "$status" -ne 0 ]
    [[ "$output" == *"not valid"* ]]
    [[ "$output" == *"true or false"* ]]
}

@test "config set: fortune-category accepts enum variant" {
    run jyn config set --global output.fortune-category tech
    [ "$status" -eq 0 ]

    run jyn config get output.fortune-category
    [ "$status" -eq 0 ]
    [ "$output" = "tech" ]
}

@test "config set: fortune-category rejects unknown variant" {
    run jyn config set --global output.fortune-category notacategory
    [ "$status" -ne 0 ]
    [[ "$output" == *"tech"* ]]
    [[ "$output" == *"humor"* ]]
}

@test "config set: --global and --local are mutually exclusive" {
    run jyn config set --global --local output.fortune false
    [ "$status" -ne 0 ]
}

@test "fortune opt-out: no fortune appears on stderr after set false" {
    jyn config set --global output.fortune false >/dev/null
    # Force a TTY by faking via script(1) so fortune's is_terminal check passes.
    # Run many times so a 20% probability would almost certainly surface at
    # least once if the gate were broken.
    # Detect util-linux vs BSD script: -ec is util-linux; BSD takes the
    # command after the file argument.
    if script --version 2>&1 | grep -q util-linux; then
        run_in_pty() { script -qec "$1" /dev/null; }
    else
        run_in_pty() { script -q /dev/null sh -c "$1"; }
    fi
    for _ in $(seq 1 20); do
        out="$(run_in_pty 'jyn add foo' 2>&1)"
        [[ "$out" != *$'\x1b[2m'* ]] || return 1
    done
}
