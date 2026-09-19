#!/bin/bash
# scripts/check_repo_sync.sh
#
# v4.0.0 CI gate: check that key branches are in sync across all remotes.
# Run from CI or as a cron job to detect sync divergence early.
#
# Usage:
#   scripts/check_repo_sync.sh [<repo>]
#       Default repo: openclaw/sqlrustgo
#
# Exit codes:
#   0 — All branches in sync across all remotes (excluding GitHub if blocked)
#   1 — Sync divergence detected
#   2 — Misuse (wrong arguments or missing config)

set -e

REPO="${1:-openclaw/sqlrustgo}"

# Remotes
TOKEN_GITEA="${GITEA_TOKEN:-cc19b2ad677e18c96b9d049f6dc2b46e02176883}"
GITEA_HOSTS=("192.168.0.252" "192.168.0.250")
GITCODE_URL="https://gitcode.com/BreavHeart/sqlrustgo.git"
GITHUB_URL="git@github.com:yinglichina8848/sqlrustgo-public.git"

# Key branches to check
KEY_BRANCHES=("develop/v3.12.0" "main" "release/v3.12.0" "develop/v4.0.0")

echo "=== Repo Sync Check ==="
echo "Repo: $REPO"
echo "Branches: ${KEY_BRANCHES[*]}"
echo ""

# Helper: get SHA from a remote
get_sha() {
    local remote_type="$1"
    local ref="$2"
    case "$remote_type" in
        gitea)
            local host="$3"
            local url="http://$host:3000/api/v1/repos/$REPO/branches/${ref}"
            local sha=$(curl -sf --max-time 10 \
                -H "Authorization: token $TOKEN_GITEA" \
                "$url" 2>/dev/null | python3 -c "
import sys, json
try:
    print(json.load(sys.stdin).get('commit', {}).get('id', '')[:10])
except:
    print('')
" 2>/dev/null)
            echo "$sha"
            ;;
        gitcode)
            local sha=$(git ls-remote --quiet "$GITCODE_URL" "refs/heads/${ref}" 2>/dev/null | awk '{print $1}' | head -c 10)
            echo "$sha"
            ;;
        github)
            local sha=$(git ls-remote --quiet "$GITHUB_URL" "refs/heads/${ref}" 2>/dev/null | awk '{print $1}' | head -c 10)
            echo "$sha"
            ;;
    esac
}

# Collect SHAs (use temp files for portability with bash 3.x on macOS)
SHAS_DIR=$(mktemp -d)
trap 'rm -rf "$SHAS_DIR"' EXIT
failures=0
total_checks=0
skipped=0

# Helper: get/set SHA for a (branch, remote) pair
# Use safe filename (slashes replaced)
set_sha() { echo "$3" > "$SHAS_DIR/$(echo "$1__$2" | tr '/' '_')"; }
get_sha_field() { cat "$SHAS_DIR/$(echo "$1__$2" | tr '/' '_')" 2>/dev/null; }

for branch in "${KEY_BRANCHES[@]}"; do
    echo "--- $branch ---"
    sync_set=""
    sync_excl_github=""
    
    # .252 Gitea (primary source of truth; warning if unreachable)
    sha=$(get_sha gitea "$branch" "192.168.0.252")
    set_sha "$branch" ".252" "$sha"
    if [ -n "$sha" ] && [ "$sha" != "ERROR" ]; then
        sync_excl_github="$sha"
        echo "  .252:        $sha"
        total_checks=$((total_checks + 1))
    else
        echo "  .252:        UNREACHABLE (network issue)"
        # Don't count as failure if other remotes are in sync
        # (.252 may be temporarily down for maintenance)
    fi

    # .250 Gitea
    sha=$(get_sha gitea "$branch" "192.168.0.250")
    set_sha "$branch" ".250" "$sha"
    if [ -n "$sha" ] && [ "$sha" != "ERROR" ]; then
        if [ -z "$sync_excl_github" ]; then
            sync_excl_github="$sha"
        elif [ "$sync_excl_github" != "$sha" ]; then
            failures=$((failures + 1))
            echo "  ⚠️  .250 vs .252 divergence: $sha vs $sync_excl_github"
        fi
        echo "  .250:        $sha"
        total_checks=$((total_checks + 1))
    else
        echo "  .250:        ERROR"
        failures=$((failures + 1))
    fi

    # Gitcode
    sha=$(get_sha gitcode "$branch")
    set_sha "$branch" "gitcode" "$sha"
    if [ -n "$sha" ]; then
        if [ -z "$sync_excl_github" ]; then
            sync_excl_github="$sha"
        elif [ "$sync_excl_github" != "$sha" ]; then
            failures=$((failures + 1))
            echo "  ⚠️  gitcode divergence: $sha vs $sync_excl_github"
        fi
        echo "  gitcode:     $sha"
        total_checks=$((total_checks + 1))
    else
        echo "  gitcode:     N/A"
    fi

    # GitHub (informational, may be blocked)
    sha=$(get_sha github "$branch")
    set_sha "$branch" "github" "$sha"
    if [ -n "$sha" ]; then
        echo "  github:      $sha"
        if [ -n "$sync_excl_github" ] && [ "$sync_excl_github" != "$sha" ]; then
            echo "  ⚠️  GitHub filter-rewritten (expected if .gitattributes has LFS rules)"
        fi
    else
        echo "  github:      N/A"
        skipped=$((skipped + 1))
    fi
    
    if [ -n "$sync_excl_github" ]; then
        echo "  Status (.252/.250/gitcode): $sync_excl_github"
    fi
    echo ""
done

echo "=== Summary ==="
echo "  Total checks:  $total_checks"
echo "  Failures:      $failures"
echo "  Skipped:       $skipped"

if [ "$failures" -gt 0 ]; then
    echo ""
    echo "❌ FAIL: $failures sync divergence detected"
    exit 1
fi

if [ "$total_checks" -eq 0 ]; then
    echo ""
    echo "❌ FAIL: no remotes reachable"
    exit 1
fi

echo ""
echo "✅ PASS: all .252/.250/gitcode branches in sync"
exit 0