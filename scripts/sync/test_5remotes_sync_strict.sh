#!/usr/bin/env bash
#
# test_5remotes_sync_strict.sh — 5 远端同步脚本的**失败传播**回归测试
# ==============================================================================
# Issue: #5520  「可能静默 exit 0」
#
# 这个测试存在的理由（全部来自实测，不是推测）：
#
#   1. `cmd | tail -N` 的退出码是 **tail** 的，不是 cmd 的。
#      脚本里有三处这样的写法（fetch / push / ssh update-ref），
#      每一处都会把「命令失败了」变成「脚本认为成功了」。
#   2. 校验阶段曾经比的是**本地 tracking ref**（`git rev-parse $r/$branch`），
#      那份数据可能过期 ⇒ 一个根本没被推进的远端也会被判成「一致」。
#   3. 凭据曾经出现在 **argv** 里（`curl -H "Authorization: …"`、
#      `git -c http.extraheader=…`）⇒ `ps aux` 能直接读到。
#
# 本测试对**每一个**失败注入都断言：**退出码非 0**，且输出里点名了原因。
# 「失败必须阻断，不能静默通过」——这是仓库的反编造要求。
#
# 全程离线: 5 个"远端"是本地 bare repo，ssh/curl 是 stub，
#           且 stub git 会拦截任何指向真实 Gitea 主机 (192.168.0.25x) 的 push。
#
# 用法:
#   bash scripts/sync/test_5remotes_sync_strict.sh
#
# 退出码:
#   0 — 全部 PASS
#   1 — FAIL
# ==============================================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SYNC_SCRIPT="${SCRIPT_DIR}/5remotes_sync.sh"
HELPER_SCRIPT="${SCRIPT_DIR}/gitea_credential_common.sh"

PASS_COUNT=0
FAIL_COUNT=0

pass() { PASS_COUNT=$((PASS_COUNT + 1)); echo "  ✅ PASS: $1"; }
fail() { FAIL_COUNT=$((FAIL_COUNT + 1)); echo "  ❌ FAIL: $1"; [[ -n "${2:-}" ]] && echo "       $2"; return 0; }

if [ ! -f "${SYNC_SCRIPT}" ] || [ ! -f "${HELPER_SCRIPT}" ]; then
    echo "ERROR: 找不到被测脚本或其共用 helper" >&2
    exit 1
fi

TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/sync_strict_test.XXXXXX")"
cleanup() { [ -n "${TMP_ROOT:-}" ] && [ -d "$TMP_ROOT" ] && rm -rf "$TMP_ROOT"; }
trap cleanup EXIT INT TERM

REAL_GIT="$(command -v git)"
REMOTES_DIR="${TMP_ROOT}/remotes"
STUB_BIN="${TMP_ROOT}/bin"
CLONE_DIR="${TMP_ROOT}/clone"
FAKE_REPO="${REMOTES_DIR}/gitea250.git"
ARGV_LOG="${TMP_ROOT}/argv.log"

mkdir -p "${REMOTES_DIR}" "${STUB_BIN}"

# ── stub ssh ───────────────────────────────────────────────────────────────
# 忠实记录 argv（用来断言「凭据不进进程列表」），
# 并按 FAIL_UPDATE_REF 决定 update-ref 是否失败。
cat > "${STUB_BIN}/ssh" <<'STUB_EOF'
#!/usr/bin/env bash
{
    printf 'SSH_ARGV:'
    for a in "$@"; do printf ' %s' "$a"; done
    printf '\n'
} >> "${ARGV_LOG}"

eval "cmd=\${$#}"

case "$cmd" in
    *docker\ ps*)
        echo "fakecid ${FAKE_REPO}"
        exit 0
        ;;
esac

case "$cmd" in
    *"bash -c '"*)
        pre="${cmd#*bash -c \'}"
        inner="${pre%\'}"
        # 失败注入：让受保护分支的 update-ref 失败
        if [ "${FAIL_UPDATE_REF:-0}" = "1" ]; then
            case "$inner" in
                *"update-ref refs/heads/"*)
                    echo "stub ssh: update-ref failed (injected)" >&2
                    exit 1
                    ;;
            esac
        fi
        ( cd "${FAKE_REPO}" && eval "${inner}" )
        exit $?
        ;;
esac

exit 0
STUB_EOF
chmod +x "${STUB_BIN}/ssh"

# ── stub curl ──────────────────────────────────────────────────────────────
# Gitea API: 读当前 ref。忠实模拟真实 API 的三种回答：
#   200 + SHA → 分支存在；404 → 分支**不存在**；其它 → 问不到。
# 状态从假仓库的真实状态读出来，不是写死的常量。
cat > "${STUB_BIN}/curl" <<'STUB_EOF'
#!/usr/bin/env bash
{
    printf 'CURL_ARGV:'
    for a in "$@"; do printf ' %s' "$a"; done
    printf '\n'
} >> "${ARGV_LOG}"

url=""
outfile=""
prev=""
for a in "$@"; do
    case "$prev" in
        -o) outfile="$a"; prev=""; continue ;;
        -w|-K) prev=""; continue ;;
    esac
    case "$a" in
        -o) prev="-o"; continue ;;
        -w) prev="-w"; continue ;;
        -K) prev="-K"; continue ;;
        http://*|https://*) url="$a" ;;
    esac
done
branch="${url##*/refs/heads/}"
sha=""
if [ -n "${FAKE_REPO:-}" ] && [ -n "$branch" ]; then
    # --verify --quiet: 分支不存在时**什么都不打印**。
    # （裸 rev-parse 会把没解析出来的参数名打到 stdout 上 —— 那不是 SHA。）
    sha=$(git --git-dir="${FAKE_REPO}" rev-parse --verify --quiet "refs/heads/$branch" 2>/dev/null)
fi
if [ -n "$sha" ]; then
    code=200
    body="[{\"object\":{\"sha\":\"$sha\"}}]"
else
    code=404
    body="[]"
fi
[ -n "$outfile" ] && printf '%s' "$body" > "$outfile"
printf '%s\n' "$code"
STUB_EOF
chmod +x "${STUB_BIN}/curl"

# ── stub git ───────────────────────────────────────────────────────────────
# 记录 argv；拦截任何指向真实 Gitea 主机的 push（安全网）；其余原样转发。
cat > "${STUB_BIN}/git" <<STUB_EOF
#!/usr/bin/env bash
{
    printf 'GIT_ARGV:'
    for a in "\$@"; do printf ' %s' "\$a"; done
    printf '\n'
} >> "\${ARGV_LOG}"

if [ "\${1:-}" = "push" ]; then
    for a in "\$@"; do
        case "\$a" in
            *192.168.0.25*)
                echo "BLOCKED BY TEST: refusing to push to a real Gitea host (\$a)" >&2
                exit 1
                ;;
        esac
    done
fi
exec "${REAL_GIT}" "\$@"
STUB_EOF
chmod +x "${STUB_BIN}/git"

# ── 构造 5 个本地 bare repo ───────────────────────────────────────────────
# 每个用例前都要重置：被测脚本会真的改这些仓库的状态。
SRC_DIR="${TMP_ROOT}/src"
"${REAL_GIT}" init -q "${SRC_DIR}"
(
    cd "${SRC_DIR}" || exit 1
    "${REAL_GIT}" -c user.email=t@t -c user.name=t commit -q --allow-empty -m OLD
    "${REAL_GIT}" -c user.email=t@t -c user.name=t commit -q --allow-empty -m NEW
) || { echo "  ERROR: 构造提交失败"; exit 1; }
OLD_SHA=$(cd "${SRC_DIR}" && "${REAL_GIT}" rev-list --max-parents=0 HEAD | tail -1)
NEW_SHA=$(cd "${SRC_DIR}" && "${REAL_GIT}" rev-parse HEAD)

# 重置 5 个假远端：gitea252 在 NEW，其余在 OLD。
reset_remotes() {
    local r
    for r in gitea252 gitea250 gitcode gitee github; do
        rm -rf "${REMOTES_DIR}/${r}.git"
        "${REAL_GIT}" init --bare -q "${REMOTES_DIR}/${r}.git" 2>/dev/null || return 1
        "${REAL_GIT}" --git-dir="${REMOTES_DIR}/${r}.git" \
            symbolic-ref HEAD refs/heads/develop/v4.1.0 2>/dev/null
    done
    (
        cd "${SRC_DIR}" || return 1
        "${REAL_GIT}" push -q "${REMOTES_DIR}/gitea252.git" \
            "${NEW_SHA}:refs/heads/develop/v4.1.0" 2>/dev/null || exit 1
        for r in gitea250 gitcode gitee github; do
            "${REAL_GIT}" push -q "${REMOTES_DIR}/${r}.git" \
                "${OLD_SHA}:refs/heads/develop/v4.1.0" 2>/dev/null || exit 1
        done
    ) || return 1
    return 0
}
reset_remotes || { echo "  ERROR: 构造假远端失败"; exit 1; }

# ── 每个用例一个干净克隆 ───────────────────────────────────────────────────
make_clone() {
    local dir="$1"
    rm -rf "$dir"
    "${REAL_GIT}" clone -q -o gitea252 "${REMOTES_DIR}/gitea252.git" "$dir" 2>/dev/null || return 1
    for r in gitea250 gitcode gitee github; do
        "${REAL_GIT}" -C "$dir" remote add "$r" "${REMOTES_DIR}/${r}.git"
    done
    mkdir -p "$dir/scripts/sync"
    cp "${SYNC_SCRIPT}" "$dir/scripts/sync/5remotes_sync.sh"
    cp "${HELPER_SCRIPT}" "$dir/scripts/sync/gitea_credential_common.sh"
    return 0
}

# 在克隆里跑被测脚本；stdout+stderr 落到 $2
run_sync() {
    local dir="$1" out="$2"; shift 2
    (
        cd "$dir" || exit 1
        export PATH="${STUB_BIN}:${PATH}"
        export ARGV_LOG="${ARGV_LOG}"
        export FAKE_REPO="${FAKE_REPO}"
        export GITEA_HOSTS="192.168.0.250:liying"
        export GITEA_PUSH_URL="${FAKE_REPO}"
        export GITEA_USER="stub-user"
        export GITEA_TOKEN="${TEST_TOKEN}"
        export GIT_TERMINAL_PROMPT=0
        bash scripts/sync/5remotes_sync.sh develop/v4.1.0
    ) > "$out" 2>&1
    return $?
}

TEST_TOKEN="STRICT-TEST-TOKEN-9d3f7a2b"

echo ""
echo "[T1] ssh update-ref 失败 ⇒ 必须非 0 退出（不得静默通过）"
echo "----------------------------------------------------------"
: > "${ARGV_LOG}"
reset_remotes || { fail "重置假远端失败"; }
if make_clone "${CLONE_DIR}"; then
    if FAIL_UPDATE_REF=1 run_sync "${CLONE_DIR}" "${TMP_ROOT}/t1.out"; then
        t1_rc=0
    else
        t1_rc=$?
    fi
    if [ "$t1_rc" -eq 0 ]; then
        fail "ssh update-ref 失败时脚本仍然 exit 0" \
             "—— 这正是 #5520 里的「静默假绿」"
    else
        pass "ssh update-ref 失败 ⇒ exit ${t1_rc}（非 0）"
    fi
    if grep -q "ERROR: ssh update-ref" "${TMP_ROOT}/t1.out"; then
        pass "输出点名了失败原因（ERROR: ssh update-ref …）"
    else
        fail "输出没有点名失败原因" "$(tail -5 "${TMP_ROOT}/t1.out")"
    fi
else
    fail "构造克隆失败"
fi

echo ""
echo "[T2] 有远端回答不了 ⇒ 必须非 0 退出（不能判成「一致」）"
echo "----------------------------------------------------------"
reset_remotes || { fail "重置假远端失败"; }
if make_clone "${CLONE_DIR}"; then
    # 让 github 指向一个不存在的位置：读不到 ⇒ 无法验证
    "${REAL_GIT}" -C "${CLONE_DIR}" remote set-url github "${REMOTES_DIR}/does-not-exist.git"
    run_sync "${CLONE_DIR}" "${TMP_ROOT}/t2.out"
    t2_rc=$?
    if [ "$t2_rc" -eq 0 ]; then
        fail "有远端读不到时脚本仍然 exit 0" \
             "—— 「查不到」被当成了「一致」"
    else
        pass "有远端读不到 ⇒ exit ${t2_rc}（非 0）"
    fi
    if grep -qE "could not read|ERROR" "${TMP_ROOT}/t2.out"; then
        pass "输出点名了读不到的远端"
    else
        fail "输出没有点名读不到的远端" "$(tail -5 "${TMP_ROOT}/t2.out")"
    fi
else
    fail "构造克隆失败"
fi

echo ""
echo "[T3] 没有可用凭据 ⇒ 必须明确报错并非 0 退出"
echo "----------------------------------------------------------"
reset_remotes || { fail "重置假远端失败"; }
if make_clone "${CLONE_DIR}"; then
    (
        cd "${CLONE_DIR}" || exit 1
        export PATH="${STUB_BIN}:${PATH}"
        export ARGV_LOG="${ARGV_LOG}"
        export FAKE_REPO="${FAKE_REPO}"
        export GITEA_HOSTS="192.168.0.250:liying"
        export GITEA_PUSH_URL="${FAKE_REPO}"
        export GIT_TERMINAL_PROMPT=0
        # 隔离本机的 credential store：HOME 指到空目录 + 全局配置置空
        export HOME="${TMP_ROOT}/empty-home"
        export GIT_CONFIG_GLOBAL=/dev/null
        export GIT_CONFIG_SYSTEM=/dev/null
        mkdir -p "${HOME}"
        unset GITEA_USER
        unset GITEA_TOKEN
        bash scripts/sync/5remotes_sync.sh develop/v4.1.0
    ) > "${TMP_ROOT}/t3.out" 2>&1
    t3_rc=$?
    if [ "$t3_rc" -eq 0 ]; then
        fail "没有凭据时脚本仍然 exit 0" "—— 「查不到」被当成了「落后」"
    else
        pass "没有凭据 ⇒ exit ${t3_rc}（非 0）"
    fi
    if grep -q "no Gitea credential available" "${TMP_ROOT}/t3.out"; then
        pass "输出明确说了「没有可用凭据」"
    else
        fail "输出没有明确说凭据不可用" "$(tail -5 "${TMP_ROOT}/t3.out")"
    fi
else
    fail "构造克隆失败"
fi

echo ""
echo "[T4] 凭据不得出现在进程列表（argv）里"
echo "----------------------------------------------------------"
: > "${ARGV_LOG}"
reset_remotes || { fail "重置假远端失败"; }
if make_clone "${CLONE_DIR}"; then
    run_sync "${CLONE_DIR}" "${TMP_ROOT}/t4.out" >/dev/null 2>&1
    # 凭据可能以两种形态出现在 argv 里：原文（token / user:pass），
    # 或者 Basic 认证的 base64 形态。两种都要查。
    auth_b64=$(printf '%s:%s' "stub-user" "$TEST_TOKEN" | base64 | tr -d '\n')
    if grep -q "$TEST_TOKEN" "${ARGV_LOG}" 2>/dev/null; then
        fail "凭据（原文）出现在 argv 里" "$(grep -m3 "$TEST_TOKEN" "${ARGV_LOG}")"
    else
        pass "argv 里没有凭据原文"
    fi
    if grep -q "$auth_b64" "${ARGV_LOG}" 2>/dev/null; then
        fail "凭据（base64 形态）出现在 argv 里" "$(grep -m3 "$auth_b64" "${ARGV_LOG}")"
    else
        pass "argv 里没有凭据的 base64 形态"
    fi
    if grep -q "$TEST_TOKEN" "${CLONE_DIR}/.git/config" 2>/dev/null; then
        fail "凭据被写进了 .git/config"
    else
        pass ".git/config 里没有凭据"
    fi
else
    fail "构造克隆失败"
fi

echo ""
echo "[T5] 远端**没有这个分支** ⇒ 必须把它建出来（不是「查不到」）"
echo "----------------------------------------------------------"
reset_remotes || { fail "重置假远端失败"; }
if make_clone "${CLONE_DIR}"; then
    # 让 250 那个假仓库**没有** develop/v4.1.0 这条分支
    "${REAL_GIT}" --git-dir="${FAKE_REPO}" update-ref -d refs/heads/develop/v4.1.0 2>/dev/null
    run_sync "${CLONE_DIR}" "${TMP_ROOT}/t5.out"
    t5_rc=$?
    t5_sha=$("${REAL_GIT}" --git-dir="${FAKE_REPO}" rev-parse refs/heads/develop/v4.1.0 2>/dev/null)
    if [ "$t5_rc" -eq 0 ] && [ "$t5_sha" = "$NEW_SHA" ]; then
        pass "分支不存在 ⇒ 被建出来并推进到目标 SHA（exit 0）"
    else
        fail "分支不存在时没有把它建出来" \
             "rc=${t5_rc}, 250 现在的 SHA=${t5_sha:-<none>}, 目标=${NEW_SHA}"
    fi
    if grep -q "does not exist on" "${TMP_ROOT}/t5.out"; then
        pass "输出明确区分了「分支不存在」与「查不到」"
    else
        fail "输出没有区分「分支不存在」与「查不到」" "$(tail -8 "${TMP_ROOT}/t5.out")"
    fi
else
    fail "构造克隆失败"
fi

echo ""
echo "=========================================="
echo "Total: PASS=${PASS_COUNT}  FAIL=${FAIL_COUNT}"
echo "=========================================="

if [ "$FAIL_COUNT" -gt 0 ]; then
    echo "❌ 失败传播不完整 — 阻止合并"
    exit 1
fi
echo "✅ 失败一律阻断，没有静默 exit 0"
exit 0
