#!/usr/bin/env bash
#
# test_5remotes_sync_ssh.sh — 5 远端同步脚本的 SSH 通道回归测试
# ==============================================================================
# Issue: #5380  5remotes_sync.sh 在 ssh-agent 失效时无限挂起
#
# 背景（实测，非推测）:
#   本机 SSH_AUTH_SOCK 指向一个"socket 文件仍在但 agent 已死"的路径。
#   OpenSSH 会去连这个 agent；unix socket 只要文件存在 connect() 就不会立刻
#   失败，于是 ssh 阻塞在 agent 往返上，**没有任何输出**。
#   BatchMode=yes 只禁止交互式口令提示，IdentitiesOnly=yes 只限制提供哪些身份，
#   两者都**不能**切断这条路径。唯一能切断的是 IdentityAgent=none。
#
# 本测试用一个"行为替身"stub ssh 忠实复现该阻塞:
#   若 SSH_AUTH_SOCK 非空 且 参数里没有 IdentityAgent=none  →  阻塞（和真 ssh 一样）
#   否则                                                  →  按命令语义响应
#
# 因此: 脚本一旦漏传 IdentityAgent=none，stub 就会像真 ssh 一样挂住，测试超时失败。
#       这不是对脚本文本的断言，是对**行为**的断言。
#
# 测试分层:
#   S1 静态检查 — ssh 选项完整性 / 无明文凭据 / 无全盘 find /
#   S2 行为检查 — 失效 agent 下脚本不挂起，且每个 ssh 调用都带 IdentityAgent=none
#   S3 行为检查 — 远端命令确实构造了 update-ref <branch> <sha> 与临时 ref 清理
#
# 全程离线: 5 个"远端"是本地 bare repo，ssh/curl/git-push 均为 stub，
#           绝不允许触碰真实 Gitea（见 stub git 的拦截逻辑）。
#
# 用法:
#   bash scripts/sync/test_5remotes_sync_ssh.sh [--quiet]
#
# 退出码:
#   0 — 全部 PASS
#   1 — FAIL
# ==============================================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SYNC_SCRIPT="${SCRIPT_DIR}/5remotes_sync.sh"

QUIET=false
for arg in "$@"; do
    case "$arg" in
        --quiet) QUIET=true ;;
        --help|-h)
            grep "^#" "$0" | head -30
            exit 0
            ;;
    esac
done

PASS_COUNT=0
FAIL_COUNT=0

pass() { PASS_COUNT=$((PASS_COUNT + 1)); [ "$QUIET" = true ] || echo "  ✅ PASS: $1"; }
fail() { FAIL_COUNT=$((FAIL_COUNT + 1)); echo "  ❌ FAIL: $1"; [[ -n "${2:-}" ]] && echo "       $2"; return 0; }

if [ ! -f "${SYNC_SCRIPT}" ]; then
    echo "ERROR: ${SYNC_SCRIPT} not found"
    exit 1
fi

# ── S1: 静态检查 ─────────────────────────────────────────────────────────────
echo ""
echo "[S1] 静态检查 — SSH 通道自洽 / 无明文凭据 / 无全盘 find"
echo "----------------------------------------------------------"

# 1a. IdentityAgent=none —— 切断失效 agent 阻塞的唯一选项
if grep -q 'IdentityAgent=none' "${SYNC_SCRIPT}"; then
    pass "IdentityAgent=none 存在（唯一能切断失效 agent 阻塞的选项）"
else
    fail "缺 IdentityAgent=none" \
         "ssh 会向已死的 agent socket 阻塞 —— 这正是 #5380 的根因"
fi

# 1b. IdentitiesOnly=yes —— 只用默认身份文件，不用 agent 提供的身份
if grep -q 'IdentitiesOnly=yes' "${SYNC_SCRIPT}"; then
    pass "IdentitiesOnly=yes 存在"
else
    fail "缺 IdentitiesOnly=yes" "否则会尝试 agent 里所有身份，失败次数不可控"
fi

# 1c. git 的 SSH 通道也要免疫坏 agent（GIT_SSH_COMMAND）
#     必须匹配**代码行里的实际 export**，而不是注释里提到这个变量名。
if grep -vE '^[[:space:]]*#' "${SYNC_SCRIPT}" | grep -qE 'export[[:space:]]+GIT_SSH_COMMAND='; then
    pass "GIT_SSH_COMMAND 被显式 export（git push/fetch 同样免疫坏 agent）"
else
    fail "缺 GIT_SSH_COMMAND 的 export" "git push 走 SSH 时仍会继承调用方失效的 agent"
fi

# 1d. 每一个 ssh 调用点都必须经过同一个选项来源
#     统计的是**命令位置**上的 ssh 调用（行首为 ssh，或 `if ! ssh`），
#     排除 echo 文本里的 "ssh ..." 字样。
SSH_CALLS=$(grep -cE '^[[:space:]]*(if[[:space:]]+![[:space:]]*)?ssh[[:space:]]' "${SYNC_SCRIPT}" 2>/dev/null | head -1)
SSH_OPTS_USES=$(grep -c 'SSH_OPTS\[@\]' "${SYNC_SCRIPT}" 2>/dev/null | head -1)
[ -n "${SSH_CALLS}" ] || SSH_CALLS=0
[ -n "${SSH_OPTS_USES}" ] || SSH_OPTS_USES=0
if [ "${SSH_CALLS}" -gt 0 ] && [ "${SSH_CALLS}" -eq "${SSH_OPTS_USES}" ]; then
    pass "全部 ${SSH_CALLS} 处 ssh 调用都经过统一的 SSH 选项来源"
else
    fail "ssh 调用点 (${SSH_CALLS}) 与选项来源用法 (${SSH_OPTS_USES}) 数量不匹配" \
         "漏掉一处就复现 #5380 的挂起"
fi

# 1e. 容器发现不得全盘 find（只看代码行，注释里描述旧问题的文字不算）
if grep -vE '^[[:space:]]*#' "${SYNC_SCRIPT}" | grep -qE 'find[[:space:]]+/([[:space:]]|$)'; then
    fail "容器发现使用全盘 find /" "实测 250 容器上 9 分钟未完成，吃光 900s 预算"
else
    pass "容器发现不做全盘 find"
fi

# 1f. 不得含明文凭据
#     检测的是凭据的**形状**，不是具体字面量——检测器本身也不能把密钥写进仓库。
#     [0-9a-f]{40}  : 40 位十六进制 token
#     details[0-9]{4}: 口令
#     //user:pass@  : 嵌在 URL 里的凭据
if grep -vE '^[[:space:]]*#' "${SYNC_SCRIPT}" \
   | grep -qE '([0-9a-f]{40}|details[0-9]{4}|//[^/[:space:]]*:[^/@[:space:]]{6,}@)'; then
    fail "脚本内含明文凭据" "见 #5381：仓库内 token 经实测仍然有效"
else
    pass "脚本内无明文凭据"
fi

# ── S2/S3: 行为检查 ─────────────────────────────────────────────────────────
echo ""
echo "[S2/S3] 行为检查 — 失效 agent 下不挂起 + 远端命令正确"
echo "----------------------------------------------------------"

TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/sync_ssh_test.XXXXXX")"
cleanup() { [ -n "${TMP_ROOT:-}" ] && [ -d "${TMP_ROOT}" ] && rm -rf "${TMP_ROOT}"; }
trap cleanup EXIT INT TERM

REAL_GIT="$(command -v git)"
REMOTES_DIR="${TMP_ROOT}/remotes"
STUB_BIN="${TMP_ROOT}/bin"
CLONE_DIR="${TMP_ROOT}/clone"
DEAD_SOCK="${TMP_ROOT}/dead-agent.sock"
SSH_LOG="${TMP_ROOT}/ssh.log"
FAKE_REPO="${REMOTES_DIR}/gitea250.git"

mkdir -p "${REMOTES_DIR}" "${STUB_BIN}"

# 一个"死 agent"：socket 文件存在（所以 connect 不会立刻失败）但没有 agent 在跑
: > "${DEAD_SOCK}"

# ── stub ssh ────────────────────────────────────────────────────────────────
# 行为替身：复现真 ssh 在失效 agent 上的阻塞，并执行脚本发来的远端命令。
cat > "${STUB_BIN}/ssh" <<'STUB_EOF'
#!/usr/bin/env bash
set -uo pipefail

# 记录调用（最后一个参数是远端命令，选项在前面）
{
    printf 'SSH_CALL:'
    for a in "$@"; do printf ' %s' "$a"; done
    printf '\n'
} >> "${SSH_LOG}"

# 真 ssh 的行为: SSH_AUTH_SOCK 指向死 agent 时阻塞在 agent 往返上。
# IdentityAgent=none 是唯一能完全绕开 agent 的选项。
if [ -n "${SSH_AUTH_SOCK:-}" ]; then
    has_none=0
    for a in "$@"; do
        case "$a" in *IdentityAgent=none*) has_none=1 ;; esac
    done
    if [ "${has_none}" -eq 0 ]; then
        # 和真 ssh 一样: 无输出地挂住
        sleep 600
        exit 255
    fi
fi

# 取最后一个参数（远端命令）
eval "cmd=\${$#}"

# 发现阶段: 返回 "<cid> <repo_path>"
case "$cmd" in
    *docker\ ps*)
        echo "fakecid ${FAKE_REPO}"
        exit 0
        ;;
esac

# update-ref 阶段: 剥掉 "docker exec <cid> bash -c '<inner>'"，在本地执行 inner
case "$cmd" in
    *"bash -c '"*)
        pre="${cmd#*bash -c \'}"
        inner="${pre%\'}"
        ( cd "${FAKE_REPO}" && eval "${inner}" )
        exit $?
        ;;
esac

exit 0
STUB_EOF
chmod +x "${STUB_BIN}/ssh"

# ── stub curl ───────────────────────────────────────────────────────────────
# Gitea API: 读当前 ref。
#
# 这个替身**忠实模拟真实 API 的三种回答**（#5520 §4.A.1）：
#   200 + SHA → 分支存在
#   404       → 分支**不存在**（不是「问不到」！）
#   （网络错误由调用方制造）
#
# 它还必须支持 `-o <file> -w '%{http_code}'` 这套调用方式 ——
# 被测脚本正是靠 HTTP 状态码把「不存在」和「问不到」分开的。
# 状态从**假仓库的真实状态**读出来（FAKE_REPO），不是写死的常量：
# 写死的常量会让「分支不存在」这条路径永远测不到。
cat > "${STUB_BIN}/curl" <<'STUB_EOF'
#!/usr/bin/env bash
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
# 安全网: 拦截任何指向真实 Gitea 主机 (192.168.0.25x) 的 push，
#         保证本测试绝不改动真实远端。其余 git 调用原样转发。
cat > "${STUB_BIN}/git" <<STUB_EOF
#!/usr/bin/env bash
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

# ── 构造 5 个本地 bare repo 当作 5 个远端 ──────────────────────────────────
setup_failed=0
for r in gitea252 gitea250 gitcode gitee github; do
    if ! "${REAL_GIT}" init --bare -q "${REMOTES_DIR}/${r}.git" 2>/dev/null; then
        echo "  ERROR: 无法创建 bare repo ${r}.git"
        setup_failed=1
        continue
    fi
    # 让 bare repo 的 HEAD 指向真实的分支，clone 才会检出它
    "${REAL_GIT}" --git-dir="${REMOTES_DIR}/${r}.git" \
        symbolic-ref HEAD refs/heads/develop/v4.1.0 2>/dev/null
done

if [ "${setup_failed}" -ne 0 ]; then
    echo "  ERROR: 测试环境搭建失败"
    exit 1
fi

# 造两个提交: OLD (初始) 与 NEW (同步目标)
SRC_DIR="${TMP_ROOT}/src"
"${REAL_GIT}" init -q "${SRC_DIR}"
(
    cd "${SRC_DIR}" || exit 1
    "${REAL_GIT}" -c user.email=t@t -c user.name=t commit -q --allow-empty -m OLD
    OLD_SHA=$("${REAL_GIT}" rev-parse HEAD)
    "${REAL_GIT}" -c user.email=t@t -c user.name=t commit -q --allow-empty -m NEW
    NEW_SHA=$("${REAL_GIT}" rev-parse HEAD)
    # 源远端 gitea252 已经在 NEW
    "${REAL_GIT}" push -q "${REMOTES_DIR}/gitea252.git" \
        "${NEW_SHA}:refs/heads/develop/v4.1.0" 2>/dev/null
    # 其余 4 个还在 OLD
    for r in gitea250 gitcode gitee github; do
        "${REAL_GIT}" push -q "${REMOTES_DIR}/${r}.git" \
            "${OLD_SHA}:refs/heads/develop/v4.1.0" 2>/dev/null
    done
) || { echo "  ERROR: 构造提交失败"; exit 1; }

OLD_SHA=$(cd "${SRC_DIR}" && "${REAL_GIT}" rev-list --max-parents=0 HEAD | tail -1)
NEW_SHA=$(cd "${SRC_DIR}" && "${REAL_GIT}" rev-parse HEAD)

# 克隆（远端名就用脚本期望的名字）
"${REAL_GIT}" clone -q -o gitea252 "${REMOTES_DIR}/gitea252.git" "${CLONE_DIR}" 2>/dev/null || {
    echo "  ERROR: clone 失败"; exit 1; }
for r in gitea250 gitcode gitee github; do
    "${REAL_GIT}" -C "${CLONE_DIR}" remote add "$r" "${REMOTES_DIR}/${r}.git"
done

# 把被测脚本复制进克隆目录: 脚本会 cd 到自己的 REPO_ROOT，
# 复制后 REPO_ROOT 就是这个临时克隆，测试不碰真实仓库。
mkdir -p "${CLONE_DIR}/scripts/sync"
cp "${SYNC_SCRIPT}" "${CLONE_DIR}/scripts/sync/5remotes_sync.sh"
# 凭据解析的共用实现也必须一起复制 —— 脚本 source 它，缺了它脚本会
# fail-fast(#5520)。只复制被测脚本会让测试测到的是"依赖缺失"而不是被测行为。
cp "${SCRIPT_DIR}/gitea_credential_common.sh" \
   "${CLONE_DIR}/scripts/sync/gitea_credential_common.sh"

# ── 跑被测脚本 ─────────────────────────────────────────────────────────────
# GITEA_HOSTS / GITEA_PUSH_URL / GITEA_USER / GITEA_TOKEN 是脚本提供的覆盖开关；
# 修复前的脚本不认识它们 —— 那正是 RED 的一部分。
run_with_deadline() {
    local secs=$1; shift
    "$@" &
    local pid=$! i=0
    while kill -0 "$pid" 2>/dev/null; do
        if [ "$i" -ge "$secs" ]; then
            kill -9 "$pid" 2>/dev/null
            wait "$pid" 2>/dev/null
            return 124
        fi
        sleep 1
        i=$((i + 1))
    done
    wait "$pid"
    return $?
}

: > "${SSH_LOG}"
SYNC_OUT="${TMP_ROOT}/sync.out"

# 在临时克隆里跑被测脚本（环境全部指向 stub / 本地 bare repo）
run_sync_under_test() {
    cd "${CLONE_DIR}" || return 1
    export PATH="${STUB_BIN}:${PATH}"
    export SSH_AUTH_SOCK="${DEAD_SOCK}"
    export SSH_LOG="${SSH_LOG}"
    export FAKE_REPO="${FAKE_REPO}"
    export CURL_SHA="${OLD_SHA}"
    export GITEA_HOSTS="192.168.0.250:liying"
    export GITEA_PUSH_URL="${FAKE_REPO}"
    export GITEA_USER="stub-user"
    export GITEA_TOKEN="stub-token"
    bash scripts/sync/5remotes_sync.sh develop/v4.1.0
}

# 30s 看门狗: 脚本若挂起（真 ssh 的阻塞行为）会被杀掉，退出码 124
run_with_deadline 30 run_sync_under_test > "${SYNC_OUT}" 2>&1
SYNC_RC=$?

# 2a. 脚本没有挂起
if [ "${SYNC_RC}" = "124" ]; then
    fail "脚本在失效 agent 环境下挂起（被 30s 看门狗杀掉）" \
         "ssh stub 复现了真 ssh 的阻塞：SSH_AUTH_SOCK 非空且参数缺 IdentityAgent=none"
else
    pass "脚本在失效 agent 环境下未挂起（退出码 ${SYNC_RC}）"
fi

# 2b. 每个 ssh 调用都带了 IdentityAgent=none
SSH_LINES=$(grep -c '^SSH_CALL:' "${SSH_LOG}" 2>/dev/null | head -1)
SSH_WITH_NONE=$(grep -c 'SSH_CALL:.*IdentityAgent=none' "${SSH_LOG}" 2>/dev/null | head -1)
[ -n "${SSH_LINES}" ] || SSH_LINES=0
[ -n "${SSH_WITH_NONE}" ] || SSH_WITH_NONE=0
if [ "${SSH_LINES}" -gt 0 ] && [ "${SSH_LINES}" -eq "${SSH_WITH_NONE}" ]; then
    pass "全部 ${SSH_LINES} 次 ssh 调用都带 IdentityAgent=none"
elif [ "${SSH_LINES}" -eq 0 ]; then
    fail "一次 ssh 调用都没发生" "脚本在到达 SSH 阶段前就退出了（见下方输出）"
else
    fail "只有 ${SSH_WITH_NONE}/${SSH_LINES} 次 ssh 调用带 IdentityAgent=none" \
         "漏掉的那几次在真实环境里就是挂起点"
fi

# 2c. 脚本确实走到了受保护分支的 update-ref
if grep -q 'update-ref refs/heads/develop/v4.1.0' "${SSH_LOG}" 2>/dev/null; then
    pass "远端命令构造了 update-ref refs/heads/develop/v4.1.0 <sha>"
else
    fail "远端命令里没有 update-ref refs/heads/develop/v4.1.0" \
         "见 SSH_LOG: ${SSH_LOG}"
fi

# 2d. 临时 ref 被清理
if grep -q 'update-ref -d refs/heads/' "${SSH_LOG}" 2>/dev/null; then
    pass "远端命令清理了临时 ref"
else
    fail "远端命令没有清理临时 ref" "会留下一堆 tmp-sync-* 分支"
fi

# 2e. 端到端: 5 个远端最终都指向 NEW
ALL_AT_NEW=1
for r in gitea252 gitea250 gitcode gitee github; do
    got=$("${REAL_GIT}" -C "${CLONE_DIR}" ls-remote "${REMOTES_DIR}/${r}.git" \
            refs/heads/develop/v4.1.0 2>/dev/null | cut -f1)
    if [ "${got}" != "${NEW_SHA}" ]; then
        ALL_AT_NEW=0
        fail "远端 ${r} 未同步到目标 SHA" "got=${got:-none} want=${NEW_SHA}"
    fi
done
[ "${ALL_AT_NEW}" -eq 1 ] && pass "5 个远端最终都指向目标 SHA ${NEW_SHA:0:12}"

if [ "${FAIL_COUNT}" -gt 0 ]; then
    echo ""
    echo "--- 被测脚本输出（末尾 25 行） ---"
    tail -25 "${SYNC_OUT}" 2>/dev/null | sed 's/^/  | /'
fi

# ── 总结 ───────────────────────────────────────────────────────────────────
echo ""
echo "=========================================="
echo "Total: PASS=${PASS_COUNT}  FAIL=${FAIL_COUNT}"
echo "=========================================="

if [ "${FAIL_COUNT}" -gt 0 ]; then
    echo "❌ 5remotes_sync.sh 的 SSH 通道不健壮 — 阻止合并"
    exit 1
fi
echo "✅ SSH 通道健壮"
exit 0
