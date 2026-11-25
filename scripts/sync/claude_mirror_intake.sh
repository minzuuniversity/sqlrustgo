#!/usr/bin/env bash
# scripts/sync/claude_mirror_intake.sh — 收口「远端 Claude 只读分析」闭环。
#
# 闭环的形状（Gitea 252 是发布真源，GitHub 只是镜像）：
#
#   1. 远端 Claude（够不到 252）做只读分析，把结果发布到 GitHub 镜像的一个
#      文档分支上（本脚本用 --source 指定的远端，默认 mz）。
#   2. 本脚本把该分支**读**下来，落到 Gitea 252（同名分支），
#      并在 252 上开 PR —— 评审与合并发生在真源侧，不在镜像侧。
#   3. 合并后由 scripts/sync/5remotes_sync.sh 把 252 的新 tip 回灌到全部远端，
#      最后用 5remotes_drift_check.sh 证明「零漂移」。
#
# 本脚本**刻意不做**的事：
#   - 不 force-push 任何远端。252 是受保护分支；镜像上 force 推送会抹掉别的
#     远端独有的提交 —— 2026-11-26 就发生过：github 仓 develop/v4.0.0 上的
#     3 个 dependabot 提交被 --force-with-lease 覆盖掉（事后靠
#     recovered/develop-v4.0.0-pre-sync-20261126 找回）。
#   - 不在镜像上开 PR、不改镜像内容（镜像只读，AGENTS.md §3.2 / §8 #11）。
#   - 不吞错误：任何一步失败都以非零码退出，并说明卡在哪一步。
#
# 用法：
#   scripts/sync/claude_mirror_intake.sh --list [--source mz]
#       列出镜像上「252 还没有」的候选分支（Claude 新发布的分支会出现在这里）。
#   scripts/sync/claude_mirror_intake.sh --branch <name> [--dry-run]
#       只探测：分支在镜像上的 tip、与 base 的差异、要落地的目标。不写任何远端。
#   scripts/sync/claude_mirror_intake.sh --branch <name> [--land-only]
#       只把分支落到 252（不开 PR、不合并）—— 供无 API 凭据时使用。
#   scripts/sync/claude_mirror_intake.sh --branch <name> [--merge] [--sync]
#       落地到 252 → 开 PR →（--merge）合并 →（--sync）回灌全部远端 → 校验漂移。
#   scripts/sync/claude_mirror_intake.sh --selftest
#       离线自检（本地 bare 仓库，不触网、不碰真远端）。
#
# 环境变量：
#   GITEA_URL   default http://192.168.0.252:3000
#   GITEA_REPO  default openclaw/sqlrustgo
#   GITEA_USER / GITEA_TOKEN
#               Gitea 凭据。也可由本机 git credential store 提供（首选）。
#               取不到时**明确失败**，绝不回落到任何写死的值 —— #5381。
#
# 退出码：
#   0 — 成功（每个被要求的阶段都完成了）
#   1 — 某一步失败（分支不存在 / 分叉 / 合并未落地 / 漂移未清零）
#   2 — 网络 / API / 凭据错误
#   4 — 参数错误

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT" || exit 1

# shellcheck source=scripts/sync/gitea_credential_common.sh
source "${REPO_ROOT}/scripts/sync/gitea_credential_common.sh"

GITEA_URL="${GITEA_URL:-http://192.168.0.252:3000}"
GITEA_REPO="${GITEA_REPO:-openclaw/sqlrustgo}"
API_BASE="${GITEA_URL}/api/v1/repos/${GITEA_REPO}"

SOURCE_NAME="mz"
GITEA_NAME="gitea252"
BASE_BRANCH="develop/v4.1.0"
BRANCH=""
PR_TITLE=""
PR_BODY_FILE=""
MODE="intake"
DO_MERGE="no"
DO_SYNC="no"
DRY_RUN="no"
LAND_ONLY="no"
LIST_PATTERN="refs/heads/docs/*"

GIT_AUTH_CFG=""
CURL_AUTH_CFG=""
API_BODY=""

cleanup() {
    [ -n "$GIT_AUTH_CFG" ] && rm -f "$GIT_AUTH_CFG"
    [ -n "$CURL_AUTH_CFG" ] && rm -f "$CURL_AUTH_CFG"
    [ -n "$API_BODY" ] && rm -f "$API_BODY"
}
trap cleanup EXIT

log() { printf '%s\n' "$*"; }
step() { log ""; log "== $* =="; }
die() {
    local msg="$1" rc="${2:-1}"
    log "ERROR: $msg" >&2
    exit "$rc"
}

usage() { sed -n '2,45p' "$0"; }

# ── 参数 ───────────────────────────────────────────────────────────────────
while [ "${1:-}" != "" ]; do
    case "$1" in
        --branch)       BRANCH="${2:-}"; shift 2 ;;
        --source)       SOURCE_NAME="${2:-}"; shift 2 ;;
        --gitea-remote) GITEA_NAME="${2:-}"; shift 2 ;;
        --base)         BASE_BRANCH="${2:-}"; shift 2 ;;
        --title)        PR_TITLE="${2:-}"; shift 2 ;;
        --body-file)    PR_BODY_FILE="${2:-}"; shift 2 ;;
        --list)         MODE="list"; shift ;;
        --selftest)     MODE="selftest"; shift ;;
        --dry-run)      DRY_RUN="yes"; shift ;;
        --land-only)    LAND_ONLY="yes"; shift ;;
        --merge)        DO_MERGE="yes"; shift ;;
        --sync)         DO_SYNC="yes"; shift ;;
        -h|--help)      usage; exit 0 ;;
        *)              die "unknown option: $1" 4 ;;
    esac
done

[ "$MODE" != "intake" ] || [ -n "$BRANCH" ] || die "--branch <name> is required (see --help)" 4

# 分支名必须是普通 ref 名：不含 '..'、不以 '-' 开头、不连续斜杠、字符集受限。
valid_branch_name() {
    case "$1" in
        ""|-*|*/|.|..|*..*|*/*/*) return 1 ;;
        *[!A-Za-z0-9._/-]*) return 1 ;;
        *) return 0 ;;
    esac
}

# remote 名 → URL/路径；也接受一个本地路径（自检用）。
resolve_remote() {
    local name="$1" url
    if url="$(git remote get-url "$name" 2>/dev/null)" && [ -n "$url" ]; then
        printf '%s\n' "$url"
        return 0
    fi
    if [ -d "$name" ]; then
        printf '%s\n' "$name"
        return 0
    fi
    return 1
}

remote_sha() {  # remote ref -> sha（空 = 该 ref 不存在）
    git ls-remote "$1" "$2" 2>/dev/null | awk 'NR==1{print $1}'
}

# ── 凭据（只经 include.path / curl -K，绝不进 argv 或 URL） ───────────────
setup_credentials() {
    local url="$1"
    case "$url" in
        http://*|https://*) ;;
        *) return 0 ;;   # 本地路径（自检）不需要凭据
    esac
    local host
    host="$(printf '%s' "$url" | sed -E 's#^[a-z]+://([^/]+)/.*#\1#')"
    gitea_credential_resolve "$host" \
        || die "no Gitea credential for $host — set GITEA_USER/GITEA_TOKEN or store it in the git credential store (#5381)" 2
    GIT_AUTH_CFG="$(gitea_credential_git_config_file)" || die "cannot build the git auth config" 2
    CURL_AUTH_CFG="$(gitea_credential_curl_config_file)" || die "cannot build the curl auth config" 2
}

# api METHOD PATH [BODY] → HTTP code（stdout）；响应体在 $API_BODY
api() {
    local method="$1" path="$2" body="${3:-}"
    API_BODY="$(mktemp "${TMPDIR:-/tmp}/claude-intake-api.XXXXXX")" || return 1
    chmod 600 "$API_BODY" 2>/dev/null
    local args=(-sS --max-time 30 -o "$API_BODY" -w '%{http_code}' -X "$method" -H 'Content-Type: application/json')
    [ -n "$CURL_AUTH_CFG" ] && args+=(-K "$CURL_AUTH_CFG")
    [ -n "$body" ] && args+=(--data-binary "$body")
    curl "${args[@]}" "${API_BASE}${path}" 2>/dev/null
}

push_gitea() {  # <refspec>
    if [ -n "$GIT_AUTH_CFG" ]; then
        git -c include.path="$GIT_AUTH_CFG" push --no-verify -q "$GITEA_NAME" "$1"
    else
        git push --no-verify -q "$GITEA_NAME" "$1"
    fi
}

json_field() {
    python3 -c "import json,sys;d=json.load(open(sys.argv[1]));v=d.get(sys.argv[2]);print('' if v is None else v)" \
        "$API_BODY" "$1"
}

# ── --list：镜像上有、252 上还没有的候选分支 ──────────────────────────────
do_list() {
    local src
    src="$(resolve_remote "$SOURCE_NAME")" || die "source remote '$SOURCE_NAME' not found" 4
    step "candidate branches on '$SOURCE_NAME' that '$GITEA_NAME' does not have"
    local n=0 sha ref name have
    while read -r sha ref; do
        [ -n "${sha:-}" ] || continue
        name="${ref#refs/heads/}"
        have="$(remote_sha "$GITEA_NAME" "refs/heads/$name")"
        [ -n "$have" ] && continue
        printf '  %s\t%s\n' "${sha:0:12}" "$name"
        n=$((n + 1))
    done < <(git ls-remote "$src" "$LIST_PATTERN" 2>/dev/null)
    log ""
    log "  $n candidate(s). Next: --branch <name> --dry-run"
    [ "$n" -gt 0 ] || log "  (none — nothing new from the mirror)"
    return 0
}

# ── 主流程 ───────────────────────────────────────────────────────────────
do_intake() {
    local src gitea
    src="$(resolve_remote "$SOURCE_NAME")" || die "source remote '$SOURCE_NAME' not found" 4
    gitea="$(resolve_remote "$GITEA_NAME")" || die "gitea remote '$GITEA_NAME' not found" 4
    valid_branch_name "$BRANCH" || die "invalid branch name: '$BRANCH'" 4
    valid_branch_name "$BASE_BRANCH" || die "invalid base branch name: '$BASE_BRANCH'" 4

    step "PREFLIGHT"
    log "  source remote : $SOURCE_NAME → $src"
    log "  gitea remote  : $GITEA_NAME → $gitea"
    log "  base branch   : $BASE_BRANCH"
    log "  branch        : $BRANCH"
    [ "$DRY_RUN" = "yes" ] && log "  mode          : DRY-RUN — no remote will be written"
    setup_credentials "$gitea"

    step "DISCOVER"
    local src_sha base_sha counts
    src_sha="$(remote_sha "$src" "refs/heads/$BRANCH")"
    [ -n "$src_sha" ] || die "branch '$BRANCH' does not exist on '$SOURCE_NAME'" 1
    base_sha="$(remote_sha "$gitea" "refs/heads/$BASE_BRANCH")"
    [ -n "$base_sha" ] || die "base branch '$BASE_BRANCH' does not exist on '$GITEA_NAME'" 1
    log "  source tip : $src_sha"
    log "  base tip   : $base_sha"

    local ref
    ref="refs/intake/$(printf '%s' "$BRANCH" | tr '/' '_')"
    git fetch --no-tags --quiet "$src" "+refs/heads/$BRANCH:$ref" \
        || die "fetch of '$BRANCH' from '$SOURCE_NAME' failed" 2
    git fetch --no-tags --quiet "$gitea" "+refs/heads/$BASE_BRANCH:$ref.base" \
        || die "fetch of '$BASE_BRANCH' from '$GITEA_NAME' failed" 2
    counts="$(git rev-list --left-right --count "$ref.base...$ref" 2>/dev/null)"
    log "  base-only / branch-only commits: ${counts:-?}"
    log "  files changed vs base:"
    git diff --name-status "$ref.base...$ref" 2>/dev/null | sed 's/^/    /' | head -60

    step "LAND on '$GITEA_NAME'"
    local exist_sha
    exist_sha="$(remote_sha "$gitea" "refs/heads/$BRANCH")"
    if [ -n "$exist_sha" ]; then
        if [ "$exist_sha" = "$src_sha" ]; then
            log "  '$BRANCH' already on '$GITEA_NAME' at the same tip — nothing to push"
        else
            die "'$BRANCH' already exists on '$GITEA_NAME' at a different tip ($exist_sha). This script never force-pushes; reconcile the two tips by hand first." 1
        fi
    elif [ "$DRY_RUN" = "yes" ]; then
        log "  [dry-run] would push $src_sha → refs/heads/$BRANCH on '$GITEA_NAME'"
    else
        push_gitea "$src_sha:refs/heads/$BRANCH" || die "push to '$GITEA_NAME' failed" 1
        local after
        after="$(remote_sha "$gitea" "refs/heads/$BRANCH")"
        [ "$after" = "$src_sha" ] || die "push reported success but '$GITEA_NAME' holds '${after:-<none>}'" 1
        log "  pushed $src_sha → refs/heads/$BRANCH (read-back verified)"
    fi

    [ "$LAND_ONLY" = "yes" ] && { step "LAND-ONLY requested — stopping before the PR"; return 0; }
    [ "$DRY_RUN" = "yes" ] && { step "[dry-run] would open PR head=$BRANCH base=$BASE_BRANCH"; return 0; }

    step "OPEN PR on $GITEA_REPO"
    local pr_num="" http
    http="$(api GET "/pulls?state=open&limit=50")"
    if [ "$http" = "200" ]; then
        pr_num="$(python3 - "$API_BODY" "$BRANCH" "$BASE_BRANCH" <<'PY'
import json, sys
try:
    rows = json.load(open(sys.argv[1]))
except Exception:
    rows = []
head, base = sys.argv[2], sys.argv[3]
for r in rows if isinstance(rows, list) else []:
    if r.get("head", {}).get("ref") == head and r.get("base", {}).get("ref") == base:
        print(r.get("number", ""))
        break
PY
)"
        [ -n "$pr_num" ] && log "  reusing the open PR #$pr_num for head=$BRANCH base=$BASE_BRANCH"
    fi
    if [ -z "$pr_num" ]; then
        local title body payload
        title="${PR_TITLE:-[intake] $BRANCH → $BASE_BRANCH}"
        body=""
        if [ -n "$PR_BODY_FILE" ]; then
            [ -f "$PR_BODY_FILE" ] || die "body file not found: $PR_BODY_FILE" 4
            body="$(cat "$PR_BODY_FILE")"
        else
            body="由 scripts/sync/claude_mirror_intake.sh 自动开单：把镜像 '$SOURCE_NAME' 的 '$BRANCH' 落到 252。"
        fi
        payload="$(mktemp "${TMPDIR:-/tmp}/claude-intake-pr.XXXXXX")"
        python3 - "$payload" "$title" "$body" "$BRANCH" "$BASE_BRANCH" <<'PY'
import json, sys
out, title, body, head, base = sys.argv[1:6]
with open(out, "w") as fh:
    json.dump({"head": head, "base": base, "title": title, "body": body}, fh)
PY
        http="$(api POST "/pulls" "$(cat "$payload")")"
        rm -f "$payload"
        [ "$http" = "201" ] || die "POST /pulls → HTTP $http: $(head -c 200 "$API_BODY")" 2
        pr_num="$(json_field number)"
        log "  created PR #$pr_num"
    fi
    [ -n "$pr_num" ] || die "could not determine the PR number" 1

    if [ "$DO_MERGE" != "yes" ]; then
        step "STOP after PR #$pr_num (pass --merge to merge, --sync to回灌)"
        log "  review: ${GITEA_URL}/${GITEA_REPO}/pulls/$pr_num"
        return 0
    fi

    step "MERGE PR #$pr_num"
    http="$(api POST "/pulls/$pr_num/merge" '{"Do":"merge"}')"
    [ "$http" = "200" ] || die "merge API → HTTP $http: $(head -c 200 "$API_BODY")" 1
    local merge_sha tip verdict
    merge_sha="$(json_field merge_commit_sha)"
    tip="$(remote_sha "$gitea" "refs/heads/$BASE_BRANCH")"
    if [ -n "$merge_sha" ] && [ "$tip" = "$merge_sha" ]; then
        verdict="PASS-EXACT"
    else
        git fetch --no-tags --quiet "$gitea" "+refs/heads/$BASE_BRANCH:$ref.merged" 2>/dev/null
        if [ -n "$merge_sha" ] && git merge-base --is-ancestor "$merge_sha" "$ref.merged" 2>/dev/null; then
            verdict="PASS-LANDED"
        else
            die "merge returned HTTP $http but '$BASE_BRANCH' does not contain the merge commit (tip=${tip:-<none>}, merge=${merge_sha:-<none>})" 1
        fi
    fi
    log "  merge commit : ${merge_sha:-<none>}"
    log "  base tip now : $tip"
    log "  verdict      : $verdict"

    if [ "$DO_SYNC" != "yes" ]; then
        step "STOP after merge (pass --sync to回灌全部远端)"
        return 0
    fi

    step "SYNC all remotes from '$GITEA_NAME'"
    if ! GITEA_USER="${GITEA_USER:-}" GITEA_TOKEN="${GITEA_TOKEN:-}" \
         bash "${REPO_ROOT}/scripts/sync/5remotes_sync.sh" "$BASE_BRANCH"; then
        die "5remotes_sync.sh failed — mirrors are NOT converged" 1
    fi
    log "  5remotes_sync.sh exit 0"

    step "DRIFT CHECK"
    if ! bash "${REPO_ROOT}/scripts/sync/5remotes_drift_check.sh"; then
        die "drift check did not return 0 — mirrors are NOT converged" 1
    fi
    log "  5remotes_drift_check.sh exit 0"
    return 0
}

# ── --selftest：离线自检（本地 bare 仓库，不触网） ────────────────────────
do_selftest() {
    local tmp
    tmp="$(mktemp -d "${TMPDIR:-/tmp}/claude-intake-selftest.XXXXXX")"
    local mirror="$tmp/mirror.git" gitea="$tmp/gitea.git" work="$tmp/work"
    git init --bare -q "$mirror"
    git init --bare -q "$gitea"
    git init -q "$work"
    (
        cd "$work" || exit 1
        git config user.email selftest@example.invalid
        git config user.name selftest
        printf 'base\n' > base.md
        git add base.md
        git commit -q -m "selftest: base"
        git push -q "$gitea" HEAD:refs/heads/develop/v4.1.0
        git push -q "$mirror" HEAD:refs/heads/develop/v4.1.0
        printf 'selftest payload\n' > report.md
        git add report.md
        git commit -q -m "selftest: analysis branch"
        git push -q "$mirror" HEAD:refs/heads/docs/selftest-intake
    ) || die "selftest setup failed" 1

    local branch="docs/selftest-intake"
    local sha
    sha="$(remote_sha "$mirror" "refs/heads/$branch")"
    [ -n "$sha" ] || die "selftest: the mirror branch was not created" 1

    log "=== T1: --dry-run must not write to the gitea remote ==="
    if "$0" --branch "$branch" --source "$mirror" --gitea-remote "$gitea" --dry-run >/dev/null 2>&1; then
        log "  dry-run exit 0 ✓"
    else
        die "selftest T1: dry-run failed" 1
    fi
    if [ -z "$(remote_sha "$gitea" "refs/heads/$branch")" ]; then
        log "  gitea remote still has no branch ✓ (no write happened)"
    else
        die "selftest T1: dry-run wrote to the gitea remote" 1
    fi

    log "=== T2: --land-only must put the same tip on the gitea remote ==="
    if ! "$0" --branch "$branch" --source "$mirror" --gitea-remote "$gitea" --land-only >/dev/null 2>&1; then
        die "selftest T2: land-only failed" 1
    fi
    local landed
    landed="$(remote_sha "$gitea" "refs/heads/$branch")"
    if [ "$landed" = "$sha" ]; then
        log "  landed tip matches the mirror tip ✓"
    else
        die "selftest T2: landed tip '${landed:-<none>}' != mirror tip $sha" 1
    fi

    log "=== T3: a divergent tip on the gitea remote must be REFUSED (no force) ==="
    (
        cd "$work" || exit 1
        printf 'divergent\n' > other.md
        git add other.md
        git commit -q -m "selftest: divergent tip"
        git push -q "$gitea" HEAD:refs/heads/$branch
    ) || die "selftest setup failed" 1
    local divergent
    divergent="$(remote_sha "$gitea" "refs/heads/$branch")"
    if "$0" --branch "$branch" --source "$mirror" --gitea-remote "$gitea" --land-only >/dev/null 2>&1; then
        die "selftest T3: the script accepted a divergent tip (it would have force-pushed)" 1
    fi
    local still
    still="$(remote_sha "$gitea" "refs/heads/$branch")"
    if [ "$still" = "$divergent" ]; then
        log "  the divergent tip is untouched ✓ (refused, nothing overwritten)"
    else
        die "selftest T3: the divergent tip was overwritten" 1
    fi

    log "=== T4: a missing branch must fail with a clear error ==="
    if "$0" --branch "docs/does-not-exist" --source "$mirror" --gitea-remote "$gitea" --dry-run >/dev/null 2>&1; then
        die "selftest T4: a missing branch was accepted" 1
    fi
    log "  missing branch rejected ✓"

    log "=== T5: an invalid branch name must exit 4 ==="
    "$0" --branch "../evil" --source "$mirror" --gitea-remote "$gitea" --dry-run >/dev/null 2>&1
    local rc=$?
    if [ "$rc" -eq 4 ]; then
        log "  invalid branch name → exit 4 ✓"
    else
        die "selftest T5: expected exit 4, got $rc" 1
    fi

    rm -rf "$tmp"
    git for-each-ref --format='%(refname)' 'refs/intake/*' | while read -r r; do
        git update-ref -d "$r" 2>/dev/null
    done
    log ""
    log "SELFTEST: PASS (5/5)"
    return 0
}

case "$MODE" in
    list)     do_list ;;
    selftest) do_selftest ;;
    intake)   do_intake ;;
    *)        die "unknown mode: $MODE" 4 ;;
esac
