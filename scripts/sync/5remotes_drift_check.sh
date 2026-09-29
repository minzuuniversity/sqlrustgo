#!/usr/bin/env bash
# scripts/sync/5remotes_drift_check.sh — read-only checker for 5-remote drift.
#
# Compares the same ref across the 5 remotes and reports ahead/behind counts.
# Designed to run as a cron / on-demand health check. NEVER pushes.
#
# Usage:
#   scripts/sync/5remotes_drift_check.sh [--branches b1,b2,...] [--alert-threshold N]
#
# Exit codes:
#   0 — all 10 pairs at all branches are within threshold
#   1 — drift exceeds threshold (alert)
#   2 — network error fetching from a remote
#
# Output format (tab-separated for log scraping):
#   branch  pair_a  pair_b  ahead  behind
#
# Designed to be piped into ops tools:
#   scripts/sync/5remotes_drift_check.sh --branches develop/v4.1.0,main > /var/log/drift.tsv

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

DEFAULT_BRANCHES="develop/v4.1.0,main,release/v4.0.0,develop/v4.0.0"
DEFAULT_THRESHOLD=2

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
        -h|--help)
            sed -n '2,20p' "$0"
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
            if [ "$total" -gt "$threshold" ]; then
                EXIT_CODE=1
            fi
        done
    done
done

echo ""
if [ "$EXIT_CODE" -eq 0 ]; then
    echo "RESULT: all branches within threshold ($threshold)"
else
    echo "RESULT: drift exceeds threshold ($threshold) on at least one pair"
fi

exit "$EXIT_CODE"