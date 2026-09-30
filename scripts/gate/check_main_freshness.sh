#!/usr/bin/env bash
# scripts/gate/check_main_freshness.sh — anti-divergence gate for the `main` branch.
#
# WHY THIS EXISTS
#   On 2026-09-30 an audit found that `main` had diverged from `develop/v4.1.0`:
#   merge-base dated 2026-06-01, with 16657 commits unique to `main` and 16944
#   unique to develop. `main` also lacked every one of develop's implementation
#   files (0 files unique to main, but 93 files / +8000 lines missing from main).
#   The cause was a history rewrite (same commit subjects, different SHAs and
#   patch-ids), not parallel development.
#
#   `main` is the GA release pointer (STAGE_CONFIG branches.main:
#   { stage: GA_only, mergeable: false, protected: true }). It must never be a
#   stale orphan. This gate makes that machine-enforced instead of convention.
#
# WHAT IT CHECKS
#   C-MAIN-01  `main` is byte-identical across all 5 remotes (zero tolerance)
#   C-MAIN-02  `main` is not BIDIRECTIONALLY diverged from the newest release
#              branch (divergence, not mere staleness, is the defect)
#   C-MAIN-03  `main` is not more than --max-behind commits behind that release
#              branch (configurable; default 100)
#
# EXIT CODES
#   0 — all checks pass
#   1 — at least one check failed
#   2 — a required ref could not be resolved (treat as failure, never as pass)
#
# NOTE ON FAIL-CLOSED
#   Every check fails closed. An unresolvable ref yields FAIL, never PASS —
#   a gate that cannot evaluate its target must never report success.
#   See ANTI_FABRICATION_POLICY.md 7.4 (P16, Gate Test Integrity).

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

REMOTES=(gitea252 gitea250 gitee github gitcode)
MAX_BEHIND=100

while [ "${1:-}" != "" ]; do
    case "$1" in
        --max-behind)
            MAX_BEHIND="$2"
            shift 2
            ;;
        --no-fetch)
            SKIP_FETCH=1
            shift
            ;;
        -h|--help)
            sed -n '2,35p' "$0"
            exit 0
            ;;
        *)
            echo "ERROR: unknown option $1" >&2
            exit 4
            ;;
    esac
done

PASS=0
FAIL=0
log_pass() { echo "[PASS] $*"; PASS=$((PASS + 1)); }
log_fail() { echo "[FAIL] $*"; FAIL=$((FAIL + 1)); }

echo "=== C-MAIN Freshness / Anti-Divergence Check ==="
echo "HEAD: $(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
echo ""

if [ "${SKIP_FETCH:-0}" != "1" ]; then
    echo "fetching remotes (--no-fetch to skip)..."
    for r in "${REMOTES[@]}"; do
        git fetch "$r" --quiet 2>/dev/null || true
    done
    echo ""
fi

# ---------------------------------------------------------------------------
# C-MAIN-01: main identical across all 5 remotes
# ---------------------------------------------------------------------------
echo "[C-MAIN-01] Checking 'main' is identical across all ${#REMOTES[@]} remotes..."

main_tips=()
missing=0
for r in "${REMOTES[@]}"; do
    sha=$(git rev-parse --verify "refs/remotes/$r/main" 2>/dev/null || echo "")
    if [ -z "$sha" ]; then
        log_fail "C-MAIN-01: remote '$r' has no resolvable main ref (fail-closed)"
        missing=1
    else
        main_tips+=("$sha")
    fi
done

if [ "$missing" -eq 0 ]; then
    uniq=$(printf '%s\n' "${main_tips[@]}" | sort -u | wc -l | tr -d ' ')
    if [ "$uniq" = "1" ]; then
        log_pass "C-MAIN-01: all remotes at ${main_tips[0]:0:12}"
    else
        log_fail "C-MAIN-01: 'main' DIVERGED across remotes ($uniq distinct tips)."
        for i in "${!REMOTES[@]}"; do
            echo "         ${REMOTES[$i]} = ${main_tips[$i]:0:12}"
        done
        echo "         Resolve: docs/releases/v4.1.0/MAIN_DIVERGENCE_RESOLUTION_2026-09-30.md"
        echo "         Rollback: archive/main-pre-convergence-2026-09-30"
    fi
else
    echo "         (one or more remotes unresolvable — cannot assert main freshness)"
fi
echo ""

# ---------------------------------------------------------------------------
# C-MAIN-02 / C-MAIN-03: main vs newest release/* branch
# ---------------------------------------------------------------------------
echo "[C-MAIN-02/03] Checking 'main' against the newest release/* branch..."

main_sha=$(git rev-parse --verify refs/heads/main 2>/dev/null || git rev-parse --verify main 2>/dev/null || echo "")

# Pick the release branch with the most commits, preferring reachable-from-main.
release_branch=""
best=0
for cand in $(git for-each-ref --format='%(refname:short)' refs/heads/release/ 2>/dev/null); do
    [ -z "$main_sha" ] && continue
    if git merge-base --is-ancestor "$cand" "$main_sha" 2>/dev/null; then
        n=$(git rev-list --count "$cand..$main_sha" 2>/dev/null || echo 0)
    else
        n=$(git rev-list --count "$cand" 2>/dev/null || echo 0)
    fi
    if [ "$n" -ge "$best" ]; then
        best=$n
        release_branch="$cand"
    fi
done

if [ -z "$main_sha" ]; then
    log_fail "C-MAIN-02/03: local 'main' ref cannot be resolved (fail-closed)"
elif [ -z "$release_branch" ]; then
    log_fail "C-MAIN-02/03: no refs/heads/release/* branch found to compare against (fail-closed)"
else
    behind=$(git rev-list --count "$release_branch..$main_sha" 2>/dev/null || echo 0)
    ahead=$(git rev-list --count "$main_sha..$release_branch" 2>/dev/null || echo 0)
    echo "         release branch : $release_branch"
    echo "         main behind    : $behind commit(s)"
    echo "         main ahead     : $ahead commit(s)"

    # C-MAIN-02: bidirectional divergence is the real defect.
    if [ "$behind" -gt 0 ] && [ "$ahead" -gt 0 ]; then
        mb=$(git merge-base "$main_sha" "$release_branch" 2>/dev/null || echo unknown)
        log_fail "C-MAIN-02: 'main' has DIVERGED from $release_branch (both sides ahead; merge-base ${mb:0:12})."
        echo "         main=$ahead ahead / $behind behind. main is meant to be a release pointer,"
        echo "         not a parallel development line."
    else
        log_pass "C-MAIN-02: 'main' is not diverged from $release_branch"
    fi

    # C-MAIN-03: staleness bound
    if [ "$ahead" -gt "$MAX_BEHIND" ]; then
        log_fail "C-MAIN-03: 'main' is $ahead commits behind $release_branch (max allowed $MAX_BEHIND)."
        echo "         Raise with care — main should advance with each release line."
    else
        log_pass "C-MAIN-03: 'main' is $ahead commits behind $release_branch (limit $MAX_BEHIND)"
    fi
fi
echo ""

# ---------------------------------------------------------------------------
echo "=== Summary ==="
echo "PASSED: $PASS"
echo "FAILED: $FAIL"
echo ""
if [ "$FAIL" -gt 0 ]; then
    echo "Result: FAIL"
    exit 1
fi
echo "Result: ALL PASS"
exit 0
