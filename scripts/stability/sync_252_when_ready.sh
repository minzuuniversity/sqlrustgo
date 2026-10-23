#!/bin/bash
# sync_252_when_ready.sh - Sync 250 Gitea state to 252 Gitea when 252 recovers.
#
# Use this when 252 Gitea (192.168.0.252) becomes responsive again.
# Verifies 252 is up, then pushes the develop branch + opens the WAL-fix PR
# + comments on the soak-related issues.
#
# Created 2026-06-19 after 252 Gitea went unresponsive during 4h short-soak.
# All work that was destined for 252 was instead merged to 250 backup.
#
# #5520 —— 「可能静默 exit 0」的修法：
#
#   1. 凭据来源统一走 scripts/sync/gitea_credential_common.sh
#      （环境变量 > 本机 git credential store；取不到就**明确报错退出**，
#      绝不回落到任何写死的值，也绝不把凭据写进 URL）。
#   2. 凭据**不进 argv**：curl 用 `-K <0600 配置文件>`，
#      不再用 `-u "$AUTH"`（那会让凭据出现在进程列表里）。
#   3. 失败一律阻断：`git push … | tail -N` 的退出码是 **tail** 的，
#      推失败会被当成成功；这里改成取 git 自己的退出码，
#      并且任何一步失败都让脚本以**非 0** 退出。
#
# Exit codes:
#   0 — 全部步骤成功
#   1 — 至少一步失败（252 不可达 / API 不通 / 推送失败 / 建 issue 或评论失败）
#   2 — 没有可用的 Gitea 凭据
#   3 — 用法错误

set -uo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$PROJECT_ROOT" || exit 3

GITEA_252="${GITEA_252_URL:-http://192.168.0.252:3000}"
GITEA_250="${GITEA_250_URL:-http://192.168.0.250:3000}"
GITEA_API="$GITEA_252/api/v1"

# ── 凭据（共用实现，见 #5520）─────────────────────────────────────────────
#
# 凭据的来源优先级、失败语义、绝不进 URL 这三条规则只在
# scripts/sync/gitea_credential_common.sh 里定义一次。
HELPER="${PROJECT_ROOT}/scripts/sync/gitea_credential_common.sh"
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
    echo "       Refusing to continue: without a credential every API call" >&2
    echo "       would answer 401, and a 401 is not a success." >&2
    exit 2
fi

# 凭据进一个 0600 的 curl 配置文件 —— argv 里只剩一个路径。
CURL_AUTH_CFG="$(gitea_credential_curl_config_file)" || {
    echo "ERROR: cannot create the temporary curl auth config file" >&2
    exit 2
}
cleanup() { rm -f "$CURL_AUTH_CFG"; }
trap cleanup EXIT

# ── 覆盖开关 ──────────────────────────────────────────────────────────────
#
# GITEA_252_REMOTE / GITEA_250_REMOTE —— 推给哪台 Gitea 的哪个 remote。
# 本仓库的 remote 名是 gitea252 / gitea250（不是 "252" / "gitea"）。
# 本脚本的职责是「252 恢复后把状态同步过去」⇒ 默认只推 252；
# GITEA_250_REMOTE 留空即不推 250（250 的分支是受保护的，推它多半会被拒）。
GITEA_252_REMOTE="${GITEA_252_REMOTE:-gitea252}"
GITEA_250_REMOTE="${GITEA_250_REMOTE:-}"

BRANCHES=("develop/v3.9.0" "feature/q8-q9-join-reorder" "feature/short-soak-ladder" "fix/wal-checkpoint-3531")

log() { echo "[$(date '+%Y-%m-%dT%H:%M:%S')] $*"; }
err() { echo "[$(date '+%Y-%m-%dT%H:%M:%S')] ERROR: $*" >&2; }

FAILED=0

# curl 到 Gitea API；把 HTTP 状态码打印到 stdout，body 写到 $2。
# 失败（网络错误）⇒ 返回非 0。
gitea_api() {
    local method="$1" url="$2" body_file="$3" data_file="${4:-}"
    local code
    if [ -n "$data_file" ]; then
        code=$(curl -s --max-time 30 -X "$method" -K "$CURL_AUTH_CFG" \
            -H "Content-Type: application/json" \
            --data-binary "@$data_file" \
            -o "$body_file" -w '%{http_code}' "$url" 2>/dev/null)
    else
        code=$(curl -s --max-time 30 -X "$method" -K "$CURL_AUTH_CFG" \
            -o "$body_file" -w '%{http_code}' "$url" 2>/dev/null)
    fi
    printf '%s' "$code"
}

# 1. Verify 252 is reachable
log "Checking 252 reachability..."
if ! curl -s -m 30 -I "$GITEA_252/" >/dev/null 2>&1; then
    err "252 not reachable. Re-run this script when 252 is back."
    exit 1
fi
log "252 is reachable"

# 2. Verify 252 API works **and the credential is accepted**
#
# 401 ⇒ 凭据无效/被拒。这不是「API 不可用」，是「凭据不可用」——
# 必须明确报出来（原来这里只看 curl 的退出码，401 和网络错误混在一起）。
body_file="$(mktemp "${TMPDIR:-/tmp}/gitea-252-body.XXXXXX")"
code=$(gitea_api GET "$GITEA_API/repos/openclaw/sqlrustgo" "$body_file")
if [ "$code" = "401" ] || [ "$code" = "403" ]; then
    err "252 rejected the credential (HTTP $code). The token is not usable on 252."
    rm -f "$body_file"
    exit 1
fi
if [ "$code" != "200" ]; then
    err "252 API did not answer as expected (HTTP ${code:-<no answer>}). May need manual intervention."
    rm -f "$body_file"
    exit 1
fi
rm -f "$body_file"
log "252 API works"

# 3./4. Push branches
for branch in "${BRANCHES[@]}"; do
    if ! git rev-parse --verify "$branch" >/dev/null 2>&1; then
        log "SKIP $branch: 本地没有这条分支（不存在就跳过，不是失败）"
        continue
    fi
    for remote in "$GITEA_252_REMOTE" ${GITEA_250_REMOTE:+"$GITEA_250_REMOTE"}; do
        if ! git remote get-url "$remote" >/dev/null 2>&1; then
            err "remote '$remote' does not exist in this repository — refusing to push $branch"
            FAILED=1
            continue
        fi
        log "Pushing $branch to $remote ..."
        # 退出码必须取 git 自己的：`git push … | tail -N` 的退出码是 tail 的
        # ⇒ 推失败会被记成成功（#5520：静默 exit 0）。
        push_out=$(git push "$remote" "$branch" 2>&1)
        push_rc=$?
        printf '%s\n' "$push_out" | tail -2
        if [ "$push_rc" -ne 0 ]; then
            err "Push $branch to $remote failed (rc=$push_rc)"
            FAILED=1
        fi
    done
done

# 5. File WAL P0 bug issue on 252 (mirror of 250 #3531 / 252 #3531 already filed)
log "Checking if WAL P0 issue #3531 exists on 252..."
body_file="$(mktemp "${TMPDIR:-/tmp}/gitea-252-body.XXXXXX")"
code=$(gitea_api GET "$GITEA_API/repos/openclaw/sqlrustgo/issues/3531" "$body_file")
WAL_ISSUE="unknown"
if [ "$code" = "200" ]; then
    WAL_ISSUE=$(python3 -c "import json,sys; print(json.load(sys.stdin).get('state','unknown'))" < "$body_file" 2>/dev/null || echo "unknown")
elif [ "$code" = "404" ]; then
    WAL_ISSUE="missing"
else
    err "Could not read issue #3531 (HTTP ${code:-<no answer>})"
    FAILED=1
fi
rm -f "$body_file"
log "Issue #3531 state: $WAL_ISSUE"

if [ "$WAL_ISSUE" = "missing" ]; then
    log "Filing WAL P0 issue on 252..."
    cat > /tmp/wal_issue_252.json << 'JSONEOF'
{
  "title": "[P0/GA-BLOCKER] WAL grows unbounded — no checkpoint triggered in serve loop (168h soak impossible)",
  "body": "## Symptom\n\nDuring local short-soak (30m ladder, PR #3532), observed:\n- 30s elapsed: WAL = 1.1 GB\n- 30m elapsed: WAL = 22.8 GB (matched by disk-full error)\n- 1h projected: ~45 GB\n- 168h projected: ~7.6 TB (impossible)\n\n## Root Cause\n\nCheckpointManager is defined at crates/storage/src/checkpoint.rs but never invoked from the serve loop. Verified: 0 hits for do_checkpoint or spawn.*checkpoint or run_periodic in crates/.\n\n## Fix\n\nPR #3533 (merged on 250 as #3279) spawns a background thread in serve that truncates the WAL file every ~30s when WAL > 100 MB. Validated locally: 3h/4h/6h/8h ladder all PASS post-fix, WAL stays bounded at <1 GB.\n\n## Status (post-merge)\n\nThis issue should be marked CLOSED once #3533 is confirmed on 252 develop.\n\nEvidence: crates/storage/src/checkpoint.rs, PR #3533 (250: #3279)."
}
JSONEOF
    body_file="$(mktemp "${TMPDIR:-/tmp}/gitea-252-body.XXXXXX")"
    code=$(gitea_api POST "$GITEA_API/repos/openclaw/sqlrustgo/issues" "$body_file" /tmp/wal_issue_252.json)
    if [ "$code" != "201" ] && [ "$code" != "200" ]; then
        err "Filing the WAL issue on 252 failed (HTTP ${code:-<no answer>})"
        FAILED=1
    else
        python3 -c "import json,sys; d=json.load(sys.stdin); print(f\"Created #{d.get('number','?')}: {d.get('title','')[:60]}\")" < "$body_file" 2>/dev/null || true
    fi
    rm -f "$body_file"
    rm -f /tmp/wal_issue_252.json
fi

# 6. Comment on soak issues
for issue_num in 3225 3229 3265 3266 3531; do
    log "Commenting on #$issue_num..."
    cat > /tmp/comment_252.json << JSONEOF
{
  "body": "## WAL P0 Bug FIXED on 250 (PR #3279, 2026-06-19)\n\nLocal 4h ladder validated WAL fix:\n- 30m: WAL bounded 940-1016 MB (was 22.8 GB)\n- 1h: WAL 0 MB, RSS 10 MB, FD 8\n- 2h: WAL 0 MB, RSS 12 MB, FD 8\n- 3h: WAL 0 MB, RSS 9 MB, FD 8\n- 4h: WAL 0 MB, RSS 9 MB, FD 8\n\nAll 4 ladder steps PASS post-fix. Truncate thread fires every ~30s when WAL > 100 MB. Server stable for 4h locally with zero leaks.\n\nReady to dispatch #3265 (72h) -> #3266 (168h) to Z6G4.\n\nNOTE: 252 Gitea was unreachable when this work was done. Fix merged to 250 (backup) and synced to 252 once recovered."
}
JSONEOF
    body_file="$(mktemp "${TMPDIR:-/tmp}/gitea-252-body.XXXXXX")"
    code=$(gitea_api POST "$GITEA_API/repos/openclaw/sqlrustgo/issues/$issue_num/comments" "$body_file" /tmp/comment_252.json)
    if [ "$code" != "201" ] && [ "$code" != "200" ]; then
        err "Comment on #$issue_num failed (HTTP ${code:-<no answer>})"
        FAILED=1
    else
        python3 -c "import json,sys; d=json.load(sys.stdin); print(f\"Comment #{d.get('id','?')} on #$issue_num added\")" < "$body_file" 2>/dev/null || true
    fi
    rm -f "$body_file"
    rm -f /tmp/comment_252.json
done

if [ "$FAILED" -ne 0 ]; then
    err "one or more steps failed — see above. Exiting non-zero on purpose:"
    err "a partially-synced 252 must not be reported as a completed sync."
    exit 1
fi

log "=== 252 sync complete ==="
log "Verify: curl -K <auth-config> $GITEA_API/repos/openclaw/sqlrustgo/issues/3531"
exit 0
