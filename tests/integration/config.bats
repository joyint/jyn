#!/usr/bin/env bats
# Config system: jot config get/set/list, --global/--local, strict schema.

load setup

setup() {
    TEST_DIR="$(mktemp -d)"
    # Scope XDG_CONFIG_HOME so tests never touch the real ~/.config/jot.
    export XDG_CONFIG_HOME="$TEST_DIR/xdg"
    cd "$TEST_DIR" || exit 1
}

teardown() {
    cd /
    rm -rf "$TEST_DIR"
    unset XDG_CONFIG_HOME
}

@test "config get: returns default when nothing is set" {
    run jot config get output.fortune
    [ "$status" -eq 0 ]
    [ "$output" = "true" ]
}

@test "config get: missing key exits 1 silently" {
    run jot config get no.such.key
    [ "$status" -eq 1 ]
    [ -z "$output" ]
}

@test "config list: bare command shows merged view with [default] markers" {
    run jot config
    [ "$status" -eq 0 ]
    [[ "$output" == *"fortune"* ]]
    [[ "$output" == *"[default]"* ]]
}

@test "config set: fails loudly when neither local nor global exists" {
    run jot config set output.fortune false
    [ "$status" -ne 0 ]
    [[ "$output" == *"No config exists yet"* ]]
    [[ "$output" == *"--global"* ]]
    [[ "$output" == *"--local"* ]]
}

@test "config set --global: auto-creates ~/.config/jot and writes" {
    run jot config set --global output.fortune false
    [ "$status" -eq 0 ]
    [[ "$output" == *"[global]"* ]]
    [ -f "$XDG_CONFIG_HOME/jot/config.yaml" ]

    run jot config get output.fortune
    [ "$status" -eq 0 ]
    [ "$output" = "false" ]
}

@test "config set --local: auto-creates .jot/ and writes" {
    run jot config set --local output.fortune false
    [ "$status" -eq 0 ]
    [[ "$output" == *"[local]"* ]]
    [ -f "$PWD/.jot/config.yaml" ]

    run jot config get output.fortune
    [ "$status" -eq 0 ]
    [ "$output" = "false" ]
}

@test "config set: default picks local when .jot/ exists in cwd" {
    # Seed a project by creating .jot via any jot write (add creates it).
    jot add "seed" >/dev/null
    run jot config set output.fortune false
    [ "$status" -eq 0 ]
    [[ "$output" == *"[local]"* ]]
    [ -f "$PWD/.jot/config.yaml" ]
}

@test "config set: default picks global when .jot/ absent but global exists" {
    # Prime the global file.
    jot config set --global output.fortune true >/dev/null
    # Now implicit set: no .jot/ here, global exists, should target global.
    run jot config set output.fortune false
    [ "$status" -eq 0 ]
    [[ "$output" == *"[global]"* ]]
}

@test "config set: local overrides global" {
    jot config set --global output.fortune true >/dev/null
    jot add "seed" >/dev/null
    jot config set --local output.fortune false >/dev/null

    run jot config get output.fortune
    [ "$status" -eq 0 ]
    [ "$output" = "false" ]
}

@test "config set: strict schema rejects unknown key" {
    run jot config set --global outpt.fortune false
    [ "$status" -ne 0 ]
    [[ "$output" == *"not a known config key"* || "$output" == *"wrong type"* ]]
}

@test "config set: type error carries a schema-derived hint" {
    run jot config set --global output.fortune notabool
    [ "$status" -ne 0 ]
    [[ "$output" == *"not valid"* ]]
    [[ "$output" == *"true or false"* ]]
}

@test "config set: fortune-category accepts enum variant" {
    run jot config set --global output.fortune-category tech
    [ "$status" -eq 0 ]

    run jot config get output.fortune-category
    [ "$status" -eq 0 ]
    [ "$output" = "tech" ]
}

@test "config set: fortune-category rejects unknown variant" {
    run jot config set --global output.fortune-category notacategory
    [ "$status" -ne 0 ]
    [[ "$output" == *"tech"* ]]
    [[ "$output" == *"humor"* ]]
}

@test "config set: --global and --local are mutually exclusive" {
    run jot config set --global --local output.fortune false
    [ "$status" -ne 0 ]
}

@test "fortune opt-out: no fortune appears on stderr after set false" {
    jot config set --global output.fortune false >/dev/null
    # Force a TTY by faking via script(1) so fortune's is_terminal check passes.
    # Run many times so a 20% probability would almost certainly surface at
    # least once if the gate were broken.
    for _ in $(seq 1 20); do
        out="$(script -qec 'jot add foo' /dev/null 2>&1)"
        [[ "$out" != *$'\x1b[2m'* ]] || return 1
    done
}
