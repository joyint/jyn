#!/usr/bin/env bats
# jyn tutorial: renders the user guide and lists chapters. JYN-0005-90.

load setup

@test "jyn tutorial prints the user tutorial" {
    run jyn tutorial
    [ "$status" -eq 0 ]
    [[ "$output" == *"Jyn Tutorial"* ]]
    # Chapter headings that must appear.
    [[ "$output" == *"Capturing Tasks"* ]]
    [[ "$output" == *"Listing Tasks"* ]]
    [[ "$output" == *"Where Jyn Stores Your Tasks"* ]]
    [[ "$output" == *"Configuration"* ]]
}

@test "jyn tutorial documents the upward .jyn search" {
    run jyn tutorial
    [ "$status" -eq 0 ]
    [[ "$output" == *".jyn"* ]]
    [[ "$output" == *"upward"* ]]
}

@test "jyn tutorial --help describes the command" {
    run jyn tutorial --help
    [ "$status" -eq 0 ]
    [[ "$output" == *"tutorial"* ]]
    [[ "$output" == *"--interactive"* ]]
}
