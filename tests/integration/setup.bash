#!/usr/bin/env bash
# Common setup for bats integration tests.
# Sources: https://bats-core.readthedocs.io/

# Prefer debug build for speed; fall back to jyn on PATH.
JYN_BIN="${JYN_BIN:-$(pwd)/target/debug/jyn}"
if [ ! -x "$JYN_BIN" ]; then
    JYN_BIN="$(command -v jyn)"
fi
export PATH="$(dirname "$JYN_BIN"):$PATH"

setup() {
    TEST_DIR="$(mktemp -d)"
    cd "$TEST_DIR" || exit 1
}

teardown() {
    cd /
    rm -rf "$TEST_DIR"
}
