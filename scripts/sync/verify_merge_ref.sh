#!/usr/bin/env bash
# scripts/sync/verify_merge_ref.sh — assert a merged PR's merge commit is
# actually reachable from its base branch ref (issue #5076 AC4).
#
# Gitea can return HTTP 200 + merged=True from POST /pulls/{n}/merge while
# refs/heads/<base> never moves (observed on gitea252 with PR #5075,
# 2026-10-07). Anything trusting merged=True alone gets a false positive:
# the merge "looks" successful but no branch contains the commit.
#
# AC classification for issue #5076 (what this script can / cannot fix):
#   AC1 — after a 200 merge response, base ref must equal merge_commit_sha.
#          Gitea server behavior; NOT enforceable from this repo.
#          This script DETECTS violations (modes --pr / --selftest).
#   AC2 — PATCH /git/refs/heads/<branch> must correct drift (currently
#          HTTP 405). Server capability; UNFIXABLE here. Correction path
#          stays 5remotes_sync.sh Step 3: push temp ref -> docker
#          update-ref -> **git fetch local ref** -> sync. The fetch is
#          mandatory: without it the syncer re-pushes the stale local SHA
#          and rolls the fix back (that happened once already).
#   AC3 — a failed merge must not return 200 / merged=True. Gitea server
#          behavior; NOT enforceable here. This script flags the observable
#          symptom: merged=True but merge_commit_sha unreachable from base.
#   AC4 — regression: construct a merge, then assert
#          base ref == merge_commit_sha. THIS SCRIPT (--selftest builds a
#          throwaway pair of branches, merges via API, asserts, cleans up;
#          --pr N asserts the same property on any existing merged PR).
#
# Usage:
#   scripts/sync/verify_merge_ref.sh --pr N
#       Verify that merged PR N's merge_commit_sha is on its base branch.
#   scripts/sync/verify_merge_ref.sh --selftest
#       Construct a throwaway merge (two selftest/* branches + API PR),
#       assert base ref == merge_commit_sha, then delete everything.
#
# Environment (defaults match the sibling sync scripts):
#   GITEA_URL   default http://192.168.0.252:3000
#   GITEA_USER  default openclaw
#   GITEA_PASS  default details8848
#   GITEA_REPO  default openclaw/sqlrustgo
#
# Exit codes:
#   0 — base ref contains merge_commit_sha
#       (EXACT == strict AC4; LANDED == ancestor, tip moved past it)
#   1 — assertion FAILED: ref drift, PR not merged, or selftest failure
#   2 — network / API error
#   4 — argument error

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

GITEA_URL="${GITEA_URL:-http://192.168.0.252:3000}"
GITEA_USER="${GITEA_USER:-openclaw}"
GITEA_PASS="${GITEA_PASS:-details8848}"
GITEA_REPO="${GITEA_REPO:-openclaw/sqlrustgo}"
AUTH_URL="http://${GITEA_USER}:${GITEA_PASS}@${GITEA_URL#http://}/${GITEA_REPO}.git"
API="${GITEA_URL}/api/v1/repos/${GITEA_REPO}"

MODE=""
PR_N=""

while [ "${1:-}" != "" ]; do
    case "$1" in
        --pr)
            MODE="pr"
            PR_N="${2:-}"
            [ -n "$PR_N" ] || { echo "ERROR: --pr needs a number" >&2; exit 4; }
            shift 2
            ;;
        --selftest)
            MODE="selftest"
            shift
            ;;
        -h|--help)
            sed -n '2,46p' "$0"
            exit 0
            ;;
        *)
            echo "ERROR: unknown option $1" >&2
            exit 4
            ;;
    esac
done

[ -n "$MODE" ] || { echo "ERROR: --pr N or --selftest required (see --help)" >&2; exit 4; }

# api METHOD PATH [JSON_BODY] -> prints HTTP code on stdout, body in $API_BODY
API_BODY="$(mktemp)"
api() {
    local method="$1" path="$2" body="${3:-}"
    local args=(-sS --max-time 15 -u "${GITEA_USER}:${GITEA_PASS}" -o "$API_BODY" -w '%{http_code}' -X "$method")
    if [ -n "$body" ]; then
        args+=(-H 'Content-Type: application/json' -d "$body")
    fi
    curl "${args[@]}" "${API}${path}" 2>/dev/null
}

# verify_core BASE EXPECTED TIP — shared AC4 assertion core.
# exact match -> EXACT (0); ancestor -> LANDED (0); otherwise DRIFT (1)
verify_core() {
    local base="$1" expected="$2" tip="$3"

    if [ "$tip" = "$expected" ]; then
        echo "  RESULT: PASS-EXACT (base ref == merge_commit_sha, strict AC4)"
        return 0
    fi

    echo "  RESULT: base ref $tip != merge_commit_sha $expected"
    echo "  fetching base branch to test reachability..."
    if ! git fetch --no-tags --quiet "$AUTH_URL" \
         "+refs/heads/${base}:refs/verify-merge/base" 2>/dev/null; then
        echo "  ERROR: fetch failed" >&2
        return 2
    fi
    local rc=1
    if ! git cat-file -e "${expected}^{commit}" 2>/dev/null; then
        echo "  RESULT: FAIL-DRIFT — merge commit is NOT reachable from base (code never landed)"
    elif git merge-base --is-ancestor "$expected" refs/verify-merge/base 2>/dev/null; then
        echo "  RESULT: PASS-LANDED — merge_commit_sha is on base history (tip moved past it)"
        rc=0
    else
        echo "  RESULT: FAIL-DRIFT — merge commit exists but is NOT on base history"
    fi
    git update-ref -d refs/verify-merge/base 2>/dev/null
    if [ "$rc" -ne 0 ]; then
        echo "  --- AC classification (issue #5076) ---"
        echo "  AC1/AC3: server-side Gitea behavior — detectable here, not fixable in this repo."
        echo "  AC2:     PATCH /git/refs/heads/* returns 405 — server-side, unfixable here."
        echo "  fix:     scripts/sync/5remotes_sync.sh Step 3 (temp ref -> docker update-ref"
        echo "           -> git fetch local ref -> sync). The fetch step is MANDATORY."
    fi
    return "$rc"
}

if [ "$MODE" = "pr" ]; then
    echo "=== verify_merge_ref --pr $PR_N ==="
    http=$(api GET "/pulls/$PR_N")
    if [ "$http" != "200" ]; then
        echo "ERROR: GET /pulls/$PR_N -> HTTP $http" >&2
        exit 2
    fi
    read -r merged sha base < <(python3 -c "
import json,sys
d=json.load(open('$API_BODY'))
print(d.get('merged', False), d.get('merge_commit_sha') or '-', d.get('base',{}).get('ref','-'))
")
    echo "  merged:   $merged"
    echo "  base:     $base"
    echo "  merge_sha:${sha:0:12}"

    if [ "$merged" != "True" ]; then
        echo "  RESULT: FAIL — PR $PR_N is not merged (merged=$merged)"
        exit 1
    fi
    if [ "$sha" = "-" ] || [ -z "$sha" ]; then
        echo "  RESULT: FAIL — merged=True but merge_commit_sha missing (AC3 symptom)"
        exit 1
    fi

    tip=$(git ls-remote "$AUTH_URL" "refs/heads/$base" 2>/dev/null | awk '{print $1}')
    [ -n "$tip" ] || { echo "ERROR: cannot read refs/heads/$base" >&2; exit 2; }
    echo "  ref tip:  ${tip:0:12}"

    verify_core "$base" "$sha" "$tip"
    exit $?
fi

ts=$(date +%s)
BASE="selftest/merge-ref-verify-${ts}-base"
FEAT="selftest/merge-ref-verify-${ts}-feat"
PR_CREATED=""
MERGED_OK=""

cleanup() {
    if [ -n "$PR_CREATED" ] && [ "$MERGED_OK" != "yes" ]; then
        api DELETE "/pulls/$PR_CREATED" >/dev/null 2>&1 || true
    fi
    git push --no-verify -q "$AUTH_URL" \
        ":refs/heads/$BASE" ":refs/heads/$FEAT" >/dev/null 2>&1 || true
    git update-ref -d refs/verify-merge/base 2>/dev/null || true
}
trap cleanup EXIT

echo "=== verify_merge_ref --selftest (AC4: construct merge, assert base ref == merge_commit_sha) ==="

base_sha=$(git rev-parse origin/develop/v4.1.0 2>/dev/null || git rev-parse HEAD)
echo "  base commit source: ${base_sha:0:12}"

# Build the feat commit with plumbing only — no working-tree mutation.
blob=$(printf 'verify_merge_ref AC4 selftest %s\n' "$ts" | git hash-object -w --stdin)
idx="$(mktemp)"
GIT_INDEX_FILE="$idx" git read-tree "$base_sha"
GIT_INDEX_FILE="$idx" git update-index --add --cacheinfo "100644,$blob,selftest/merge-ref-verify-${ts}.txt"
tree=$(GIT_INDEX_FILE="$idx" git write-tree)
rm -f "$idx"
feat_sha=$(printf 'selftest: verify_merge_ref AC4 payload %s\n' "$ts" | git commit-tree "$tree" -p "$base_sha")

git push --no-verify -q "$AUTH_URL" \
    "$base_sha:refs/heads/$BASE" "$feat_sha:refs/heads/$FEAT" \
    || { echo "ERROR: push selftest branches failed" >&2; exit 2; }
echo "  pushed $BASE / $FEAT"

http=$(api POST "/pulls" "{\"head\":\"$FEAT\",\"base\":\"$BASE\",\"title\":\"[selftest] verify_merge_ref AC4 regression (auto-created, safe to delete)\"}")
if [ "$http" != "201" ]; then
    echo "ERROR: POST /pulls -> HTTP $http" >&2
    exit 2
fi
PR_CREATED=$(python3 -c "import json;print(json.load(open('$API_BODY'))['number'])")
echo "  created PR #$PR_CREATED"

http=$(api POST "/pulls/$PR_CREATED/merge" '{"Do":"merge"}')
echo "  merge POST -> HTTP $http"
if [ "$http" != "200" ]; then
    echo "  RESULT: FAIL — merge API did not return 200 (AC3 check surfaced a failure)"
    exit 1
fi
MERGED_OK="yes"

http=$(api GET "/pulls/$PR_CREATED")
if [ "$http" != "200" ]; then
    echo "ERROR: GET /pulls/$PR_CREATED -> HTTP $http" >&2
    exit 2
fi
read -r merged sha < <(python3 -c "
import json,sys
d=json.load(open('$API_BODY'))
print(d.get('merged', False), d.get('merge_commit_sha') or '-')
")
echo "  merged:   $merged"
echo "  merge_sha:${sha:0:12}"

if [ "$merged" != "True" ]; then
    echo "  RESULT: FAIL — merge POST returned 200 but merged=$merged (AC3 false positive)"
    exit 1
fi
if [ "$sha" = "-" ] || [ -z "$sha" ]; then
    echo "  RESULT: FAIL — merged=True but merge_commit_sha missing (AC3 symptom)"
    exit 1
fi

tip=$(git ls-remote "$AUTH_URL" "refs/heads/$BASE" 2>/dev/null | awk '{print $1}')
[ -n "$tip" ] || { echo "ERROR: cannot read refs/heads/$BASE" >&2; exit 2; }
echo "  ref tip:  ${tip:0:12}"

if [ "$tip" = "$sha" ]; then
    echo "  RESULT: PASS-EXACT — base ref == merge_commit_sha (AC4 satisfied)"
    echo "  PR #$PR_CREATED and selftest branches will be deleted on exit."
    exit 0
fi
echo "  RESULT: FAIL — AC4 violated: merge 200/merged=True but base ref did not move"
echo "  PR #$PR_CREATED, base=$BASE tip=${tip:0:12} expected=${sha:0:12}"
exit 1
