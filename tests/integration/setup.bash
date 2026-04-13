#!/usr/bin/env bash
# Common setup for bats integration tests.
# Sources: https://bats-core.readthedocs.io/

# Prefer debug build for speed; fall back to jot on PATH.
JOT_BIN="${JOT_BIN:-$(pwd)/target/debug/jot}"
if [ ! -x "$JOT_BIN" ]; then
    JOT_BIN="$(command -v jot)"
fi
export PATH="$(dirname "$JOT_BIN"):$PATH"

setup() {
    TEST_DIR="$(mktemp -d)"
    cd "$TEST_DIR" || exit 1
}

teardown() {
    cd /
    rm -rf "$TEST_DIR"
}
