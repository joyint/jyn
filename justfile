# Jot -- Task Runner

# List recipes
default:
    @just --list

# Release (auto patch bump from latest git tag)
release version="":
    #!/usr/bin/env bash
    set -euo pipefail
    v="{{version}}"
    if [ -n "$v" ]; then
        semver="${v#v}"
    else
        current=$(git describe --tags --abbrev=0 2>/dev/null || echo "v0.0.0")
        current="${current#v}"
        major=$(echo "$current" | cut -d. -f1)
        minor=$(echo "$current" | cut -d. -f2)
        patch=$(echo "$current" | cut -d. -f3)
        semver="${major}.${minor}.$((patch + 1))"
    fi
    tag="v${semver}"

    if [ -n "$(git status --porcelain)" ]; then
        echo "Error: working tree is not clean."
        exit 1
    fi

    # Update Cargo.toml versions if they exist
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
    if git diff --cached --quiet; then
        echo "No version files to update."
    else
        git commit -m "bump to ${tag}"
    fi
    git tag "${tag}"
    git push && git push origin "${tag}"
    echo "Released ${tag}"
