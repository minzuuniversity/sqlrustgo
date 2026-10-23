#!/usr/bin/env bash
# Sprint 5 v6/v7/v8 Q21 + Q5/Q15/Q16 + Q7/Q8/Q9 — push to Gitea when servers recover.
# Run this after the Gitea 252/250 servers are back online.
#
# #5520 —— 「可能静默 exit 0」的修法：
#
#   1. 原来脚本第一行就 `cd /Users/liying/workspace/dev/openheart/sqlrustgo/
#      .worktrees/v390-merge-q21` —— 那条路径在这台机器上不存在，
#      脚本**每次都在第一行就死掉**（而且死因与「同步」无关，看起来像成功）。
#      现在仓库根目录由脚本自身的位置推导。
#   2. 凭据来源统一走 scripts/sync/gitea_credential_common.sh
#      （环境变量 > 本机 git credential store；取不到就**明确报错退出**，
#      绝不回落到任何写死的值，也绝不把凭据写进 URL）。
#   3. 凭据**不进 argv**：curl 用 `-K <0600 配置文件>`，不再用 `-u "$AUTH"`。
#   4. 失败一律阻断：建 PR 的 curl 之前被 `| python3 …` 吞掉了错误，
#      失败时只打印 "err" 却仍然 exit 0。现在按 HTTP 状态码判定，
#      任何一步失败都以**非 0** 退出。
#
# Exit codes:
#   0 — 全部步骤成功
#   1 — 至少一步失败（推送失败 / 建 PR 失败 / 远端不可达）
#   2 — 没有可用的 Gitea 凭据
#   3 — 用法错误

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT" || exit 3

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
    echo "       export GITEA_USER=<user> GITEA_TOKEN=<token>   # see #5381" >&2
    echo "       (or store one for ${GITEA_CREDENTIAL_HOST} in your git" >&2
    echo "        credential store — see #5520)" >&2
    echo "       Refusing to continue: without a credential the PR call" >&2
    echo "       would answer 401, and 401 is not a PR." >&2
    exit 2
fi

CURL_AUTH_CFG="$(gitea_credential_curl_config_file)" || {
    echo "ERROR: cannot create the temporary curl auth config file" >&2
    exit 2
}
cleanup() { rm -f "$CURL_AUTH_CFG"; }
trap cleanup EXIT

# ── 覆盖开关 ──────────────────────────────────────────────────────────────
#
# GITEA_252_REMOTE / GITEA_250_REMOTE —— 本仓库的 remote 名是
# gitea252 / gitea250（不是 "origin" / "gitea"：origin 在这里指向 250）。
GITEA_252_REMOTE="${GITEA_252_REMOTE:-gitea252}"
GITEA_250_REMOTE="${GITEA_250_REMOTE:-gitea250}"

# GITEA_252_URL / GITEA_250_URL —— 覆盖 API 端点（离线测试用）。
GITEA_252_URL="${GITEA_252_URL:-http://192.168.0.252:3000}"
GITEA_250_URL="${GITEA_250_URL:-http://192.168.0.250:3000}"

BRANCHES=("release/v3.9.0-q21-merge" "fix/v390-q21-multi-col-index" "v3.9.0-q21-gate")
PR_HEAD="release/v3.9.0-q21-merge"
PR_BASE="develop/v3.9.0"

FAILED=0

# Check both Giteas
for entry in "252:$GITEA_252_URL" "250:$GITEA_250_URL"; do
    host="${entry%%:*}"
    base="${entry#*:}"
    if curl -sf -o /dev/null -w "%{http_code}" "$base/" --max-time 3 2>/dev/null | grep -q "200"; then
        echo "Gitea $host is UP"
    else
        echo "Gitea $host is DOWN, skipping"
    fi
done

# Push to Gitea 252 / Gitea 250
for remote in "$GITEA_252_REMOTE" "$GITEA_250_REMOTE"; do
    if ! git remote get-url "$remote" >/dev/null 2>&1; then
        echo "ERROR: remote '$remote' does not exist in this repository — refusing to push" >&2
        FAILED=1
        continue
    fi
    for branch in "${BRANCHES[@]}"; do
        if ! git rev-parse --verify "$branch" >/dev/null 2>&1; then
            echo "SKIP $branch: 本地没有这条分支（不存在就跳过，不是失败）"
            continue
        fi
        echo "Pushing $branch to $remote ($remote)..."
        # 退出码取 git 自己的：`git push … | tail -N` 的退出码是 tail 的。
        push_out=$(git push "$remote" "$branch" 2>&1)
        push_rc=$?
        printf '%s\n' "$push_out" | tail -2
        if [ "$push_rc" -ne 0 ]; then
            echo "ERROR: push $branch to $remote failed (rc=$push_rc)" >&2
            FAILED=1
        fi
    done
done

# Create PR on Gitea 252
if curl -sf -o /dev/null "$GITEA_252_URL/" --max-time 3 2>/dev/null; then
    echo "Creating PR on Gitea 252..."
    body_file="$(mktemp "${TMPDIR:-/tmp}/gitea-pr-body.XXXXXX")"
    cat > "${TMPDIR:-/tmp}/q21-pr.json" << 'JSONEOF'
{
  "head": "release/v3.9.0-q21-merge",
  "base": "develop/v3.9.0",
  "title": "[v390] Sprint 5 v6/v7/v8: Q21 + Q5/Q15/Q16 + Q7/Q8/Q9 fixes → 22/22 PASS, 0 FAIL, 0 SKIP",
  "body": "Merges all 5 Sprint 5 bug fixes (Q21 perf, Q5/Q15/Q16 derived-table, Q7-Q9 EXTRACT, Q8 re-projection) into develop/v3.9.0. See docs/audit/status/2026-06-09-sprint5-merge-report-v390.md for full details.\n\nTag: v3.9.0-q21-gate\nCloses: #2977 (Q21 perf)"
}
JSONEOF
    code=$(curl -s --max-time 30 -X POST \
        -K "$CURL_AUTH_CFG" \
        -H "Content-Type: application/json" \
        --data-binary "@${TMPDIR:-/tmp}/q21-pr.json" \
        -o "$body_file" -w '%{http_code}' \
        "$GITEA_252_URL/api/v1/repos/openclaw/sqlrustgo/pulls" 2>/dev/null)
    rm -f "${TMPDIR:-/tmp}/q21-pr.json"
    if [ "$code" = "201" ] || [ "$code" = "200" ]; then
        python3 -c "import json,sys; d=json.load(sys.stdin); print(f\"PR #{d.get('number','?')} created: {d.get('html_url','err')}\")" < "$body_file" 2>/dev/null || true
    else
        echo "ERROR: PR creation failed (HTTP ${code:-<no answer>})" >&2
        echo "       body: $(head -c 400 "$body_file" 2>/dev/null)" >&2
        FAILED=1
    fi
    rm -f "$body_file"
fi

if [ "$FAILED" -ne 0 ]; then
    echo "FAILED: one or more steps failed — see above. Exiting non-zero on purpose." >&2
    exit 1
fi

echo "DONE"
exit 0
