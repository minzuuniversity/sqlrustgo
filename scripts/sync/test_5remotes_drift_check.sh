#!/usr/bin/env bash
#
# test_5remotes_drift_check.sh — 漂移检查脚本的**测量可信度**回归测试
# ==============================================================================
# 看板行：DRIFT-STALE-1
# Issue：#5778（看板 SYNC-GITCODE-1 的收口记录里另行登记）
#
# 这个测试存在的理由（全部来自实测，不是推测）：
#
#   1. `git fetch --all … | tail -3` 的退出码是 **tail** 的，不是 git 的。
#      ⇒ 某个远端 fetch 失败时，脚本**看不见**，却继续往下走。
#   2. 脚本随后用 `git rev-parse $remote/$branch`（**本地 tracking ref**）
#      算 ahead/behind。那份数据只跟"上一次成功的 fetch"一样新。
#      ⇒ fetch 失败的那个远端，报告里出现的是**过期数字**，而且是当实测数印的。
#
#   实测形态（2026-11-13）：gitcode 已被推到 252 的 tip（ls-remote 实证），
#   脚本却报 `develop/v4.1.0  gitcode  gitea252  0  357` —— 与实况相反。
#
# 本测试对每一个"量不到"的形态都断言：**该行不能是数字**，必须显式标
# UNREACHABLE / STALE-DATA，且**整次退出码非 0**（"查不到"≠"一致"）。
#
# 全程离线：5 个"远端"是本地 bare repo，fetch 故障用 stub git 注入。
#
# 用法:
#   bash scripts/sync/test_5remotes_drift_check.sh
#
# 退出码:
#   0 — 全部 PASS
#   1 — FAIL
# ==============================================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DRIFT_SCRIPT="${SCRIPT_DIR}/5remotes_drift_check.sh"

PASS_COUNT=0
FAIL_COUNT=0

pass() { PASS_COUNT=$((PASS_COUNT + 1)); echo "  ✅ PASS: $1"; }
fail() { FAIL_COUNT=$((FAIL_COUNT + 1)); echo "  ❌ FAIL: $1"; [[ -n "${2:-}" ]] && echo "       $2"; return 0; }

if [ ! -f "${DRIFT_SCRIPT}" ]; then
    echo "ERROR: 找不到被测脚本 ${DRIFT_SCRIPT}" >&2
    exit 1
fi

TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/drift_check_test.XXXXXX")"
cleanup() { [ -n "${TMP_ROOT:-}" ] && [ -d "$TMP_ROOT" ] && rm -rf "$TMP_ROOT"; }
trap cleanup EXIT INT TERM

REAL_GIT="$(command -v git)"
REMOTES_DIR="${TMP_ROOT}/remotes"
STUB_BIN="${TMP_ROOT}/bin"
CLONE_DIR="${TMP_ROOT}/clone"
SRC_DIR="${TMP_ROOT}/src"

REMOTE_NAMES=(gitcode gitea250 gitea252 gitee github)

mkdir -p "${REMOTES_DIR}" "${STUB_BIN}" "${SRC_DIR}"

# ── 构造源仓库：两个提交 OLD → NEW，外加 4 条分支 ────────────────────────
(
    cd "${SRC_DIR}" || exit 1
    "${REAL_GIT}" init -q .
    "${REAL_GIT}" checkout -q -b develop/v4.1.0
    "${REAL_GIT}" -c user.email=t@t -c user.name=t commit -q --allow-empty -m OLD
    "${REAL_GIT}" -c user.email=t@t -c user.name=t commit -q --allow-empty -m NEW
    first=$("${REAL_GIT}" rev-list --max-parents=0 HEAD | tail -1)
    "${REAL_GIT}" branch main "$first"
    "${REAL_GIT}" branch release/v4.0.0 "$first"
    "${REAL_GIT}" branch develop/v4.0.0 "$first"
) || { echo "ERROR: 构造源仓库失败" >&2; exit 1; }

OLD_SHA=$(cd "${SRC_DIR}" && "${REAL_GIT}" rev-list --max-parents=0 HEAD | tail -1)
NEW_SHA=$(cd "${SRC_DIR}" && "${REAL_GIT}" rev-parse HEAD)

# ── 构造 5 个假远端：全部对齐在 NEW ──────────────────────────────────────
reset_remotes() {
    local r
    for r in "${REMOTE_NAMES[@]}"; do
        rm -rf "${REMOTES_DIR}/${r}.git"
        "${REAL_GIT}" init --bare -q "${REMOTES_DIR}/${r}.git" 2>/dev/null || return 1
    done
    (
        cd "${SRC_DIR}" || exit 1
        for r in "${REMOTE_NAMES[@]}"; do
            "${REAL_GIT}" push -q "${REMOTES_DIR}/${r}.git" \
                "refs/heads/develop/v4.1.0:refs/heads/develop/v4.1.0" \
                "refs/heads/main:refs/heads/main" \
                "refs/heads/release/v4.0.0:refs/heads/release/v4.0.0" \
                "refs/heads/develop/v4.0.0:refs/heads/develop/v4.0.0" 2>/dev/null || exit 1
        done
    ) || return 1
    return 0
}
reset_remotes || { echo "ERROR: 构造假远端失败" >&2; exit 1; }

# ── 每个用例一个干净克隆 ─────────────────────────────────────────────────
make_clone() {
    local dir="$1"
    rm -rf "$dir"
    "${REAL_GIT}" clone -q "${REMOTES_DIR}/gitea252.git" "$dir" 2>/dev/null || return 1
    local r
    for r in "${REMOTE_NAMES[@]}"; do
        "${REAL_GIT}" -C "$dir" remote add "$r" "${REMOTES_DIR}/${r}.git" 2>/dev/null
        "${REAL_GIT}" -C "$dir" fetch -q "$r" \
            "refs/heads/develop/v4.1.0:refs/remotes/${r}/develop/v4.1.0" \
            "refs/heads/main:refs/remotes/${r}/main" \
            "refs/heads/release/v4.0.0:refs/remotes/${r}/release/v4.0.0" \
            "refs/heads/develop/v4.0.0:refs/remotes/${r}/develop/v4.0.0" 2>/dev/null || return 1
    done
    mkdir -p "$dir/scripts/sync"
    cp "${DRIFT_SCRIPT}" "$dir/scripts/sync/5remotes_drift_check.sh"
    return 0
}

# 在克隆里跑被测脚本；stdout+stderr 落到 $2
run_drift() {
    local dir="$1" out="$2"; shift 2
    (
        cd "$dir" || exit 1
        export PATH="${STUB_BIN}:${PATH}"
        export FAIL_FETCH_REMOTE="${FAIL_FETCH_REMOTE:-}"
        export GIT_TERMINAL_PROMPT=0
        bash scripts/sync/5remotes_drift_check.sh --branches develop/v4.1.0 "$@"
    ) > "$out" 2>&1
    return $?
}

# ── stub git：只让指定远端的 fetch 失败，ls-remote 照常 ──────────────────
#
# 两种 fetch 形态都要拦，否则测试只对新脚本忠实、对旧脚本失效：
#   fetch <remote> <branches>   —— 新实现（逐远端取）
#   fetch --all …               —— 旧实现（一次性取全部）
# 拦不住 `--all` 的话，旧实现照样把过期的 tracking ref 刷新掉，
# "过期数字被当实测"这一条就测不出来了。
cat > "${STUB_BIN}/git" <<STUB_EOF
#!/usr/bin/env bash
if [ "\${1:-}" = "fetch" ] && [ -n "\${FAIL_FETCH_REMOTE:-}" ]; then
    if [ "\${2:-}" = "--all" ]; then
        echo "stub: fetch --all failed (injected)" >&2
        exit 128
    fi
    for a in "\$@"; do
        if [ "\$a" = "\${FAIL_FETCH_REMOTE}" ]; then
            echo "stub: fetch to \$a failed (injected)" >&2
            exit 128
        fi
    done
fi
exec "${REAL_GIT}" "\$@"
STUB_EOF
chmod +x "${STUB_BIN}/git"

# ── 断言工具 ─────────────────────────────────────────────────────────────
# 该行必须带指定标签（而不是数字）
assert_row_label() {
    local out="$1" a="$2" b="$3" want="$4"
    awk -F'\t' -v a="$a" -v b="$b" -v want="$want" '
        $1 == "develop/v4.1.0" && (($2 == a && $3 == b) || ($2 == b && $3 == a)) {
            found = 1
            if ($4 != want || $5 != want) { bad = 1; badrow = $0 }
        }
        END {
            if (!found) { print "row not found: " a " vs " b > "/dev/stderr"; exit 1 }
            if (bad)    { print "wrong label: " badrow > "/dev/stderr"; exit 1 }
            exit 0
        }' "$out"
}

# 该行必须带数字（= 实测）
assert_row_numeric() {
    local out="$1" a="$2" b="$3"
    awk -F'\t' -v a="$a" -v b="$b" '
        $1 == "develop/v4.1.0" && (($2 == a && $3 == b) || ($2 == b && $3 == a)) {
            found = 1
            if ($4 !~ /^[0-9]+$/ || $5 !~ /^[0-9]+$/) { bad = 1; badrow = $0 }
        }
        END {
            if (!found) { print "row not found: " a " vs " b > "/dev/stderr"; exit 1 }
            if (bad)    { print "not numeric: " badrow > "/dev/stderr"; exit 1 }
            exit 0
        }' "$out"
}

echo ""
echo "[T1] 全部对齐 ⇒ exit 0，且每行都是实测数字（0 0）"
echo "----------------------------------------------------------"
reset_remotes || { echo "ERROR: 重置假远端失败" >&2; exit 1; }
if make_clone "${CLONE_DIR}"; then
    FAIL_FETCH_REMOTE=""
    run_drift "${CLONE_DIR}" "${TMP_ROOT}/t1.out"
    t1_rc=$?
    if [ "$t1_rc" -eq 0 ]; then
        pass "全部对齐 ⇒ exit 0"
    else
        fail "全部对齐时不应非 0 退出" "rc=${t1_rc}" "$(tail -6 "${TMP_ROOT}/t1.out")"
    fi
    if assert_row_numeric "${TMP_ROOT}/t1.out" gitcode gitea252; then
        pass "gitcode/gitea252 这一行是数字（实测）"
    else
        fail "对齐的行不是数字" "$(awk -F'\t' '$1=="develop/v4.1.0"' "${TMP_ROOT}/t1.out" | head -5)"
    fi
    if awk -F'\t' '$1=="develop/v4.1.0" && $4=="0" && $5=="0"' "${TMP_ROOT}/t1.out" | grep -q .; then
        pass "所有远端两两都是 0/0（确实无漂移）"
    else
        fail "对齐时出现了非 0 的漂移数" "$(awk -F'\t' '$1=="develop/v4.1.0"' "${TMP_ROOT}/t1.out" | head -5)"
    fi
else
    fail "构造克隆失败"
fi

echo ""
echo "[T2] 某远端不可达 ⇒ 该行**不得是数字**，须标 UNREACHABLE，且 rc≠0"
echo "----------------------------------------------------------"
reset_remotes || { echo "ERROR: 重置假远端失败" >&2; exit 1; }
if make_clone "${CLONE_DIR}"; then
    # gitee 指向一个不存在的地址 ⇒ 服务器问不到
    "${REAL_GIT}" -C "${CLONE_DIR}" remote set-url gitee "${REMOTES_DIR}/does-not-exist.git"
    FAIL_FETCH_REMOTE=""
    run_drift "${CLONE_DIR}" "${TMP_ROOT}/t2.out"
    t2_rc=$?
    if assert_row_label "${TMP_ROOT}/t2.out" gitcode gitee UNREACHABLE; then
        pass "不可达远端的行标成 UNREACHABLE（不是数字）"
    else
        fail "不可达远端的行没有标 UNREACHABLE" \
             "$(awk -F'\t' '$1=="develop/v4.1.0" && (($2=="gitcode"&&$3=="gitee")||($2=="gitee"&&$3=="gitcode"))' "${TMP_ROOT}/t2.out" | head -3)"
    fi
    if [ "$t2_rc" -ne 0 ]; then
        pass "存在不可达远端 ⇒ exit ${t2_rc}（非 0，不判成「一致」）"
    else
        fail "有远端不可达时仍然 exit 0" "—— 「查不到」被当成了「一致」"
    fi
    if assert_row_numeric "${TMP_ROOT}/t2.out" gitcode gitea252; then
        pass "其余（可测量的）远端之间仍然给出实测数字"
    else
        fail "可测量的远端之间不该被牵连"
    fi
else
    fail "构造克隆失败"
fi

echo ""
echo "[T3] 服务器答了但 fetch 失败 ⇒ 该行标 STALE-DATA，**过期数字不得当实测**"
echo "----------------------------------------------------------"
reset_remotes || { echo "ERROR: 重置假远端失败" >&2; exit 1; }
if make_clone "${CLONE_DIR}"; then
    # 本地 tracking ref 停在 OLD（过期），而服务器上 gitee 已经是 NEW。
    # 再让 gitee 的 fetch 失败 ⇒ 脚本既拿不到对象、tracking ref 也不会被更新。
    # 旧实现会把 OLD 当成 gitee 的现状，算出一个"漂移数"印出来。
    "${REAL_GIT}" -C "${CLONE_DIR}" update-ref refs/remotes/gitee/develop/v4.1.0 "${OLD_SHA}"
    FAIL_FETCH_REMOTE="gitee"
    run_drift "${CLONE_DIR}" "${TMP_ROOT}/t3.out"
    t3_rc=$?
    if assert_row_label "${TMP_ROOT}/t3.out" gitcode gitee STALE-DATA; then
        pass "fetch 失败的远端标成 STALE-DATA（不是数字）"
    else
        fail "fetch 失败的远端没有标 STALE-DATA" \
             "$(awk -F'\t' '$1=="develop/v4.1.0" && (($2=="gitcode"&&$3=="gitee")||($2=="gitee"&&$3=="gitcode"))' "${TMP_ROOT}/t3.out" | head -3)"
    fi
    if [ "$t3_rc" -ne 0 ]; then
        pass "有远端量不到 ⇒ exit ${t3_rc}（非 0）"
    else
        fail "有远端量不到时仍然 exit 0" "—— 过期 tracking ref 被当成了实测"
    fi
    if awk -F'\t' '$1=="develop/v4.1.0" && ($2=="gitee"||$3=="gitee") && $4 ~ /^[0-9]+$/ && $5 ~ /^[0-9]+$/' "${TMP_ROOT}/t3.out" | grep -q .; then
        fail "gitee 那一行仍是数字" "$(awk -F'\t' '$1=="develop/v4.1.0" && ($2=="gitee"||$3=="gitee")' "${TMP_ROOT}/t3.out" | head -3)"
    else
        pass "gitee 相关的行没有任何数字"
    fi
else
    fail "构造克隆失败"
fi

echo ""
echo "[T4] DRIFT-STALE-1 现场形态：远端已对齐但本地 ref 过期 ⇒ 不得报出反向漂移数"
echo "----------------------------------------------------------"
reset_remotes || { echo "ERROR: 重置假远端失败" >&2; exit 1; }
if make_clone "${CLONE_DIR}"; then
    # gitcode 的服务器上**已经**与 gitea252 一致（都是 NEW），
    # 但本地 tracking ref 还停在 OLD，且 gitcode 的 fetch 失败。
    # 旧实现 ⇒ 报出 "gitcode 落后 N 笔"（与实况相反）；新实现 ⇒ 不给数字。
    "${REAL_GIT}" -C "${CLONE_DIR}" update-ref refs/remotes/gitcode/develop/v4.1.0 "${OLD_SHA}"
    FAIL_FETCH_REMOTE="gitcode"
    run_drift "${CLONE_DIR}" "${TMP_ROOT}/t4.out"
    t4_rc=$?
    if awk -F'\t' '$1=="develop/v4.1.0" && (($2=="gitcode"&&$3=="gitea252")||($2=="gitea252"&&$3=="gitcode")) && $4 ~ /^[0-9]+$/ && $5 ~ /^[0-9]+$/' "${TMP_ROOT}/t4.out" | grep -q .; then
        fail "仍然把过期 ref 当成 gitcode 的现状报了数字" \
             "$(awk -F'\t' '$1=="develop/v4.1.0" && (($2=="gitcode"&&$3=="gitea252")||($2=="gitea252"&&$3=="gitcode"))' "${TMP_ROOT}/t4.out" | head -3)"
    else
        pass "gitcode/gitea252 行没有数字（量不到就不给数）"
    fi
    if [ "$t4_rc" -ne 0 ]; then
        pass "量不到 ⇒ exit ${t4_rc}（非 0，不报「一致」）"
    else
        fail "量不到却 exit 0" "—— 「查不到」被当成了「一致」"
    fi
    if grep -q "UNREACHABLE\|STALE-DATA" "${TMP_ROOT}/t4.out"; then
        pass "输出显式点名了不可测量的形态"
    else
        fail "输出没有点名不可测量的形态" "$(tail -8 "${TMP_ROOT}/t4.out")"
    fi
else
    fail "构造克隆失败"
fi

echo ""
echo "[T5] fetch 成功但本地 ref 过期 ⇒ 数字必须按**服务器 tip**算（不是过期 ref）"
echo "----------------------------------------------------------"
reset_remotes || { echo "ERROR: 重置假远端失败" >&2; exit 1; }
if make_clone "${CLONE_DIR}"; then
    # 服务器上 gitcode == NEW，本地 ref 停在 OLD；fetch 正常 ⇒ 应当量到 0/0。
    "${REAL_GIT}" -C "${CLONE_DIR}" update-ref refs/remotes/gitcode/develop/v4.1.0 "${OLD_SHA}"
    FAIL_FETCH_REMOTE=""
    run_drift "${CLONE_DIR}" "${TMP_ROOT}/t5.out"
    t5_rc=$?
    if assert_row_numeric "${TMP_ROOT}/t5.out" gitcode gitea252; then
        pass "gitcode/gitea252 行是数字"
    else
        fail "gitcode/gitea252 行不是数字" \
             "$(awk -F'\t' '$1=="develop/v4.1.0" && (($2=="gitcode"&&$3=="gitea252")||($2=="gitea252"&&$3=="gitcode"))' "${TMP_ROOT}/t5.out" | head -3)"
    fi
    if awk -F'\t' '$1=="develop/v4.1.0" && ($2=="gitcode" && $3=="gitea252" || $2=="gitea252" && $3=="gitcode") && $4=="0" && $5=="0"' "${TMP_ROOT}/t5.out" | grep -q .; then
        pass "按服务器 tip 算出 0/0（真实结论：无漂移）"
    else
        fail "按过期 ref 算出了漂移数" \
             "$(awk -F'\t' '$1=="develop/v4.1.0" && (($2=="gitcode"&&$3=="gitea252")||($2=="gitea252"&&$3=="gitcode"))' "${TMP_ROOT}/t5.out" | head -3)"
    fi
    if [ "$t5_rc" -eq 0 ]; then
        pass "全部对齐 ⇒ exit 0"
    else
        fail "全部对齐时不应非 0 退出" "rc=${t5_rc}" "$(tail -6 "${TMP_ROOT}/t5.out")"
    fi
else
    fail "构造克隆失败"
fi

echo ""
echo "=========================================="
echo "Total: PASS=${PASS_COUNT}  FAIL=${FAIL_COUNT}"
echo "=========================================="

if [ "$FAIL_COUNT" -gt 0 ]; then
    echo "❌ 测量可信度不达标 — 阻止合并"
    exit 1
fi
echo "✅ 量不到的远端一律不给数字，也没有静默 exit 0"
exit 0
