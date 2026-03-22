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
check:
    just fmt-check && just lint && just test

# Check tools and deps
doctor:
    #!/usr/bin/env bash
    red=$'\033[31m' reset=$'\033[0m'
    ok()   { local v; v=$("$1" --version 2>/dev/null) && echo "  $2: $v" || echo "  $2: ok"; }
    miss() { printf "  %s%s: MISSING%s\n" "$red" "$1" "$reset"; }
    command -v cargo         >/dev/null && ok cargo cargo           || miss cargo
    command -v rustfmt       >/dev/null && ok rustfmt rustfmt       || miss rustfmt
    command -v clippy-driver >/dev/null && ok clippy-driver clippy  || miss clippy

# Setup (nothing extra needed)
setup:
    @true

# Install to ~/.local/bin/
install:
    cargo build --release -p jot && mkdir -p ~/.local/bin && cp target/release/jot ~/.local/bin/jot

# Release (bump: patch, minor, or major)
release bump="patch" confirm="ask":
    #!/usr/bin/env bash
    set -euo pipefail
    if git describe --tags --exact-match HEAD >/dev/null 2>&1; then
        echo "No changes since last tag, skipping."
        exit 0
    fi
    if [ -n "$(git status --porcelain)" ]; then
        echo "Error: working tree is not clean."
        exit 1
    fi
    current=$(git describe --tags --abbrev=0 2>/dev/null || echo "v0.0.0")
    current="${current#v}"
    major=$(echo "$current" | cut -d. -f1)
    minor=$(echo "$current" | cut -d. -f2)
    patch=$(echo "$current" | cut -d. -f3)
    case "{{bump}}" in
        major) semver="$((major + 1)).0.0" ;;
        minor) semver="${major}.$((minor + 1)).0" ;;
        patch) semver="${major}.${minor}.$((patch + 1))" ;;
        *) echo "Error: bump must be patch, minor, or major"; exit 1 ;;
    esac
    tag="v${semver}"
    if [ "{{confirm}}" = "ask" ]; then
        read -rp "Release ${tag}? [y/N] " c
        if [[ "$c" != [yY] ]]; then echo "Aborted."; exit 0; fi
    fi
    for f in $(find . -name Cargo.toml -not -path '*/target/*'); do
        if grep -q '^version = ' "$f"; then
            sed -i "s/^version = \".*\"/version = \"${semver}\"/" "$f"
            echo "  ${f} -> ${semver}"
        fi
    done
    if [ -f Cargo.toml ]; then
        cargo generate-lockfile 2>/dev/null || cargo check 2>/dev/null
    fi
    git add -A
    git commit --quiet -m "bump to ${tag}"
    # Annotated tag with release notes (shown as GitHub Release body)
    if [ -f ".joy/project.yaml" ] && command -v joy >/dev/null 2>&1; then
        joy release show "${tag}" | git tag -a "${tag}" -F -
    else
        git tag "${tag}"
    fi
    git push --quiet && git push --quiet origin "${tag}"
    echo "Released ${tag}"
    # Optional GitHub Release
    if command -v gh >/dev/null 2>&1; then
        read -rp "Create GitHub Release? [y/N] " gh_confirm
        if [[ "$gh_confirm" == [yY] ]]; then
            if [ -f ".joy/project.yaml" ] && command -v joy >/dev/null 2>&1; then
                joy release show "${tag}" | gh release create "${tag}" --title "${tag}" --notes-file -
            else
                gh release create "${tag}" --title "${tag}" --generate-notes
            fi
            echo "GitHub Release created."
        fi
    fi
