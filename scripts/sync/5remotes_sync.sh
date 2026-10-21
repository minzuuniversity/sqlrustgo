#!/usr/bin/env bash
# scripts/sync/5remotes_sync.sh — bring all 5 remotes to the same tip on
# a given branch (or all main branches).
#
# This is the post-convergence tool for the SQLRustGo 5-remote topology:
#
#   gitcode    — gitcode.com (BreavHeart/sqlrustgo), unprotected, push ok
#   gitea250   — http://192.168.0.250:3000/openclaw/sqlrustgo, branch-protected
#   gitea252   — http://192.168.0.252:3000/openclaw/sqlrustgo, branch-protected
#   gitee      — gitee.com (yinglichina/sqlrustgo), unprotected
#   github     — github.com (yinglichina8848/sqlrustgo-public), unprotected
#
# gitea250 and gitea252 reject direct push on protected branches
# (`enable_push: false`, `enable_force_push: false`,
# `block_admin_merge_override: true`). To advance a protected branch
# we SSH into the container that hosts the repo and run
# `git update-ref` directly. The container ID and repo path are
# resolved at run time by discover_gitea_target(): it finds the
# container whose docker port mapping is 0.0.0.0:3000->3000/tcp, then
# locates the openclaw/sqlrustgo repository inside it. Both are
# discovered fresh on every run because a container restart changes the
# ID, and the two hosts use different repository layouts.
#
# SSH is always non-interactive (`BatchMode=yes`). If authentication
# cannot complete without a prompt the script says so and skips that
# host rather than letting ssh cycle through every identity until the
# server drops it with "Too many authentication failures".
#
# SSH is also made independent of the caller's ssh-agent state
# (`IdentityAgent=none`). When SSH_AUTH_SOCK points at a dead agent —
# macOS launchd leaves the socket file behind after the agent exits —
# OpenSSH blocks on the agent round-trip and produces no output at all.
# That is what made this script hang for 900s on #5380. See the SSH_OPTS
# block below for the measured before/after.
#
# Environment:
#   GITEA_USER      Gitea user for HTTP auth. Required for the
#                   protected-branch path (gitea250 / gitea252).
#   GITEA_TOKEN     Gitea token or password for HTTP auth. Required
#                   for the protected-branch path.
#                   These used to be hardcoded in this file; those values
#                   are live and are tracked as a leak in #5381. There is
#                   no silent fallback to a baked-in credential.
#   GITEA_HOSTS     Space-separated "host:ssh_user" list to sync.
#                   Default: "192.168.0.250:liying 192.168.0.252:liying"
#   GITEA_PUSH_URL  Where to push the temporary ref. Default: derived
#                   from the host and GITEA_USER/GITEA_TOKEN.
#   GIT_SSH_COMMAND If set, it wins — otherwise the script sets it from
#                   its own SSH options so git's SSH channel is equally
#                   immune to a dead agent.
#
# Usage:
#   scripts/sync/5remotes_sync.sh <branch> [<source-remote>]
#       Default source remote: gitea252
#       Example: scripts/sync/5remotes_sync.sh develop/v4.1.0
#                scripts/sync/5remotes_sync.sh main gitea250
#
#   scripts/sync/5remotes_sync.sh --all
#       Sync develop/v4.1.0 + main + release/v4.0.0 (the canonical
#       post-v4.0.0 trio) using gitea252 as the source of truth.
#
# Exit codes:
#   0 — all 5 remotes + local have the same tip
#   1 — drift detected (printed, nothing pushed)
#   2 — protected-branch update failed on a Gitea remote
#   3 — network/SSH error to a Gitea container
#   4 — argument error
#   5 — refused to sync: could not fetch, or the local ref disagrees with what
#       the source remote actually holds (nothing pushed)

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT" || exit 1

ALL_BRANCHES=("develop/v4.1.0" "main" "release/v4.0.0")

# ── SSH 通道选项（唯一来源）────────────────────────────────────────────────
#
# 每一处 ssh 调用都必须经过这里，漏掉一处就复现 #5380 的挂起。
#
# IdentityAgent=none 是本段里唯一不可协商的选项:
# 当 SSH_AUTH_SOCK 指向一个"socket 文件还在但 agent 已经死了"的路径时
# （macOS launchd 的 socket 常常如此），OpenSSH 会去连那个 agent;
# unix socket 只要文件存在 connect() 就不立刻失败，于是 ssh 阻塞在
# agent 往返上且**没有任何输出**。
#
# BatchMode=yes 只禁止交互式口令提示，IdentitiesOnly=yes 只限制提供哪些身份,
# 两者都**不能**切断这条路径。只有 IdentityAgent=none 能让 ssh 完全不去碰 agent。
#
# 实测（本机，agent 已死）:
#   ssh -T git@github.com                              → 挂起，无输出
#   ssh -o IdentityAgent=none -T git@github.com        → 认证成功
#   ssh -o IdentitiesOnly=yes -o IdentityAgent=none \
#       -T git@{github,gitee,gitcode}.com               → 3/3 认证成功
#
# ServerAliveInterval/CountMax 给"连上了但服务端不应答"这条路径封顶 ~45s。
SSH_OPTS=(
    -o BatchMode=yes
    -o ConnectTimeout=10
    -o IdentitiesOnly=yes
    -o IdentityAgent=none
    -o StrictHostKeyChecking=accept-new
    -o ServerAliveInterval=15
    -o ServerAliveCountMax=3
)

# git 的 SSH 通道（push / fetch）走的是 GIT_SSH_COMMAND，不走上面的数组。
# 调用方没有设置时由本脚本设置，否则 git 会继承调用方那个失效的 agent 而挂起。
if [ -z "${GIT_SSH_COMMAND:-}" ]; then
    export GIT_SSH_COMMAND="ssh ${SSH_OPTS[*]}"
fi

# ── Gitea 凭据（外置，不允许硬编码）────────────────────────────────────────
#
# 曾经本脚本内嵌了明文的 Gitea 口令与 token。经实测这些凭据**仍然有效**
# （见 #5381），即仓库读权限 == Gitea 写权限。
# 现在一律从环境变量读取，缺失就明确报错退出——不静默回落到任何硬编码值。
#
# #5520: 环境的缺失并不等于「没有凭据」——本机的 git credential store 里
# 就存着一套可用凭据。解析规则（来源优先级、失败语义、绝不进 URL）由
# scripts/sync/gitea_credential_common.sh 统一定义，三个脚本共用一份实现。
GITEA_USER="${GITEA_USER:-}"
GITEA_TOKEN="${GITEA_TOKEN:-}"
if [ ! -f "${REPO_ROOT}/scripts/sync/gitea_credential_common.sh" ]; then
    echo "ERROR: missing ${REPO_ROOT}/scripts/sync/gitea_credential_common.sh" >&2
    echo "       (the shared credential resolver — see #5520). Refusing to run" >&2
    echo "       without it: a silently absent resolver would look like" >&2
    echo "       'no credential available' and skip every Gitea host." >&2
    exit 2
fi
source "${REPO_ROOT}/scripts/sync/gitea_credential_common.sh"

# 凭据 store 里查哪个 host（252 才是存凭据的那台）。
GITEA_CREDENTIAL_HOST="${GITEA_CREDENTIAL_HOST:-192.168.0.252:3000}"

# ── 覆盖开关 ──────────────────────────────────────────────────────────────
#
# GITEA_HOSTS   同步哪些主机，格式 "host:ssh_user" 空格分隔
# GITEA_PUSH_URL 临时 ref 推到哪（默认由 host + 凭据算出）
#
# 这两个开关既让脚本可被离线测试，也是"只同步部分主机""推到镜像仓库"的真实需求。
GITEA_HOSTS="${GITEA_HOSTS:-192.168.0.250:liying 192.168.0.252:liying}"
GITEA_PUSH_URL="${GITEA_PUSH_URL:-}"

usage() {
    # Print the header comment block (everything between the shebang and the
    # first code line) — a fixed line range would truncate it whenever the
    # header grows.
    sed -n '2,/^set -uo/p' "$0" | sed '$d'
    exit 4
}

source_remote="gitea252"
branches=()

case "${1:-}" in
    --all|"")
        branches=("${ALL_BRANCHES[@]}")
        ;;
    -h|--help)
        usage
        ;;
    *)
        branches=("$1")
        if [ "${2:-}" != "" ]; then
            source_remote="$2"
        fi
        ;;
esac

# Resolve source SHA for each branch (POSIX-portable, no associative arrays)
source_sha_main=""
source_sha_release_v400=""
source_sha_develop_v410=""
source_sha_develop_v400=""

declare -a SOURCE_SHA_VARS=()  # list of variable names to look up later

# Fetch BEFORE reading any SHA, then verify the local ref against what the
# remote actually holds.
#
# This used to read the local "$source_remote/$branch" ref first and fetch
# afterwards. Whenever that ref was stale — the normal state right after a PR
# is merged, because merging updates the server and not this clone — the stale
# SHA was taken as the intended target and force-pushed to every remote,
# silently rewinding the branch. Observed twice in one session:
#
#   PR #5090 merged -> gitea252/develop/v4.1.0 = ed181dfc0b39
#   run this script -> all five remotes rewound to c4aed21d93f4
#
# The verification is the part that matters. `git fetch` can fail, be skipped,
# or race a concurrent merge, and on its own a fetch failure is not an error
# here (the script runs without `set -e`). So after fetching, every branch is
# checked with `git ls-remote`, which asks the server directly and never
# consults a cached ref:
#
#   local ref != server -> refuse (exit 5), never push
#   ls-remote empty    -> refuse (exit 5), never push
#
# A stale ref is indistinguishable from a correct one by inspection alone, so
# this tool must ask. Failing closed is the whole point: "did not sync" is
# recoverable, a rewind on five remotes is not.
echo ""
echo "Step 0: fetch before reading any SHA"
if ! git fetch "$source_remote" --no-tags 2>&1 | tail -3; then
    echo "ERROR: fetch from $source_remote failed; refusing to continue"
    echo "       without a confirmed-fresh view of the source refs"
    exit 5
fi
if ! git fetch --all --no-tags --prune 2>&1 | tail -3; then
    echo "ERROR: 'git fetch --all' failed; refusing to continue"
    exit 5
fi

for branch in "${branches[@]}"; do
    if ! git rev-parse --verify "$source_remote/$branch" >/dev/null 2>&1; then
        echo "ERROR: source remote $source_remote has no $branch"
        exit 4
    fi
    local_sha=$(git rev-parse "$source_remote/$branch")
    actual_sha=$(git ls-remote "$source_remote" "refs/heads/$branch" 2>/dev/null | cut -f1)
    if [ -z "$actual_sha" ]; then
        # ls-remote answers from the server, so an empty answer means the query
        # failed or the branch is gone upstream. Either way we do not know what
        # the remote holds, and pushing the local value is a guess.
        echo "ERROR: could not read refs/heads/$branch from $source_remote"
        echo "       (ls-remote returned nothing). Refusing to sync on a guess."
        exit 5
    fi
    if [ "$local_sha" != "$actual_sha" ]; then
        echo "ERROR: local $source_remote/$branch is $local_sha"
        echo "       but $source_remote actually holds $actual_sha"
        echo "       Refusing to sync: pushing the local value would rewind the"
        echo "       branch on every remote. Fetch and re-run."
        exit 5
    fi
    # Encode branch name -> variable name (POSIX-portable)
    case "$branch" in
        main)                   var="source_sha_main" ;;
        release/v4.0.0)         var="source_sha_release_v400" ;;
        develop/v4.1.0)         var="source_sha_develop_v410" ;;
        develop/v4.0.0)         var="source_sha_develop_v400" ;;
        *)
            echo "ERROR: branch $branch not in allow-list (main, release/v4.0.0, develop/v4.1.0, develop/v4.0.0)"
            exit 4
            ;;
    esac
    sha=$(git rev-parse "$source_remote/$branch")
    eval "$var=\"\$sha\""
    SOURCE_SHA_VARS+=("$var")
    echo "[$branch] source SHA = $sha (from $source_remote)"
done

echo ""
echo "Step 1: (fetch already done in Step 0, before any SHA was read)"

# Step 2: push to unprotected remotes
echo ""
echo "Step 2: push to gitcode / gitee / github (unprotected remotes)"
PUSH_OK=1
for branch in "${branches[@]}"; do
    # Resolve sha via branch->var mapping
    case "$branch" in
        main)            sha="$source_sha_main" ;;
        release/v4.0.0)  sha="$source_sha_release_v400" ;;
        develop/v4.1.0)  sha="$source_sha_develop_v410" ;;
        develop/v4.0.0)  sha="$source_sha_develop_v400" ;;
    esac
    for r in gitcode gitee github; do
        current=$(git rev-parse --verify "$r/$branch" 2>/dev/null || echo MISSING)
        if [ "$current" = "$sha" ]; then
            echo "  $r/$branch: already at $sha ✓"
            continue
        fi
        echo "  $r/$branch: pushing $sha (was ${current:0:12})"
        if git push --force-with-lease "$r" "$sha:refs/heads/$branch" 2>&1 | tail -3; then
            :
        else
            echo "  ERROR: push to $r/$branch failed"
            PUSH_OK=0
        fi
    done
done

# Step 3: SSH container update-ref for protected Gitea remotes
echo ""
echo "Step 3: SSH container update-ref for gitea250 / gitea252"
# Format: "host:ssh_user". The container ID and repo path are NOT
# hardcoded — see discover_gitea_target below. They used to be baked
# into this array while the header comment claimed they were
# "auto-detected"; nothing detected anything. A container restart
# changes the ID, so a stale value makes `docker exec` fail against a
# host that was merely recreated.
ALL_FAILED=0
# GITEA_HOSTS: 空格分隔的 "host:ssh_user"，可用环境变量覆盖
read -r -a GITEA_HOST_LIST <<< "${GITEA_HOSTS}"
# bash 3.2 (still the system bash on macOS) treats an empty array expansion
# under `set -u` as an unbound variable, so refuse an empty list explicitly.
if [ "${#GITEA_HOST_LIST[@]}" -eq 0 ]; then
    echo "ERROR: GITEA_HOSTS is empty — no Gitea host to sync"
    exit 4
fi

# Print "<container_id> <repo_path>" for the container on $host that
# serves gitea on port 3000 and holds the openclaw/sqlrustgo repo.
# Resolved on every run: the path differs between the two hosts
# (/git/... on 250, /data/git/repositories/... on 252) and has changed
# before.
#
# The repo path is found by PROBING the layouts Gitea actually uses, not by
# searching the whole container filesystem. `find / -maxdepth 6` has to walk
# every mount in the container and was measured taking more than 9 minutes on
# 250 — long enough to eat the entire budget of the run that contains it.
# The known layouts cover both hosts; the narrowed `find` stays as a
# fallback for anything else.
discover_gitea_target() {
    local host=$1
    ssh "${SSH_OPTS[@]}" \
        "$host" '
        cid=$(docker ps --format "{{.ID}}|{{.Ports}}" \
              | grep "0.0.0.0:3000->3000/tcp" | head -1 | cut -d"|" -f1)
        [ -n "$cid" ] || { echo "NO_CONTAINER"; exit 1; }
        path=$(docker exec "$cid" bash -c "
            for p in /git/*/sqlrustgo.git \
                     /data/git/repositories/*/sqlrustgo.git \
                     /data/git/gitea-repositories/*/sqlrustgo.git; do
                if [ -d \"\$p\" ]; then echo \"\$p\"; exit 0; fi
            done
            find /git /data/git -maxdepth 4 -type d -name sqlrustgo.git 2>/dev/null | head -1
        ")
        [ -n "$path" ] || { echo "NO_REPO"; exit 1; }
        echo "$cid $path"
    ' 2>/dev/null
}

# Move a commit into a Gitea container's object pool **without any HTTP
# credential**: the objects travel as a git bundle piped over SSH.
#
#   $1 host          $2 ssh user      $3 container id
#   $4 repo path     $5 target sha
#   $6 the remote's current sha (the bundle's prerequisite)
#   $7 temporary ref name to create inside the container
#
# gitea250 rejects the HTTP credential outright (401 on the API and on the
# git handshake) while its SSH channel works, so this is the only way to get
# the objects in there. The update-ref itself still happens over SSH, inside
# the container, because the branch is protected.
transfer_objects_over_ssh() {
    local host="$1" ssh_user="$2" container_id="$3" repo_path="$4"
    local sha="$5" base_sha="$6" tmp_ref="$7"
    local bundle remote_bundle bundle_ref

    bundle=$(mktemp "${TMPDIR:-/tmp}/5remotes-sync.XXXXXX.bundle") || return 1
    remote_bundle="/tmp/5remotes-sync-$$.bundle"
    # A fixed-per-run ref name. The bundle's HEAD has to be a REF NAME: with
    # a bare sha `git bundle create` says "Refusing to create empty bundle"
    # because it cannot name the bundle's head (measured, #5520). The
    # receiving side then has to ask for that exact name.
    bundle_ref="refs/heads/sync-transfer-head-$$"

    # The remote's current sha is the bundle's prerequisite: it is by
    # definition an object the remote already has, so the receiver applies
    # a few-hundred-KB delta instead of the whole history (~33 MB).
    git update-ref "$bundle_ref" "$sha" 2>/dev/null || return 1
    if [ -n "$base_sha" ] && git cat-file -e "${base_sha}^{commit}" 2>/dev/null; then
        git bundle create "$bundle" "$bundle_ref" "^$base_sha" >/dev/null 2>&1 \
            || git bundle create "$bundle" "$bundle_ref" >/dev/null 2>&1 \
            || { git update-ref -d "$bundle_ref"; rm -f "$bundle"; return 1; }
    else
        git bundle create "$bundle" "$bundle_ref" >/dev/null 2>&1 \
            || { git update-ref -d "$bundle_ref"; rm -f "$bundle"; return 1; }
    fi
    git update-ref -d "$bundle_ref"

    if ! ssh "${SSH_OPTS[@]}" "$ssh_user@$host" \
            "docker exec -i $container_id bash -c 'cat > $remote_bundle'" \
            < "$bundle" >/dev/null 2>&1; then
        rm -f "$bundle"
        echo "    (bundle upload to $host failed)"
        return 1
    fi
    rm -f "$bundle"

    # `git fetch <bundle> <head-ref>` — naming the head ref is required:
    # `git fetch <bundle>` alone looks for a HEAD the bundle does not have,
    # and `git fetch <bundle> FETCH_HEAD` reads FETCH_HEAD as the name of
    # a ref to fetch (both measured, #5520).
    if ! ssh "${SSH_OPTS[@]}" "$ssh_user@$host" \
            "docker exec $container_id bash -c 'cd $repo_path && \
             git -c safe.directory=* fetch $remote_bundle $bundle_ref && \
             git -c safe.directory=* update-ref refs/heads/$tmp_ref FETCH_HEAD && \
             rm -f $remote_bundle'" >/dev/null 2>&1; then
        echo "    (bundle fetch inside $host failed)"
        return 1
    fi
    return 0
}

for entry in "${GITEA_HOST_LIST[@]}"; do
    IFS=':' read -r host ssh_user <<< "$entry"
    # BatchMode=yes: never fall back to an interactive password prompt.
    # With BatchMode=no a wrong or missing key makes ssh try every
    # identity in turn until the server drops it with "Too many
    # authentication failures" — which is what happened on 250 while
    # this script still named a user (`z440`) that does not exist
    # there. Auth is expected to work non-interactively; if it cannot,
    # failing fast is the correct outcome.
    #
    # IdentityAgent=none (in SSH_OPTS) is what keeps this from hanging
    # when the caller's SSH_AUTH_SOCK points at a dead agent — see #5380.
    target=$(discover_gitea_target "$host")
    if [ -z "$target" ] || [ "${target%% *}" = "NO_CONTAINER" ] \
       || [ "${target%% *}" = "NO_REPO" ]; then
        echo "  [$host] ERROR: cannot discover container/repo (${target:-no response})"
        ALL_FAILED=1
        continue
    fi
    container_id="${target%% *}"
    repo_path="${target#* }"
    echo "  [$host / $container_id / $repo_path]"

    if ! ssh "${SSH_OPTS[@]}" \
            "$ssh_user@$host" true 2>/dev/null; then
        echo "  [$host] ERROR: non-interactive SSH auth as '$ssh_user' failed. Skipping."
        ALL_FAILED=1
        continue
    fi

    # The protected-branch path needs Gitea credentials. They used to be
    # hardcoded in this script; those values were live and are now tracked
    # as a leak in #5381. They come from the environment — with the local
    # git credential store as a fallback (#5520) — and there is no silent
    # fallback to a baked-in value.
    if ! gitea_credential_resolve "${GITEA_CREDENTIAL_HOST}"; then
        echo "  [$host] ERROR: no Gitea credential available — cannot read or"
        echo "         update a protected branch. Skipping."
        echo "         export GITEA_USER=<user> GITEA_TOKEN=<token>   # see #5381"
        echo "         (or store one for ${GITEA_CREDENTIAL_HOST} in your git"
        echo "          credential store — see #5520)"
        ALL_FAILED=1
        continue
    fi
    # Basic auth header, so the credential never travels inside a URL
    # (a URL leaks into the process table, .git/config and CI logs).
    # Basic encoding accepts either a token or a password as GITEA_TOKEN.
    # The encoding itself lives in the shared helper — one implementation
    # of "how do we turn a credential into a header" (C2 of the helper).
    auth_header="$(gitea_credential_basic_header)"

    for branch in "${branches[@]}"; do
        case "$branch" in
            main)            sha="$source_sha_main" ;;
            release/v4.0.0)  sha="$source_sha_release_v400" ;;
            develop/v4.1.0)  sha="$source_sha_develop_v410" ;;
            develop/v4.0.0)  sha="$source_sha_develop_v400" ;;
        esac
        tmp_ref="tmp-sync-$$-$branch"

        # Read what the remote ACTUALLY holds.
        #
        # Two probes, because the two Gitea hosts differ: 252 accepts the
        # credential, 250 **rejects** it (401 on the API) while the
        # repository itself is anonymously readable. The anonymous probe is
        # therefore a real fallback, not a guess.
        #
        # If BOTH probes fail we do not know what the remote holds. Treating
        # that as "behind" is what made this tool report success while
        # silently skipping the update (#5520) — so it is an error now.
        probe_ref() {
            local mode="$1"
            local url="http://$host:3000/api/v1/repos/openclaw/sqlrustgo/git/refs/heads/$branch"
            if [ "$mode" = "authenticated" ]; then
                curl -sf --max-time 5 -H "$auth_header" "$url" 2>/dev/null
            else
                curl -sf --max-time 5 "$url" 2>/dev/null
            fi | python3 -c "import json,sys;r=json.load(sys.stdin);print((r[0] if isinstance(r,list) else r).get('object',{}).get('sha',''))" 2>/dev/null
        }
        current=$(probe_ref authenticated)
        probe_mode="authenticated"
        if [ -z "$current" ]; then
            current=$(probe_ref anonymous)
            probe_mode="anonymous"
        fi
        if [ -z "$current" ]; then
            echo "    ERROR: cannot read refs/heads/$branch from $host"
            echo "           (the authenticated and the anonymous probe both failed)."
            echo "           Refusing to treat 'unknown' as 'behind'."
            ALL_FAILED=1
            continue
        fi
        if [ "$current" = "$sha" ]; then
            echo "    $branch: already at $sha ✓ (probe: ${probe_mode})"
            continue
        fi

        echo "    $branch: current=${current:0:12}, target=${sha:0:12} (probe: ${probe_mode})"

        # Step 3a: make the commit objects land in the container's object
        # pool. `update-ref` will not pull commits across HTTP.
        #
        # Primary path: HTTP push to a temporary ref. The credential travels
        # in an http.extraheader rather than inside the URL: a credential in
        # the URL gets echoed by git on failure and is the shape that ended
        # up committed in this file before.
        #
        # Fallback path: **gitea250 rejects the HTTP credential outright**
        # (401 on both the API and the git handshake) while its SSH channel
        # works fine — there the objects are carried by a git bundle over
        # SSH instead, which needs no credential at all. Before #5520 the
        # failure of this step ended in a bare `continue`, so the update-ref
        # below never ran and 250 was never advanced.
        push_url="${GITEA_PUSH_URL:-http://${host}:3000/openclaw/sqlrustgo.git}"
        if git -c "http.extraheader=$auth_header" \
             push --no-verify \
             "$push_url" \
             "$sha:refs/heads/$tmp_ref" >/dev/null 2>&1; then
            :
        else
            echo "    HTTP temp-ref push to $host failed — carrying the objects by a bundle over SSH instead"
            if transfer_objects_over_ssh "$host" "$ssh_user" "$container_id" "$repo_path" \
                                       "$sha" "$current" "$tmp_ref"; then
                :
            else
                echo "    ERROR: could not move the objects to $host"
                ALL_FAILED=1
                continue
            fi
        fi

        # Step 3b: SSH into the host and update-ref + delete temp ref
        update_cmd="docker exec $container_id bash -c '
cd $repo_path
git -c safe.directory=* update-ref refs/heads/$branch $sha
git -c safe.directory=* update-ref -d refs/heads/$tmp_ref 2>/dev/null
echo final: \$(cat refs/heads/$branch)
'"
        if ! ssh "${SSH_OPTS[@]}" \
             "$ssh_user@$host" "$update_cmd" 2>&1 | tail -5; then
            echo "    ERROR: ssh update-ref to $host/$branch failed"
            ALL_FAILED=1
        fi
    done
done

echo ""
echo "Step 4: fetch + verify"
git fetch --all --no-tags --prune 2>&1 | tail -3

ALL_OK=1
for branch in "${branches[@]}"; do
    echo ""
    echo "[$branch]"
    for r in gitcode gitea250 gitea252 gitee github; do
        sha=$(git rev-parse --verify "$r/$branch" 2>/dev/null | cut -c1-12)
        echo "  $r = $sha"
    done
    for pair in "gitcode gitea250" "gitcode gitea252" "gitcode gitee" "gitcode github" "gitea250 gitea252" "gitea250 gitee" "gitea250 github" "gitea252 gitee" "gitea252 github" "gitee github"; do
        set -- $pair
        a=$1; b=$2
        ab=$(git rev-list --count "$a/$branch" "^$b/$branch" 2>/dev/null)
        ba=$(git rev-list --count "$b/$branch" "^$a/$branch" 2>/dev/null)
        if [ "$ab" -ne 0 ] || [ "$ba" -ne 0 ]; then
            printf "  ✗ %-9s vs %-9s : ahead=%d behind=%d\n" "$a" "$b" "$ab" "$ba"
            ALL_OK=0
        fi
    done
    [ $ALL_OK -eq 1 ] && echo "  ✓ all 10 pairs consistent"
done

# A failed step must not be masked by a later successful one: report
# the first thing that went wrong, in the order things can go wrong.
if [ "${ALL_FAILED:-0}" -ne 0 ]; then
    echo ""
    echo "FAILED: one or more protected-branch updates did not complete or were"
    echo "        skipped (see above). Refusing to report success: without"
    echo "        completing that step the remotes are not proven to be in sync."
    exit 2
fi
[ $ALL_OK -eq 0 ] && exit 1
[ $PUSH_OK -eq 0 ] && exit 2
exit 0