#!/usr/bin/env bash
# Usage: scripts/prune-agents.sh [--apply] [--min-free-gb N] [--worktrees-root DIR] [--no-repo-target] [--no-tmp]
#
# Safe disk-pruner for disk-full errors. Agents run this (dry-run by default)
# when builds fail with "no space left on device".
#
# What it does:
#   1. Checks free space on the repo filesystem (df). If free space is above
#      the threshold (default 10G, env PRUNE_MIN_FREE_GB), exits 0 doing
#      nothing.
#   2. Otherwise lists git-worktree target/ dirs
#      ($PRUNE_WORKTREES_ROOT/*/*/target plus ./target) by size and age.
#   3. Removes oldest-first until free space is above the threshold.
#      target/ is pure rebuildable cargo cache (`cargo build` recreates it).
#
# What it NEVER touches:
#   - the worktree of the current branch (by path and by branch name);
#   - any tree with uncommitted SOURCE changes (`git status --porcelain`
#     excluding target/ entries and untracked dispatch logs);
#   - anything outside target/ dirs except a small temp allowlist:
#     /tmp/inkuno-* entries and stale dispatch-*.log files.
#
# Dry-run is the default: without --apply nothing is deleted and the script
# prints what it WOULD remove. All deletions are behind --apply plus the
# current-tree guards above (re-checked immediately before each rm).
#
# Env knobs (flags win over env):
#   PRUNE_MIN_FREE_GB (default 10), PRUNE_WORKTREES_ROOT
#   (default ~/.hoocode/worktrees), PRUNE_INCLUDE_REPO_TARGET (default 1),
#   PRUNE_INCLUDE_TMP (default 1). The *_INCLUDE_* knobs exist so tests can
#   simulate --apply against a temp fixture without touching the real repo
#   target/ or /tmp.
set -euo pipefail
cd "$(dirname "$0")/.."

REPO_ROOT="$(pwd -P)"
APPLY=0
MIN_FREE_GB="${PRUNE_MIN_FREE_GB:-10}"
WORKTREES_ROOT="${PRUNE_WORKTREES_ROOT:-$HOME/.hoocode/worktrees}"
INCLUDE_REPO_TARGET="${PRUNE_INCLUDE_REPO_TARGET:-1}"
INCLUDE_TMP="${PRUNE_INCLUDE_TMP:-1}"

usage() {
    sed -n '1,/^set -euo/p' "$0" | sed -n '2,/^$/p' | sed 's/^# \?//'
}

while [ $# -gt 0 ]; do
    case "$1" in
        --apply) APPLY=1; shift ;;
        --min-free-gb) MIN_FREE_GB="${2:?--min-free-gb needs a value}"; shift 2 ;;
        --min-free-gb=*) MIN_FREE_GB="${1#*=}"; shift ;;
        --worktrees-root) WORKTREES_ROOT="$2"; shift 2 ;;
        --worktrees-root=*) WORKTREES_ROOT="${1#*=}"; shift ;;
        --no-repo-target) INCLUDE_REPO_TARGET=0; shift ;;
        --no-tmp) INCLUDE_TMP=0; shift ;;
        -h|--help) usage; exit 0 ;;
        *) echo "prune-agents: unknown argument: $1" >&2; echo "usage: scripts/prune-agents.sh [--apply] [--min-free-gb N] [--worktrees-root DIR] [--no-repo-target] [--no-tmp]" >&2; exit 2 ;;
    esac
done

[[ "$MIN_FREE_GB" =~ ^[0-9]+$ ]] || { echo "prune-agents: --min-free-gb must be a non-negative integer (got '$MIN_FREE_GB')" >&2; exit 2; }
[[ "$APPLY" =~ ^[01]$ && "$INCLUDE_REPO_TARGET" =~ ^[01]$ && "$INCLUDE_TMP" =~ ^[01]$ ]] || { echo "prune-agents: internal flag error" >&2; exit 2; }
# Canonicalize the worktrees root (macOS: /tmp -> /private/tmp) so the
# validate_target_path containment check compares real paths with real paths.
if [ -d "$WORKTREES_ROOT" ]; then
    WORKTREES_ROOT="$(cd "$WORKTREES_ROOT" && pwd -P)"
fi

die() { echo "prune-agents: $*" >&2; exit 1; }

# Kilobytes free on the filesystem holding DIR (df -kP is portable GNU/BSD).
free_kb_on() {
    df -kP "$1" | awk 'NR==2 {print $4}'
}
gb_to_kb() { echo "$(( $1 * 1024 * 1024 ))"; }
kb_to_gb_1dp() { awk -v kb="$1" 'BEGIN { printf "%.1f", kb / 1024 / 1024 }'; }

# Portable mtime (seconds since epoch) for a path; GNU stat -c, else BSD stat -f.
mtime_of() {
    if stat -c %Y "$1" >/dev/null 2>&1; then stat -c %Y "$1"; else stat -f %m "$1"; fi
}
# Size in KiB; missing dir reads as 0.
size_kb_of() {
    if [ -d "$1" ]; then du -sk "$1" 2>/dev/null | cut -f1; else echo 0; fi
}

CURRENT_TOP="$(git rev-parse --show-toplevel 2>/dev/null || echo "$REPO_ROOT")"
CURRENT_TOP="$(cd "$CURRENT_TOP" 2>/dev/null && pwd -P || echo "$REPO_ROOT")"
CURRENT_BRANCH="$(git branch --show-current 2>/dev/null || true)"

THRESHOLD_KB="$(gb_to_kb "$MIN_FREE_GB")"
FREE_KB="$(free_kb_on "$REPO_ROOT")"
echo "prune-agents: free $(kb_to_gb_1dp "$FREE_KB")G on repo filesystem, threshold ${MIN_FREE_GB}G (dry-run default; --apply to act)"

if [ "$FREE_KB" -ge "$THRESHOLD_KB" ]; then
    echo "prune-agents: above threshold, nothing to do."
    exit 0
fi

# --- candidate guards -------------------------------------------------------
# Echoes a skip reason, or nothing when the worktree is a prune candidate.
# A worktree is skipped when it is the current tree, on the current branch,
# not a git tree at all (cannot verify cleanliness), or dirty in SOURCE
# files. target/ entries and untracked dispatch logs never count as dirty.
skip_reason_for() {
    wt="$1"
    wt_real="$(cd "$wt" 2>/dev/null && pwd -P || echo "")"
    [ -n "$wt_real" ] || { echo "unresolvable path"; return; }
    if [ "$wt_real" = "$CURRENT_TOP" ]; then echo "current worktree"; return; fi
    if ! git -C "$wt" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
        echo "not a git worktree (cannot verify clean)"
        return
    fi
    wt_branch="$(git -C "$wt" branch --show-current 2>/dev/null || true)"
    if [ -n "$CURRENT_BRANCH" ] && [ -n "$wt_branch" ] && [ "$wt_branch" = "$CURRENT_BRANCH" ]; then
        echo "on current branch ($CURRENT_BRANCH)"
        return
    fi
    status="$(git -C "$wt" status --porcelain 2>/dev/null || echo "__GIT_STATUS_FAILED__")"
    if [ "$status" = "__GIT_STATUS_FAILED__" ]; then echo "git status failed"; return; fi
    # Drop target/ entries (both ` target/...` and `... target/...` forms)
    # and untracked dispatch logs (`?? ...dispatch...`), then see if anything
    # source-tracked remains.
    filtered="$(printf '%s\n' "$status" | grep -v ' target/' | grep -vi '^?? .*dispatch' || true)"
    if [ -n "$filtered" ]; then echo "uncommitted source changes"; return; fi
}

# Refuses to delete anything that is not exactly a <worktree>/target dir
# inside WORKTREES_ROOT (or the repo's own ./target). Returns 0 when safe.
validate_target_path() {
    t="$1"
    case "$t" in
        */target) ;;
        *) echo "refusing: not a target/ dir: $t" >&2; return 1 ;;
    esac
    [ -n "$t" ] && [ "$t" != "/" ] && [ "$t" != "$HOME" ] || { echo "refusing: unsafe path: $t" >&2; return 1; }
    parent="$(dirname "$t")"
    parent_real="$(cd "$parent" 2>/dev/null && pwd -P || echo "")"
    [ -n "$parent_real" ] || { echo "refusing: unresolvable parent: $t" >&2; return 1; }
    [ "$parent_real" != "$CURRENT_TOP" ] || { echo "refusing: current worktree: $t" >&2; return 1; }
    if [ "$parent_real" = "$REPO_ROOT" ]; then return 0; fi
    case "$parent_real" in
        "$WORKTREES_ROOT"/*) return 0 ;;
        *) echo "refusing: outside worktrees root and repo: $t" >&2; return 1 ;;
    esac
}

# --- enumerate target/ candidates -------------------------------------------
CANDIDATE_FILE="$(mktemp)"
trap 'rm -f "$CANDIDATE_FILE"' EXIT

add_candidate() {
    t="$1"
    [ -d "$t" ] || return 0
    wt="$(dirname "$t")"
    reason="$(skip_reason_for "$wt")"
    kb="$(size_kb_of "$t")"
    mt="$(mtime_of "$t")"
    if [ -n "$reason" ]; then
        printf 'SKIP\t%s\t%s\t%s (%s)\n' "$kb" "$mt" "$t" "$reason"
    else
        printf 'CAND\t%s\t%s\t%s\n' "$kb" "$mt" "$t" >>"$CANDIDATE_FILE"
        printf 'CAND\t%s\t%s\t%s\n' "$kb" "$mt" "$t"
    fi
}

echo "prune-agents: scanning $WORKTREES_ROOT/*/*/target"
if [ -d "$WORKTREES_ROOT" ]; then
    # Two-level glob mirrors ~/.hoocode/worktrees/<repo>/<label>/target.
    # find -maxdepth 3 stays equivalent if nesting varies slightly.
    while IFS= read -r d; do
        [ -n "$d" ] || continue
        add_candidate "$d"
    done <<EOF
$(find "$WORKTREES_ROOT" -maxdepth 3 -type d -name target 2>/dev/null || true)
EOF
else
    echo "prune-agents: worktrees root not found, skipping: $WORKTREES_ROOT"
fi
if [ "$INCLUDE_REPO_TARGET" = 1 ]; then
    add_candidate "$REPO_ROOT/target"
fi

if [ ! -s "$CANDIDATE_FILE" ]; then
    echo "prune-agents: no prunable target/ dirs (all skipped or absent)."
fi

# Oldest-first plan: accumulate candidates until projected free clears the
# threshold. sort -n on mtime is portable; cut/mtime fields are numeric.
PLAN_FILE="$(mktemp)"
trap 'rm -f "$CANDIDATE_FILE" "$PLAN_FILE"' EXIT
sort -n -t "$(printf '\t')" -k2,2 "$CANDIDATE_FILE" 2>/dev/null >"$PLAN_FILE" || cp "$CANDIDATE_FILE" "$PLAN_FILE"

projected_kb="$FREE_KB"
planned_kb=0
planned_n=0
PLANNED_LIST="$(mktemp)"
trap 'rm -f "$CANDIDATE_FILE" "$PLAN_FILE" "$PLANNED_LIST"' EXIT
while IFS="$(printf '\t')" read -r _kind kb mt path; do
    [ -n "${path:-}" ] || continue
    [ "$projected_kb" -ge "$THRESHOLD_KB" ] && break
    echo "$path" >>"$PLANNED_LIST"
    projected_kb=$(( projected_kb + kb ))
    planned_kb=$(( planned_kb + kb ))
    planned_n=$(( planned_n + 1 ))
done <"$PLAN_FILE"

# --- temp allowlist (report only; same --apply gate) --------------------------
TMP_ENTRIES="$(mktemp)"
trap 'rm -f "$CANDIDATE_FILE" "$PLAN_FILE" "$PLANNED_LIST" "$TMP_ENTRIES"' EXIT
if [ "$INCLUDE_TMP" = 1 ]; then
    for p in /tmp/inkuno-*; do
        [ -e "$p" ] || continue
        printf '%s\n' "$p" >>"$TMP_ENTRIES"
    done
    if [ -d "$WORKTREES_ROOT" ]; then
        find "$WORKTREES_ROOT" "$REPO_ROOT" -maxdepth 4 -name 'dispatch-*.log' -mtime +7 2>/dev/null >>"$TMP_ENTRIES" || true
    fi
fi

# --- report + act -------------------------------------------------------------
echo "prune-agents: plan: $planned_n target dir(s), ~$(kb_to_gb_1dp "$planned_kb")G projected freed (freed-space report below)"
if [ -s "$PLANNED_LIST" ]; then
    while IFS= read -r p; do
        [ -n "$p" ] || continue
        kb="$(size_kb_of "$p")"
        echo "  - $p ($(kb_to_gb_1dp "$kb")G, mtime $(mtime_of "$p"))"
    done <"$PLANNED_LIST"
fi
if [ -s "$TMP_ENTRIES" ]; then
    echo "prune-agents: temp allowlist (/tmp/inkuno-*, stale dispatch-*.log):"
    while IFS= read -r p; do
        [ -n "$p" ] || continue
        if [ -d "$p" ]; then echo "  - $p ($(kb_to_gb_1dp "$(size_kb_of "$p")")G)"; else echo "  - $p"; fi
    done <"$TMP_ENTRIES"
fi

if [ "$APPLY" = 0 ]; then
    echo "prune-agents: DRY-RUN: nothing deleted. Re-run with --apply to delete the above (projected free $(kb_to_gb_1dp "$projected_kb")G vs threshold ${MIN_FREE_GB}G)."
    exit 0
fi

echo "prune-agents: --apply: deleting oldest-first, re-checking guards and free space before each removal."
freed_kb=0
removed_n=0
while IFS= read -r p; do
    [ -n "$p" ] || continue
    cur_free="$(free_kb_on "$REPO_ROOT")"
    if [ "$cur_free" -ge "$THRESHOLD_KB" ]; then
        echo "prune-agents: threshold reached (${cur_free}K free), stopping."
        break
    fi
    validate_target_path "$p" || continue
    wt="$(dirname "$p")"
    reason="$(skip_reason_for "$wt")"
    if [ -n "$reason" ]; then
        echo "prune-agents: skip (now $reason): $p"
        continue
    fi
    kb="$(size_kb_of "$p")"
    rm -rf -- "$p"
    echo "prune-agents: removed $p (~$(kb_to_gb_1dp "$kb")G)"
    freed_kb=$(( freed_kb + kb ))
    removed_n=$(( removed_n + 1 ))
done <"$PLANNED_LIST"

if [ -s "$TMP_ENTRIES" ]; then
    while IFS= read -r p; do
        [ -n "$p" ] || continue
        case "$p" in
            /tmp/inkuno-*|*/dispatch-*.log) ;;
            *) echo "prune-agents: skip temp (outside allowlist): $p"; continue ;;
        esac
        # Never follow the allowlist into the current tree.
        case "$p" in
            "$CURRENT_TOP"/*) echo "prune-agents: skip temp (current worktree): $p"; continue ;;
        esac
        rm -rf -- "$p" && echo "prune-agents: removed temp $p" || echo "prune-agents: could not remove temp $p"
    done <"$TMP_ENTRIES"
fi

FINAL_KB="$(free_kb_on "$REPO_ROOT")"
echo "prune-agents: freed ~$(kb_to_gb_1dp "$freed_kb")G across $removed_n target dir(s); free was $(kb_to_gb_1dp "$FREE_KB")G, now $(kb_to_gb_1dp "$FINAL_KB")G (threshold ${MIN_FREE_GB}G)."
