# Jot -- Task Runner

# List recipes
default:
    @just --list

# Run all tests
test:
    cargo test --workspace

# Format all code
fmt:
    cargo fmt --all

# Check formatting
fmt-check:
    cargo fmt --all -- --check

# Lint all code
lint:
    cargo clippy --workspace -- -D warnings

# Run fmt-check, lint, test
check: fmt-check lint test

# Check tools and deps
doctor:
    #!/usr/bin/env bash
    red=$'\033[31m' reset=$'\033[0m'
    ok()   { local v; v=$("$1" --version 2>/dev/null) && echo "  $2: $v" || echo "  $2: ok"; }
    miss() { printf "  %s%s: MISSING%s\n" "$red" "$1" "$reset"; }
    opt()  { printf "  %s%s: MISSING (optional, %s)%s\n" "$red" "$1" "$2" "$reset"; }
    command -v cargo         >/dev/null && ok cargo cargo           || miss cargo
    command -v rustfmt       >/dev/null && ok rustfmt rustfmt       || miss rustfmt
    command -v clippy-driver >/dev/null && ok clippy-driver clippy  || miss clippy
    command -v gh            >/dev/null && ok gh "gh (GitHub CLI)" || opt "gh" "https://cli.github.com"

# Setup (nothing extra needed)
setup:
    @true

# Install to ~/.local/bin/
install:
    cargo build --release -p jot && mkdir -p ~/.local/bin && cp target/release/jot ~/.local/bin/jot

# Release (bump: patch, minor, or major)
release bump="patch":
    #!/usr/bin/env bash
    set -euo pipefail
    if git describe --tags --exact-match HEAD >/dev/null 2>&1; then
        echo "No changes since last tag, skipping."
        exit 0
    fi
    changed=false
    if git status --porcelain .joy/ 2>/dev/null | grep -q .; then
        git add .joy/
        changed=true
    fi
    for lockfile in Cargo.lock package-lock.json yarn.lock; do
        if git status --porcelain "$lockfile" 2>/dev/null | grep -q .; then
            git add "$lockfile"
            changed=true
        fi
    done
    if [ "$changed" = true ]; then
        git commit --quiet -m "chore: update Joy data and lockfiles [no-item]"
        echo "Committed pending changes."
    fi
    if [ -n "$(git status --porcelain)" ]; then
        echo "Error: working tree is not clean."
        exit 1
    fi
    echo "Checking (format, lint, test)..."
    if ! just check > /dev/null 2>&1; then
        echo "Checks failed. Run 'just check' for details."
        exit 1
    fi
    if [ -f ".joy/project.yaml" ] && command -v joy >/dev/null 2>&1; then
        joy release create "{{bump}}" --full
    else
        echo "No Joy project found. Use joy init to set up."
        exit 1
    fi
