#!/bin/bash
# scripts/check_repo_sync.sh
#
# v4.0.0 CI gate: check that key branches are in sync across all remotes.
# Run from CI or as a cron job to detect sync divergence early.
#
# #5520 —— 「可能静默 exit 0」的修法：
#
#   1. **区分三种回答**（原来只有两种，而且混在一起）：
#        200 + SHA → 分支存在，这是它的 SHA
#        404       → 分支**不存在**（远端根本没有这条分支）
#        401/403   → **凭据被拒**（原来被当成「网络不可达」而**不算失败**）
#        其它/超时 → 网络错误
#      原来：.252 查不到就打印 "UNREACHABLE (network issue)" 并且**不计入失败**，
#      .250 查不到才算失败 ⇒ 只剩 .250 可达时它会报「PASS」。
#      现在：**查不到就是查不到**，一律算失败并以非 0 退出；
#      「远端没有这条分支」单独报为 N/A（合法的状态，不是网络问题）。
#   2. 凭据来源统一走 scripts/sync/gitea_credential_common.sh
#      （环境变量 > 本机 git credential store；取不到就**明确报错退出**，
#      绝不回落到任何写死的值，也绝不把凭据写进 URL）。
#   3. 凭据**不进 argv**：curl 用 `-K <0600 配置文件>`。
#
# Usage:
#   scripts/check_repo_sync.sh [<repo>]
#       Default repo: openclaw/sqlrustgo
#
# Exit codes:
#   0 — All branches in sync across all remotes
#   1 — Sync divergence detected, or a remote could not be queried
#   2 — Misuse (wrong arguments or missing config)
#   3 — No usable Gitea credential

set -uo pipefail

REPO="${1:-openclaw/sqlrustgo}"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT" || exit 2

# ── 凭据（共用实现，见 #5520）─────────────────────────────────────────────
HELPER="${REPO_ROOT}/scripts/sync/gitea_credential_common.sh"
if [ ! -f "$HELPER" ]; then
    echo "ERROR: missing $HELPER (the shared credential resolver — see #5520)" >&2
    exit 2
fi
# shellcheck source=/dev/null
source "$HELPER"

GITEA_CREDENTIAL_HOST="${GITEA_CREDENTIAL_HOST:-192.168.0.252:3000}"
if ! gitea_credential_resolve "${GITEA_CREDENTIAL_HOST}"; then
    echo "ERROR: no Gitea credential available for ${GITEA_CREDENTIAL_HOST}." >&2
    echo "       export GITEA_TOKEN=<token>   # see #5381" >&2
    echo "       (or store one for ${GITEA_CREDENTIAL_HOST} in your git" >&2
    echo "        credential store — see #5520)" >&2
    echo "       Refusing to continue: a query without a credential answers 401," >&2
    echo "       and 'cannot tell' must never be reported as 'in sync'." >&2
    exit 3
fi

CURL_AUTH_CFG="$(gitea_credential_curl_config_file)" || {
    echo "ERROR: cannot create the temporary curl auth config file" >&2
    exit 3
}
cleanup() { rm -f "$CURL_AUTH_CFG"; }
trap cleanup EXIT

# Remotes
GITEA_HOSTS=("192.168.0.252" "192.168.0.250")
GITCODE_URL="https://gitcode.com/BreavHeart/sqlrustgo.git"
GITHUB_URL="git@github.com:yinglichina8848/sqlrustgo-public.git"

# Key branches to check
KEY_BRANCHES=("develop/v4.1.0" "develop/v3.12.0" "main" "release/v3.12.0" "develop/v4.0.0")

echo "=== Repo Sync Check ==="
echo "Repo: $REPO"
echo "Branches: ${KEY_BRANCHES[*]}"
echo ""

# 查询一个远端的某条分支。
#
# 输出 "<http_code> <sha>"：
#   200 + sha → 分支存在
#   404       → 分支不存在
#   401/403   → 凭据被拒
#   其它/空   → 查不到（网络错误等）
#
# 「分支不存在」与「查不到」必须分开（#5520 §4.A.1）——
# 原来这两个都塌成「空 SHA」，而空 SHA 在 .252 上还被当成
# 「不可达，不算失败」⇒ 一次真正的分歧可以报成 PASS。
query_gitea_branch() {
    local host="$1" ref="$2" body_file code
    body_file="$(mktemp "${TMPDIR:-/tmp}/repo-sync-body.XXXXXX")" || { echo "000 "; return 1; }
    code=$(curl -s --max-time 10 -K "$CURL_AUTH_CFG" \
        -o "$body_file" -w '%{http_code}' \
        "http://$host:3000/api/v1/repos/$REPO/branches/${ref}" 2>/dev/null)
    if [ "$code" = "401" ] || [ "$code" = "403" ]; then
        # 这台机器拒绝当前凭据（250 实测就是 401），但它的仓库**匿名可读**
        # ⇒ 匿名再问一次。匿名也问不到 ⇒ 才是「查不到」。
        # （这与 5remotes_sync.sh 的二段式探测是同一条规则。）
        rm -f "$body_file"
        body_file="$(mktemp "${TMPDIR:-/tmp}/repo-sync-body.XXXXXX")" || { echo "000 "; return 1; }
        code=$(curl -s --max-time 10 \
            -o "$body_file" -w '%{http_code}' \
            "http://$host:3000/api/v1/repos/$REPO/branches/${ref}" 2>/dev/null)
    fi
    if [ "$code" = "200" ]; then
        local sha
        sha=$(python3 -c "
import sys, json
try:
    print(json.load(sys.stdin).get('commit', {}).get('id', '')[:10])
except Exception:
    print('')
" < "$body_file" 2>/dev/null)
        rm -f "$body_file"
        printf '200 %s' "$sha"
        return 0
    fi
    rm -f "$body_file"
    printf '%s ' "${code:-000}"
    return 0
}

# Helper: get SHA from a remote
get_sha() {
    local remote_type="$1"
    local ref="$2"
    case "$remote_type" in
        gitea)
            local host="$3"
            local result code sha
            result=$(query_gitea_branch "$host" "$ref")
            code="${result%% *}"
            sha="${result#* }"
            case "$code" in
                200) echo "$sha"; return 0 ;;
                404) echo "NO_BRANCH"; return 0 ;;
                401|403) echo "AUTH_FAILED"; return 0 ;;
                *) echo "UNREACHABLE"; return 0 ;;
            esac
            ;;
        gitcode)
            local out rc sha
            out=$(git ls-remote --quiet "$GITCODE_URL" "refs/heads/${ref}" 2>/dev/null); rc=$?
            if [ "$rc" -ne 0 ]; then
                echo "UNREACHABLE"
            else
                sha=$(printf '%s\n' "$out" | awk '{print $1}' | head -c 10)
                if [ -z "$sha" ]; then echo "NO_BRANCH"; else echo "$sha"; fi
            fi
            ;;
        github)
            local out rc sha
            out=$(git ls-remote --quiet "$GITHUB_URL" "refs/heads/${ref}" 2>/dev/null); rc=$?
            if [ "$rc" -ne 0 ]; then
                echo "UNREACHABLE"
            else
                sha=$(printf '%s\n' "$out" | awk '{print $1}' | head -c 10)
                if [ -z "$sha" ]; then echo "NO_BRANCH"; else echo "$sha"; fi
            fi
            ;;
    esac
}

# Collect SHAs (use temp files for portability with bash 3.x on macOS)
SHAS_DIR=$(mktemp -d)
trap 'rm -rf "$SHAS_DIR"; rm -f "$CURL_AUTH_CFG"' EXIT
failures=0
total_checks=0
skipped=0
queries_failed=0

# Helper: get/set SHA for a (branch, remote) pair
# Use safe filename (slashes replaced)
set_sha() { echo "$3" > "$SHAS_DIR/$(echo "$1__$2" | tr '/' '_')"; }
get_sha_field() { cat "$SHAS_DIR/$(echo "$1__$2" | tr '/' '_')" 2>/dev/null; }

for branch in "${KEY_BRANCHES[@]}"; do
    echo "--- $branch ---"

    # .252 Gitea (primary source of truth)
    sha=$(get_sha gitea "$branch" "192.168.0.252")
    set_sha "$branch" ".252" "$sha"
    v252="$sha"
    case "$sha" in
        NO_BRANCH)
            echo "  .252:        NO_BRANCH (该远端没有这条分支)"
            total_checks=$((total_checks + 1))
            ;;
        AUTH_FAILED)
            echo "  .252:        AUTH FAILED (HTTP 401/403 —— 凭据被拒)"
            failures=$((failures + 1))
            queries_failed=$((queries_failed + 1))
            ;;
        UNREACHABLE)
            echo "  .252:        UNREACHABLE (查不到 —— 不算「一致」，算失败)"
            failures=$((failures + 1))
            queries_failed=$((queries_failed + 1))
            ;;
        *)
            echo "  .252:        $sha"
            total_checks=$((total_checks + 1))
            ;;
    esac

    # .250 Gitea
    sha=$(get_sha gitea "$branch" "192.168.0.250")
    set_sha "$branch" ".250" "$sha"
    v250="$sha"
    case "$sha" in
        NO_BRANCH)
            echo "  .250:        NO_BRANCH (该远端没有这条分支)"
            total_checks=$((total_checks + 1))
            ;;
        AUTH_FAILED)
            echo "  .250:        AUTH FAILED (HTTP 401/403 —— 凭据被拒)"
            failures=$((failures + 1))
            queries_failed=$((queries_failed + 1))
            ;;
        UNREACHABLE)
            echo "  .250:        UNREACHABLE (查不到 —— 不算「一致」，算失败)"
            failures=$((failures + 1))
            queries_failed=$((queries_failed + 1))
            ;;
        *)
            echo "  .250:        $sha"
            total_checks=$((total_checks + 1))
            ;;
    esac

    # Gitcode
    sha=$(get_sha gitcode "$branch")
    set_sha "$branch" "gitcode" "$sha"
    vgitcode="$sha"
    if [ "$sha" = "NO_BRANCH" ]; then
        echo "  gitcode:     NO_BRANCH (该远端没有这条分支)"
        total_checks=$((total_checks + 1))
    elif [ -z "$sha" ] || [ "$sha" = "UNREACHABLE" ]; then
        echo "  gitcode:     UNREACHABLE (查不到 —— 不算「一致」，算失败)"
        failures=$((failures + 1))
        queries_failed=$((queries_failed + 1))
    else
        echo "  gitcode:     $sha"
        total_checks=$((total_checks + 1))
    fi

    # GitHub (informational, may be blocked)
    sha=$(get_sha github "$branch")
    set_sha "$branch" "github" "$sha"
    if [ "$sha" = "NO_BRANCH" ]; then
        echo "  github:      NO_BRANCH (该镜像没有这条分支)"
        total_checks=$((total_checks + 1))
    elif [ -z "$sha" ] || [ "$sha" = "UNREACHABLE" ]; then
        echo "  github:      UNREACHABLE (查不到 —— 镜像，不计入失败)"
        skipped=$((skipped + 1))
    else
        echo "  github:      $sha"
    fi

    # ── 分歧判定 ──────────────────────────────────────────────────────────
    #
    # 「有 SHA」和「没有这条分支」是**两种不同的状态**，
    # 一个远端没有分支、另一个有 ⇒ 那是分歧（该推没推），
    # 不能因为「先取到的那个是空的」就把分歧吃掉。
    # 「查不到」（UNREACHABLE / AUTH_FAILED）不参与比较 ——
    # 它们已经各自记成失败了。
    ref=""
    for v in "$v252" "$v250" "$vgitcode"; do
        case "$v" in
            UNREACHABLE|AUTH_FAILED|"") continue ;;
        esac
        if [ -z "$ref" ]; then
            ref="$v"
        elif [ "$ref" != "$v" ]; then
            failures=$((failures + 1))
            echo "  ⚠️  divergence: $v vs $ref"
        fi
    done

    if [ -n "$ref" ]; then
        echo "  Status (.252/.250/gitcode): $ref"
    fi
    echo ""
done

echo "=== Summary ==="
echo "  Total checks:  $total_checks"
echo "  Failures:      $failures"
echo "  Skipped:       $skipped"

if [ "$failures" -gt 0 ]; then
    echo ""
    if [ "$queries_failed" -gt 0 ]; then
        echo "❌ FAIL: $failures sync divergence detected（其中 ${queries_failed} 处是「查不到」—— 查不到不是一致）"
    else
        echo "❌ FAIL: $failures sync divergence detected"
    fi
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
