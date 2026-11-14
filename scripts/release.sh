#!/usr/bin/env bash
# Usage: scripts/release.sh X.Y.Z [notes-file]
# DRY_RUN=1: bump and test offline, restoring files even when a check fails.
# RESUME=1: finish an interrupted preparation for an existing local tag.
# The release stays a draft until release-native.yml has verified every download.
#
# Manual releases, including the FIRST release (0.0.1): run from synchronized
# main. The label gate (release-gate.yml) only bumps versions that already
# have a previous tag; it never creates 0.0.1.
#
# Inkuno naming: APP follows crates/inkuno/Cargo.toml and REPO defaults to the Inkuno home
# below, so no edit is needed here. Remaining `bref` asset
# strings elsewhere are the rename crew's lane (see docs/releasing.md).
set -euo pipefail
cd "$(dirname "$0")/.."

APP=$(sed -n 's/^name = "\(.*\)"/\1/p' crates/inkuno/Cargo.toml | head -1)
TITLE=Inkuno
REPO="${INKUNO_REPO:-kolisachint/inkuno}"
managed=(crates/inkuno/Cargo.toml Cargo.lock)
die() { echo "release: $*" >&2; exit 1; }
ver=${1:?usage: scripts/release.sh X.Y.Z [notes-file]}
[[ $ver =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "version must look like 1.2.3"
[[ ${DRY_RUN:-0} =~ ^[01]$ && ${RESUME:-0} =~ ^[01]$ ]] || die "DRY_RUN and RESUME must be 0 or 1"
dry=${DRY_RUN:-0}
resume=${RESUME:-0}
[[ $dry != 1 || $resume != 1 ]] || die "DRY_RUN and RESUME cannot be combined"
tag="v$ver"
git diff --quiet || die "uncommitted changes"
git diff --cached --quiet || die "staged changes"
if [ "$dry" = 0 ]; then
    [ "$(git branch --show-current)" = main ] || die "not on main"
fi
if [ "$resume" = 1 ]; then
    git rev-parse -q --verify "refs/tags/$tag" >/dev/null || die "no local tag $tag to resume"
    git merge-base --is-ancestor "$tag" HEAD || die "$tag is not an ancestor of HEAD"
    tagged_ver=$(git show "$tag:crates/inkuno/Cargo.toml" | sed -n 's/^version = "\(.*\)"/\1/p' | head -1)
    current_ver=$(sed -n 's/^version = "\(.*\)"/\1/p' crates/inkuno/Cargo.toml | head -1)
    [ "$tagged_ver" = "$ver" ] && [ "$current_ver" = "$ver" ] || die "tag and checkout must both carry version $ver"
else
    if git rev-parse -q --verify "refs/tags/$tag" >/dev/null; then
        die "tag $tag already exists; use RESUME=1 to finish its preparation"
    fi
fi

release_state=
if [ "$dry" = 0 ]; then
    # Listing distinguishes a missing release from a failed API request. Never
    # treat a network/authentication error as permission to change a public one.
    release_state=$(gh api --paginate "repos/$REPO/releases" --jq ".[] | select(.tag_name == \"$tag\") | .draft")
    case "$release_state" in
        false) die "$tag is already public; this script only changes drafts" ;;
        true) [ "$resume" = 1 ] || die "draft already exists; use RESUME=1" ;;
        '') ;;
        *) die "unexpected release state" ;;
    esac
    git fetch -q origin --tags
    if [ "$resume" = 1 ]; then
        git merge-base --is-ancestor origin/main HEAD || die "main is behind or diverges from origin/main"
    else
        if git rev-parse -q --verify "refs/tags/$tag" >/dev/null; then
            die "remote tag $tag already exists; use RESUME=1"
        fi
        [ "$(git rev-parse HEAD)" = "$(git rev-parse origin/main)" ] || die "main differs from origin/main"
    fi
fi

scratch=$(mktemp -d)
rollback=0
snapshot() {
    for file in "${managed[@]}"; do
        mkdir -p "$scratch/original/$(dirname "$file")"
        cp "$file" "$scratch/original/$file"
    done
    rollback=1
}
cleanup() {
    code=$?
    trap - EXIT
    if [ "$rollback" = 1 ]; then
        # The index was clean on entry. Restore only the files owned by this
        # script; preserve untracked files.
        git reset -q HEAD -- "${managed[@]}" || code=1
        for file in "${managed[@]}"; do
            cp "$scratch/original/$file" "$file" || code=1
        done
    fi
    rm -rf "$scratch"
    exit "$code"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
notes="$scratch/notes.md"
if [ -n "${2:-}" ]; then
    cp "$2" "$notes"
else
    section=
    if [ -f CHANGELOG.md ]; then
        section=$(awk -v v="$ver" '/^## /{on = ($2 == v || $2 == "[" v "]")} on' CHANGELOG.md | sed 1d)
    fi
    {
        echo "## Changes"; echo
        if [ -n "$section" ]; then
            echo "$section"
        else
            # No previous tag is a valid (first, manual) release. Include the root commit.
            if [ "$resume" = 1 ]; then
                prev=$(git describe --tags --abbrev=0 "$tag^" 2>/dev/null || true)
                end="$tag"
            else
                prev=$(git describe --tags --abbrev=0 2>/dev/null || true)
                end=HEAD
            fi
            if [ -n "$prev" ]; then range="$prev..$end"; else range="$end"; fi
            git log "$range" --no-merges --pretty='- %s' | grep -vE '^- (chore|build|refactor)(\(.*\))?: ' || true
        fi
        echo; echo "## Install and upgrade"; echo
        echo "See the [README](https://github.com/$REPO#install). All downloads are attached before this release is published."
    } >"$notes"
fi

if [ "$resume" = 0 ]; then
    snapshot
    # First `version` line only; portable across GNU/BSD sed via perl.
    perl -pi -e 's/^version = ".*"/version = "'"$ver"'"/ if !$done && ($done = /^version = /)' crates/inkuno/Cargo.toml
    cargo update -p "$APP" --offline -q
    if [ "$dry" = 1 ]; then
        cargo test --locked --offline -q
        cat "$notes"
        git diff --stat -- "${managed[@]}"
        echo "dry run complete: restoring version files"
        exit 0
    fi
    cargo test --locked -q
    git add crates/inkuno/Cargo.toml Cargo.lock
    git diff --cached --quiet || git commit -q -m "chore: release $ver"
    rollback=0
    git tag -a "$tag" -m "$TITLE $ver"
fi
# Atomic push prevents a tag reaching the remote without its main commit.
# Repeating the same push also recovers an interruption after local tagging.
git push --atomic origin main "$tag"

if [ -z "$release_state" ]; then
    gh release create "$tag" --repo "$REPO" --draft --verify-tag --title "$TITLE $ver" --notes-file "$notes"
fi
# Draft release events do not start packaging. Dispatch the orchestrator
# explicitly, including on recovery; never publish from this local script.
gh workflow run release-native.yml --repo "$REPO" --ref main -f "tag=$tag"
echo "Prepared $tag as a draft; release-native.yml will publish only after every download passes."
