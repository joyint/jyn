#!/usr/bin/env bats
# jyn update: receipt-gated binary self-update. JYN-0004-E6.
#
# The test binary is not installed by cargo-dist, so it carries no
# install receipt. Both `jyn update` and `jyn update --check` must
# therefore report that the binary is managed by another installer and
# exit cleanly, without touching the network.

load setup

@test "jyn update reports managed-by-another-installer on a non-cargo-dist build" {
    run jyn update
    [ "$status" -eq 0 ]
    [[ "$output" == *"managed by another installer"* ]]
}

@test "jyn update --check is clean and exits 0 without a receipt" {
    run jyn update --check
    [ "$status" -eq 0 ]
    [[ "$output" == *"managed by another installer"* ]]
}

@test "jyn update --help lists the --check flag" {
    run jyn update --help
    [ "$status" -eq 0 ]
    [[ "$output" == *"--check"* ]]
}
