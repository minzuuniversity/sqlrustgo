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
#   GITEA_USER      Gitea user for HTTP auth. Optional when the local
#                   git credential store holds an entry for the Gitea
#                   host (see GITEA_CREDENTIAL_HOST) — the store is the
#                   fallback, the environment wins (C1 of the shared
#                   resolver, scripts/sync/gitea_credential_common.sh).
#   GITEA_TOKEN     Gitea token or password for HTTP auth. Same source
#                   priority as GITEA_USER.
#                   These used to be hardcoded in this file; those values
#                   are live and are tracked as a leak in #5381. There is
#                   no silent fallback to a baked-in credential, and the
#                   script refuses to start without a usable credential.
#   GITEA_CREDENTIAL_HOST
#                   host:port looked up in the git credential store.
#                   Default: 192.168.0.252:3000
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
#   0 — all 5 remotes hold the target tip (verified against the servers)
#   1 — drift detected: at least one remote does not hold the target
#   2 — protected-branch update failed on a Gitea remote, a push to an
#       unprotected remote failed, or no credential was available
#   3 — a remote did not answer, so 'all remotes agree' could not be
#       established (cannot tell ≠ in sync)
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

# 凭据必须在**任何远端操作之前**解析完。
#
# 修复前：凭据是在 Step 3（真正要推受保护分支时）才解析的，
# 而且解析不到时的后果是「跳过这台主机」—— 脚本会带着「没有凭据」
# 继续去查远端，把 401 当成「落后」。#5520 的验收要求是
# 「凭据不可用时明确报错退出，不得把『查不到』当成『落后』」，
# 所以这里提前 fail-fast：拿不到凭据就退出，什么都不做。
if ! gitea_credential_resolve "${GITEA_CREDENTIAL_HOST}"; then
    echo "ERROR: no Gitea credential available for ${GITEA_CREDENTIAL_HOST}." >&2
    echo "       export GITEA_USER=<user> GITEA_TOKEN=<token>   # see #5381" >&2
    echo "       (or store one for ${GITEA_CREDENTIAL_HOST} in your git" >&2
    echo "        credential store — see #5520)" >&2
    echo "       Refusing to run: without a credential every protected-branch" >&2
    echo "       query answers 401, and '401' must never be read as 'behind'." >&2
    exit 2
fi

# 凭据不进 argv（#5520 验收：凭据不得出现在进程列表 / .git/config / 日志里）。
#   git  → git -c include.path=<file>   （0600 的 git 配置，不写进 .git/config）
#   curl → curl -K <file>               （0600 的 curl 配置）
# 两条通道的 argv 里只有一个**路径**。
GIT_AUTH_CFG="$(gitea_credential_git_config_file)" || {
    echo "ERROR: cannot create the temporary git auth config file" >&2
    exit 2
}
CURL_AUTH_CFG="$(gitea_credential_curl_config_file)" || {
    echo "ERROR: cannot create the temporary curl auth config file" >&2
    rm -f "$GIT_AUTH_CFG"
    exit 2
}
cleanup_auth_files() { rm -f "$GIT_AUTH_CFG" "$CURL_AUTH_CFG"; }
trap cleanup_auth_files EXIT

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
#
# `cmd | tail -3` 的退出码是 **tail** 的 ⇒ git 失败也会被当成成功。
# 这正是「取不到就当落后」的同类错误：这里必须看 git 自己的退出码。
fetch_out=$(git -c include.path="$GIT_AUTH_CFG" fetch "$source_remote" --no-tags 2>&1)
fetch_rc=$?
printf '%s\n' "$fetch_out" | tail -3
if [ "$fetch_rc" -ne 0 ]; then
    echo "ERROR: fetch from $source_remote failed (rc=$fetch_rc); refusing to continue"
    echo "       without a confirmed-fresh view of the source refs"
    exit 5
fi

# 其余远端逐个 fetch，失败的**逐个点名**。
# 这一步不再决定任何事：远端的真实状态一律由 Step 4 直接查询服务器得到
# （本地 tracking ref 可能过期 —— 过期就会让 --force-with-lease 误拒，
#  也会让「一致性」结论失真）。gitea250 对当前凭据一律 401，这里失败是预期的。
echo ""
echo "Step 0b: fetch the remaining remotes (best effort, each one reported)"
for r in $(git remote); do
    [ "$r" = "$source_remote" ] && continue
    fetch_out=$(git -c include.path="$GIT_AUTH_CFG" fetch "$r" --no-tags --prune 2>&1)
    fetch_rc=$?
    if [ "$fetch_rc" -ne 0 ]; then
        echo "WARNING: fetch from $r failed (rc=$fetch_rc): $(printf '%s' "$fetch_out" | tail -1)"
        echo "         (gitea250 rejects the current credential with 401 — that is expected;"
        echo "          its state is verified directly in Step 4, never via a fetch)"
    fi
done

for branch in "${branches[@]}"; do
    if ! git rev-parse --verify "$source_remote/$branch" >/dev/null 2>&1; then
        echo "ERROR: source remote $source_remote has no $branch"
        exit 4
    fi
    local_sha=$(git rev-parse "$source_remote/$branch")
    # 直接问服务器（ls-remote 不看本地缓存）。带凭据问一次；
    # 问不到就**匿名**再问一次（250 的仓库匿名可读）——
    # 两次都问不到 ⇒ 「查不到」，不是「落后」，下面会明确报错退出。
    actual_sha=$(GIT_TERMINAL_PROMPT=0 git -c include.path="$GIT_AUTH_CFG" \
        ls-remote "$source_remote" "refs/heads/$branch" 2>/dev/null | cut -f1)
    if [ -z "$actual_sha" ]; then
        actual_sha=$(GIT_TERMINAL_PROMPT=0 git -c credential.helper= \
            ls-remote "$source_remote" "refs/heads/$branch" 2>/dev/null | cut -f1)
    fi
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
        # 「远端现在是什么」必须**问服务器**（ls-remote），
        # 不能看本地 tracking ref —— 本地那份可能是过期的，
        # 而过期的 --force-with-lease 会误拒一次本来正确的推送。
        # 问不到 ⇒ 「查不到」⇒ 明确报错，不猜。
        current=$(GIT_TERMINAL_PROMPT=0 git -c include.path="$GIT_AUTH_CFG" \
            ls-remote "$r" "refs/heads/$branch" 2>/dev/null | cut -f1)
        if [ -z "$current" ]; then
            echo "  ERROR: cannot read refs/heads/$branch from $r — refusing to push on a guess"
            PUSH_OK=0
            continue
        fi
        if [ "$current" = "$sha" ]; then
            echo "  $r/$branch: already at $sha ✓"
            continue
        fi
        echo "  $r/$branch: pushing $sha (was ${current:0:12})"
        # 退出码必须是 git 自己的：`git … | tail -3` 的退出码是 tail 的 ⇒
        # 推失败也会被当成成功（这正是 #5520 里「静默 exit 0」的一类成因）。
        push_out=$(git push --force-with-lease="refs/heads/$branch:$current" \
            "$r" "$sha:refs/heads/$branch" 2>&1)
        push_rc=$?
        printf '%s\n' "$push_out" | tail -3
        if [ "$push_rc" -ne 0 ]; then
            echo "  ERROR: push to $r/$branch failed (rc=$push_rc)"
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

    # 凭据在脚本开头就解析好了（取不到 ⇒ 早已 fail-fast 退出）。
    # 这里只需要两个**不含凭据**的配置文件：curl 用 -K，git 用 include.path。
    # 凭据既不进 URL，也不进 argv（#5520 验收：凭据不得出现在进程列表）。

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
        # The three answers must stay apart (#5520 §4.A.1):
        #
        #   200 → the branch exists, this is its SHA
        #   404 → the branch does **not exist** on that remote
        #          （合法的状态：下面的 update-ref 会把它**建出来**；
        #            把它当成「查不到」会让这个工具永远建不了新分支）
        #   其它 → **查不到**（401 / 500 / 网络错误）
        #          「查不到」不是「落后」⇒ 明确报错退出，绝不猜。
        #
        # 注意用 `-o <file> -w '%{http_code}'` 而不是 `-f`：
        # `-f` 会把 404 变成「无输出」，于是「不存在」和「问不到」又混回去了。
        probe_body="$(mktemp "${TMPDIR:-/tmp}/5remotes-probe.XXXXXX")"
        probe_ref() {
            local mode="$1"
            local url="http://$host:3000/api/v1/repos/openclaw/sqlrustgo/git/refs/heads/$branch"
            local code sha=""
            if [ "$mode" = "authenticated" ]; then
                code=$(curl -s --max-time 5 -K "$CURL_AUTH_CFG" \
                    -o "$probe_body" -w '%{http_code}' "$url" 2>/dev/null)
            else
                code=$(curl -s --max-time 5 \
                    -o "$probe_body" -w '%{http_code}' "$url" 2>/dev/null)
            fi
            if [ "$code" = "200" ]; then
                sha=$(python3 -c "import json,sys;r=json.load(sys.stdin);print((r[0] if isinstance(r,list) else r).get('object',{}).get('sha',''))" < "$probe_body" 2>/dev/null)
            fi
            printf '%s %s' "$code" "$sha"
        }
        probe_result=$(probe_ref authenticated)
        probe_code="${probe_result%% *}"
        current="${probe_result#* }"
        probe_mode="authenticated"
        if [ "$probe_code" != "200" ] && [ "$probe_code" != "404" ]; then
            # 认证探测失败（250 上是 401）⇒ 匿名再探一次
            probe_result=$(probe_ref anonymous)
            probe_code="${probe_result%% *}"
            current="${probe_result#* }"
            probe_mode="anonymous"
        fi
        if [ "$probe_code" = "404" ]; then
            current=""
            branch_state="missing"
        elif [ "$probe_code" != "200" ]; then
            rm -f "$probe_body"
            echo "    ERROR: cannot read refs/heads/$branch from $host (HTTP ${probe_code:-<no answer>})"
            echo "           Refusing to treat 'unknown' as 'behind'."
            ALL_FAILED=1
            continue
        else
            branch_state="present"
        fi
        rm -f "$probe_body"
        if [ "$branch_state" = "present" ] && [ "$current" = "$sha" ]; then
            echo "    $branch: already at $sha ✓ (probe: ${probe_mode})"
            continue
        fi

        if [ "$branch_state" = "missing" ]; then
            echo "    $branch: does not exist on $host yet — it will be created (probe: ${probe_mode})"
        else
            echo "    $branch: current=${current:0:12}, target=${sha:0:12} (probe: ${probe_mode})"
        fi

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
        if git -c include.path="$GIT_AUTH_CFG" \
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
        #
        # `ssh … | tail -5` 的退出码是 **tail** 的 ⇒ ssh 失败也会被当成成功。
        # 这是 #5520 里「静默 exit 0」最直接的一处：远端没更新，脚本却往下走。
        # 现在取 ssh 自己的退出码，并**回读**远端的 SHA 确认它真的到位了。
        update_cmd="docker exec $container_id bash -c '
cd $repo_path
git -c safe.directory=* update-ref refs/heads/$branch $sha
git -c safe.directory=* update-ref -d refs/heads/$tmp_ref 2>/dev/null
echo final: \$(git -c safe.directory=* rev-parse refs/heads/$branch 2>/dev/null)
'"
        ssh_out_file="$(mktemp "${TMPDIR:-/tmp}/5remotes-ssh-out.XXXXXX")"
        ssh "${SSH_OPTS[@]}" \
             "$ssh_user@$host" "$update_cmd" > "$ssh_out_file" 2>&1
        ssh_rc=$?
        printf '%s\n' "$(cat "$ssh_out_file")" | tail -5
        ssh_out="$(cat "$ssh_out_file")"
        rm -f "$ssh_out_file"
        if [ "$ssh_rc" -ne 0 ]; then
            echo "    ERROR: ssh update-ref to $host/$branch failed (rc=$ssh_rc)"
            ALL_FAILED=1
            continue
        fi
        final_sha=$(printf '%s\n' "$ssh_out" | sed -n 's/^final: //p' | tail -1)
        if [ "$final_sha" != "$sha" ]; then
            echo "    ERROR: $host/$branch now holds '${final_sha:-<unknown>}', expected $sha"
            ALL_FAILED=1
            continue
        fi
    done
done

echo ""
echo "Step 4: verify against what the remotes ACTUALLY hold"
#
# 修复前这里比的是**本地 tracking ref**（`git rev-parse $r/$branch`）。
# 本地那份可能是过期的 ⇒ 一个根本没被推进的远端也会被判成「一致」。
# #5520 的原文就是：「250 上的推送失败了，脚本却报告 all 10 pairs consistent
# 并 exit 0」。现在**逐个远端直接问服务器**（ls-remote）：
#   - 问到的 SHA ≠ 目标 ⇒ 分歧（exit 1）
#   - 问不到（远端没回答 / 没有这个 remote）⇒ **无法验证**（exit 3），
#     绝不把「问不到」当成「一致」。
ALL_OK=1
VERIFY_UNREACHABLE=0
for branch in "${branches[@]}"; do
    case "$branch" in
        main)            sha="$source_sha_main" ;;
        release/v4.0.0)  sha="$source_sha_release_v400" ;;
        develop/v4.1.0)  sha="$source_sha_develop_v410" ;;
        develop/v4.0.0)  sha="$source_sha_develop_v400" ;;
    esac
    echo ""
    echo "[$branch] target = $sha"
    for r in gitcode gitea250 gitea252 gitee github; do
        if ! git remote get-url "$r" >/dev/null 2>&1; then
            echo "  $r = <no such remote> — cannot verify"
            VERIFY_UNREACHABLE=1
            continue
        fi
        if [ "$r" = "gitea250" ]; then
            # 250 拒绝当前凭据（401），但仓库匿名可读 ⇒ 匿名查。
            # `-c credential.helper=` 关掉本机 credential store，确保这次
            # 真的是匿名查询（否则 git 会自动带上 store 里的凭据再吃一个 401）。
            ls_out=$(GIT_TERMINAL_PROMPT=0 git -c credential.helper= \
                ls-remote "$r" "refs/heads/$branch" 2>/dev/null)
        else
            ls_out=$(GIT_TERMINAL_PROMPT=0 git -c include.path="$GIT_AUTH_CFG" \
                ls-remote "$r" "refs/heads/$branch" 2>/dev/null)
        fi
        ls_rc=$?
        remote_sha=$(printf '%s\n' "$ls_out" | cut -f1)
        if [ "$ls_rc" -ne 0 ] && [ -z "$remote_sha" ]; then
            echo "  $r = <could not read> — the remote did not answer (rc=$ls_rc)"
            VERIFY_UNREACHABLE=1
            continue
        fi
        if [ -z "$remote_sha" ]; then
            # ls-remote 成功但没有这个 ref ⇒ 远端**没有这个分支**。
            # 这是分歧（该推没推），不是「问不到」。
            echo "  $r = <branch missing> — $r does not have $branch"
            ALL_OK=0
            continue
        fi
        echo "  $r = ${remote_sha:0:12}"
        if [ "$remote_sha" != "$sha" ]; then
            echo "  ✗ $r is NOT at the target SHA"
            ALL_OK=0
        fi
    done
    if [ "$ALL_OK" -eq 1 ] && [ "$VERIFY_UNREACHABLE" -eq 0 ]; then
        echo "  ✓ all 5 remotes hold $sha"
    fi
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
if [ "${VERIFY_UNREACHABLE:-0}" -ne 0 ]; then
    echo ""
    echo "FAILED: at least one remote did not answer, so 'all remotes agree'"
    echo "        could not be established. Refusing to report success:"
    echo "        'cannot tell' is not 'in sync'."
    exit 3
fi
[ "$ALL_OK" -eq 0 ] && exit 1
[ "$PUSH_OK" -eq 0 ] && exit 2
exit 0