# Jot -- Task Runner

# List recipes
default:
    @just --list

# Run all tests (unit + integration)
test: test-unit test-int

# Rust unit tests only
test-unit:
    cargo test --workspace

# Integration tests (bats)
test-int:
    cargo build -p jyn-cli
    bats tests/integration/*.bats

# Format all code
fmt:
    cargo fmt --all

# Check formatting
fmt-check:
    cargo fmt --all -- --check

# Lint all code
lint:
    cargo clippy --workspace -- -D warnings

# Abort if the local Rust stable toolchain is behind the latest release.
[private]
_toolchain-check:
    #!/usr/bin/env bash
    if rustup check stable 2>/dev/null | grep -q "Update available"; then
        echo "Error: Rust stable toolchain is outdated. Run 'rustup update stable'."
        exit 1
    fi

# Refresh the in-crate copy of the tutorial from the canonical doc.
# crates/jyn-cli ships its own copy because `cargo package` builds the
# crate in isolation; a unit test fails if the two drift.
sync-tutorial:
    mkdir -p crates/jyn-cli/docs/user
    cp docs/user/Tutorial.md crates/jyn-cli/docs/user/Tutorial.md

# Run sync-tutorial, fmt-check, lint, test
check: _toolchain-check sync-tutorial fmt-check lint test

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
    command -v bats          >/dev/null && ok bats bats             || opt "bats" "pacman -S bats"
    command -v gh            >/dev/null && ok gh "gh (GitHub CLI)" || opt "gh" "https://cli.github.com"

# Setup (nothing extra needed)
setup:
    @true

# Install to ~/.local/bin/
install:
    cargo build --release -p jyn-cli && mkdir -p ~/.local/bin && cp target/release/jyn ~/.local/bin/jyn

# Auto-commit known generated files (.joy/, lockfiles)
[private]
auto-commit:
    #!/usr/bin/env bash
    files=(.joy/ Cargo.lock package-lock.json yarn.lock)
    staged=false
    for f in "${files[@]}"; do
        if git status --porcelain "$f" 2>/dev/null | grep -q .; then
            git add "$f"
            staged=true
        fi
    done
    if [ "$staged" = true ]; then
        git commit --quiet -m "chore: update generated files [no-item]"
        echo "Committed pending changes."
    fi

# Release (bump: patch, minor, or major; prerelease: e.g. alpha, beta, rc1)
release bump="patch" prerelease="":
    #!/usr/bin/env bash
    set -euo pipefail
    if git describe --tags --exact-match HEAD >/dev/null 2>&1; then
        echo "No changes since last tag, skipping."
        exit 0
    fi
    just auto-commit
    if ! command -v joy >/dev/null 2>&1 || ! [ -f ".joy/project.yaml" ]; then
        echo "No Joy project found. Use joy init to set up."
        exit 1
    fi
    if ! joy release show >/dev/null 2>&1; then
        echo "No items closed since last release."
        exit 0
    fi
    # Joy logs every invocation to .joy/logs/YYYY-MM-DD.log, so the
    # `joy release show` above may have dirtied the tree. Absorb those
    # writes before the clean check, otherwise the release aborts on
    # its own audit trail.
    just auto-commit
    if [ -n "$(git status --porcelain)" ]; then
        echo "Error: working tree is not clean:"
        git status --short | sed -n '1,20s/^/  /p'
        exit 1
    fi
    echo "Updating external dependencies..."
    cargo update
    just auto-commit
    echo "Bumping version files..."
    joy release bump "{{bump}}" "{{prerelease}}"
    echo "Refreshing Cargo.lock..."
    cargo update --workspace
    echo "Checking (format, lint, test)..."
    # The output is kept: a check that fails only here, on the bumped
    # tree, cannot be asked again once the bump is rolled back.
    check_log="${TMPDIR:-/tmp}/jyn-release-check.log"
    if ! just check > "$check_log" 2>&1; then
        echo "Checks failed. What failed, from $check_log:"
        grep -E 'FAILED|panicked at|^error|^not ok|recipe .* failed' "$check_log" | tail -n 20 || true
        echo "The last lines:"
        tail -n 25 "$check_log"
        echo "Rolling bump back."
        git restore crates/ Cargo.lock
        exit 1
    fi
    # The record commits only what joy wrote, the version files. The lock
    # file refreshed for the bumped version goes in before it: left
    # uncommitted behind the tag, it keeps this checkout changed, and the
    # umbrella release that follows stops on that.
    if ! git diff --quiet Cargo.lock; then
        git add Cargo.lock
        git commit --quiet -m "chore: Cargo.lock for the release [no-item]"
    fi
    joy release record "{{bump}}" "{{prerelease}}"
    tag=$(git describe --tags --exact-match HEAD 2>/dev/null || echo "unknown")
    echo "Tagged ${tag} locally. Run 'just publish' to ship."

# Upload crates to crates.io only. Idempotent: already-uploaded
# versions are skipped. The local-first path calls this via
# `just publish`; publish.yml is manual-only for recovery runs.
# The forge release is handled separately by `joy release publish`.
publish-crates: sync-tutorial
    #!/usr/bin/env bash
    set -euo pipefail
    if [ -z "${CARGO_REGISTRY_TOKEN:-}" ]; then
        echo "Error: CARGO_REGISTRY_TOKEN is not set."
        echo "  - Local: add it to the umbrella's .env (see .env.example)."
        echo "  - CI: export it from the runner's secret store."
        exit 1
    fi
    # Order matters: dependents after dependencies.
    # Order matters: dependents after dependencies. jyn (alias crate)
    # must publish last because it depends on jyn-cli being on crates.io.
    crates=(jyn-core jyn-cli jyn)
    for crate in "${crates[@]}"; do
        version=$(cargo pkgid --quiet -p "$crate" 2>/dev/null | sed 's/.*[#@]\(.*\)/\1/')
        if [ -z "$version" ]; then
            echo "Warning: could not resolve version for $crate, skipping."
            continue
        fi
        # cargo search lags the registry by minutes, so we also catch
        # "already uploaded" from the actual publish call below.
        if cargo search "$crate" --limit 1 2>/dev/null | grep -qE "^$crate = \"$version\""; then
            echo "$crate $version already on crates.io, skipping."
            continue
        fi
        echo "Publishing $crate $version..."
        if ! out=$(cargo publish -p "$crate" 2>&1); then
            # Two cargo error variants both mean "version already published":
            # - "is already uploaded": registry rejected the upload
            # - "already exists on crates.io index": cargo's pre-check
            if echo "$out" | grep -qE "is already uploaded|already exists on crates.io index"; then
                echo "$crate $version already on crates.io (registry confirmed), skipping."
            else
                echo "$out" >&2
                exit 1
            fi
        else
            echo "$out"
        fi
        sleep 5
    done
    echo "crates.io uploads complete."

# Full publish: upload crates + push + create forge release. Called
# per sub by `just release-all -P` in the umbrella. crates.io upload
# runs first so a failed upload leaves only a local tag to drop.
# Publish workspace crates, then push + forge release.
publish: publish-crates
    joy release publish

# Push the AUR package joyint-jyn-bin for the published release <version>
# (with or without the v): the template in packaging/aur takes the
# release's checksums, is test-built and goes to the AUR with its
# .SRCINFO. Arch only; the release's AUR job (.github/workflows/aur.yml)
# runs it in an Arch container with the AUR key.
publish-aur version:
    #!/usr/bin/env bash
    set -euo pipefail
    for t in makepkg curl git; do command -v "$t" >/dev/null || { echo "$t not found: the AUR package is built on Arch"; exit 1; }; done
    version="{{version}}"
    version="${version#v}"
    pkg=joyint-jyn-bin
    out="{{source_directory()}}/target/aur/$pkg"
    rm -rf "$out"; mkdir -p "$out"
    base="https://github.com/joyint/jyn/releases/download/v$version"
    sha() {
        curl -fsSL --proto '=https' --retry 6 --retry-delay 20 --retry-all-errors \
            "$base/jyn-cli-$1-unknown-linux-gnu.tar.xz.sha256" | cut -d' ' -f1
    }
    x86_64=$(sha x86_64 || true)
    aarch64=$(sha aarch64 || true)
    if [ ${#x86_64} -ne 64 ] || [ ${#aarch64} -ne 64 ]; then
        echo "no published checksums for v$version: publish the release first"
        exit 1
    fi
    sed -e "s|@PKGVER@|${version//-/_}|g" -e "s|@VER@|$version|g" \
        -e "s|@SHA256_X86_64@|$x86_64|" -e "s|@SHA256_AARCH64@|$aarch64|" \
        "{{source_directory()}}/packaging/aur/$pkg/PKGBUILD" > "$out/PKGBUILD"
    cd "$out"
    makepkg --printsrcinfo > .SRCINFO
    # --nodeps: nothing is built, the release archive is repacked; the
    # libraries it runs on are needed where it is installed, not here.
    makepkg --force --cleanbuild --noconfirm --nodeps
    repo=$(mktemp -d)
    trap 'rm -rf "$repo"' EXIT
    git clone "ssh://aur@aur.archlinux.org/$pkg.git" "$repo"
    cp PKGBUILD .SRCINFO "$repo/"
    cd "$repo"
    git add PKGBUILD .SRCINFO
    git diff --cached --quiet && { echo "AUR already at $version"; exit 0; }
    git commit -m "Update to $version"
    git push origin HEAD:master
