#!/usr/bin/env bash
# scripts/sync/5remotes_drift_check.sh — read-only checker for 5-remote drift.
#
# Compares the same ref across the 5 remotes and reports ahead/behind counts.
# Designed to run as a cron / on-demand health check. NEVER pushes.
#
# Usage:
#   scripts/sync/5remotes_drift_check.sh [--branches b1,b2,...] [--alert-threshold N]
#   scripts/sync/5remotes_drift_check.sh [--strict-main|--no-strict-main]
#
# Exit codes:
#   0 — all pairs within threshold AND main identical across remotes
#   1 — drift exceeds threshold, OR main diverged across remotes
#   2 — network error fetching from a remote
#
# Output format (tab-separated for log scraping):
#   branch  pair_a  pair_b  ahead  behind
#
# Designed to be piped into ops tools:
#   scripts/sync/5remotes_drift_check.sh --branches develop/v4.1.0,main > /var/log/drift.tsv
#
# 2026-09-30: added --strict-main (default ON).
#   Rationale: `main` is the GA release pointer (STAGE_CONFIG branches.main:
#   { stage: GA_only, mergeable: false, protected: true }). It must be
#   byte-identical across all remotes — unlike develop branches it carries no
#   in-flight work, so ANY divergence there is a defect, not normal drift.
#   On 2026-09-30 `main` was found 16657 commits behind and diverged from
#   develop/v4.1.0 (see docs/releases/v4.1.0/MAIN_DIVERGENCE_RESOLUTION_2026-09-30.md).
#   The old uniform threshold let that accumulate unnoticed.

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

DEFAULT_BRANCHES="develop/v4.1.0,main,release/v4.0.0,develop/v4.0.0"
DEFAULT_THRESHOLD=2
STRICT_MAIN=1

branches_csv="$DEFAULT_BRANCHES"
threshold="$DEFAULT_THRESHOLD"

while [ "${1:-}" != "" ]; do
    case "$1" in
        --branches)
            branches_csv="$2"
            shift 2
            ;;
        --alert-threshold)
            threshold="$2"
            shift 2
            ;;
        --strict-main)
            STRICT_MAIN=1
            shift
            ;;
        --no-strict-main)
            STRICT_MAIN=0
            shift
            ;;
        -h|--help)
            sed -n '2,30p' "$0"
            exit 0
            ;;
        *)
            echo "ERROR: unknown option $1" >&2
            exit 4
            ;;
    esac
done

IFS=',' read -ra branches <<< "$branches_csv"

echo "=== 5-remote drift check ==="
echo "branches: ${branches[*]}"
echo "threshold: $threshold commits"
echo ""

echo "fetching..."
git fetch --all --no-tags --prune 2>&1 | tail -3
echo ""

REMOTES=(gitcode gitea250 gitea252 gitee github)
# tip_<remote> vars (POSIX-portable, no associative arrays)
tip_gitcode=""
tip_gitea250=""
tip_gitea252=""
tip_gitee=""
tip_github=""

EXIT_CODE=0
MAIN_DIVERGED=0
echo -e "branch\tpair\tpair\tahead\tbehind"

for branch in "${branches[@]}"; do
    tip_gitcode=$(git rev-parse --verify "gitcode/$branch" 2>/dev/null || echo "")
    tip_gitea250=$(git rev-parse --verify "gitea250/$branch" 2>/dev/null || echo "")
    tip_gitea252=$(git rev-parse --verify "gitea252/$branch" 2>/dev/null || echo "")
    tip_gitee=$(git rev-parse --verify "gitee/$branch" 2>/dev/null || echo "")
    tip_github=$(git rev-parse --verify "github/$branch" 2>/dev/null || echo "")

    # Use indirection to look up $tip_<remote>
    for ((i=0; i<${#REMOTES[@]}; i++)); do
        for ((j=i+1; j<${#REMOTES[@]}; j++)); do
            a="${REMOTES[$i]}"
            b="${REMOTES[$j]}"
            # Resolve sa, sb via indirect variable names
            var_a="tip_${a}"
            var_b="tip_${b}"
            sa=$(eval echo "\$$var_a")
            sb=$(eval echo "\$$var_b")
            if [ -z "$sa" ] || [ -z "$sb" ]; then
                continue
            fi
            ab=$(git rev-list --count "$sa" "^$sb" 2>/dev/null)
            ba=$(git rev-list --count "$sb" "^$sa" 2>/dev/null)
            printf "%s\t%s\t%s\t%d\t%d\n" "$branch" "$a" "$b" "$ab" "$ba"
            total=$((ab+ba))

            # Strict mode: main must be byte-identical across every remote.
            if [ "$STRICT_MAIN" -eq 1 ] && [ "$branch" = "main" ]; then
                if [ "$total" -ne 0 ]; then
                    MAIN_DIVERGED=1
                    EXIT_CODE=1
                fi
            elif [ "$total" -gt "$threshold" ]; then
                EXIT_CODE=1
            fi
        done
    done
done

echo ""
if [ "$STRICT_MAIN" -eq 1 ]; then
    # Dedicated, unmissable report for the release pointer.
    main_tips=""
    for a in "${REMOTES[@]}"; do
        var_a="tip_${a}"
        sa=$(eval echo "\$$var_a")
        [ -n "$sa" ] && main_tips="$main_tips ${sa:0:12}"
    done
    main_uniq=$(echo $main_tips | tr ' ' '\n' | grep . | sort -u | wc -l | tr -d ' ')
    if [ "$MAIN_DIVERGED" -eq 0 ] && [ "$main_uniq" = "1" ]; then
        echo "MAIN: OK — identical across all remotes (${main_tips# })"
    else
        echo "MAIN: ERROR — 'main' has DIVERGED across remotes (strict mode, zero tolerance)."
        echo "      Distinct tips: ${main_uniq}"
        echo "      'main' is the GA release pointer and must be byte-identical everywhere."
        echo "      Resolve via docs/releases/v4.1.0/MAIN_DIVERGENCE_RESOLUTION_2026-09-30.md"
        echo "      (backup branch/tag: backup/main-pre-convergence-2026-09-30,"
        echo "       archive/main-pre-convergence-2026-09-30)"
    fi
    echo ""
fi

if [ "$EXIT_CODE" -eq 0 ]; then
    echo "RESULT: all branches within threshold ($threshold)"
else
    echo "RESULT: drift detected (threshold=$threshold, strict_main=$STRICT_MAIN)"
fi

exit "$EXIT_CODE"