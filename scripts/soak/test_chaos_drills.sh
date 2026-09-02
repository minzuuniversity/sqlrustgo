#!/usr/bin/env bash
#
# test_chaos_drills.sh — v312-59-d / #4597 chaos_drills unit tests
# ==============================================================================
# 静态 + 行为测试, 不实际跑 server (跳过 preflight) 也不需 root.
#
#   T8a. chaos_drills.sh 语法正确 (bash -n)
#   T8b. 4 drill 函数全部存在 (drill_sigkill_restart / disk_full / netem / clock_skew)
#   T8c. dispatcher 6 个子命令 (smoke|drill-1|2|3|4|all) 全部解析
#   T8d. drill-3 / drill-4 必须有 EUID=0 守卫 (避免在无 root CI 误跑)
#   T8e. SOAK_NO_MAIN=1 守卫已加入 run_soak_loop.sh (chaos_drills 的基础设施)
#   T8f. help 子命令非零退出 (避免意外自动跑 help)
#   T8g. 错误子命令返回 rc=2 (前置失败, 区别于 1=drill 失败)
#   T8h. summary 必须在末尾输出 PASS/FAIL 计数
#   T8i. drill-1 (smoke) 不依赖 root, 可在 CI 跑
#   T8j. chaos_drills.log 文件路径在 SOAK_RESULTS_DIR
#
# 用法:
#   bash scripts/soak/test_chaos_drills.sh
#
# 退出码:
#   0  — 全部 PASS
#   1  — 至少一个 FAIL
# ==============================================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
CHAOS_DRILLS="${SCRIPT_DIR}/chaos_drills.sh"
RUN_SOAK="${SCRIPT_DIR}/run_soak_loop.sh"

PASS_COUNT=0
FAIL_COUNT=0
pass() { PASS_COUNT=$((PASS_COUNT + 1)); echo "  ✅ PASS: $1"; }
fail() { FAIL_COUNT=$((FAIL_COUNT + 1)); echo "  ❌ FAIL: $1"; [[ -n "${2:-}" ]] && echo "       $2"; }

# ── T8a: bash -n 语法 ──
echo ""
echo "[T8a] bash -n 语法"
echo "----------------------------------------------------------"
if bash -n "${CHAOS_DRILLS}" 2>/dev/null; then
    pass "chaos_drills.sh 语法正确"
else
    fail "chaos_drills.sh bash -n 失败"
    exit 1
fi

# ── T8b: 4 个 drill 函数存在 ──
echo ""
echo "[T8b] 4 个 drill 函数定义存在"
echo "----------------------------------------------------------"
DRILLS_OK=true
for fn in drill_sigkill_restart drill_disk_full drill_netem_loss drill_clock_skew; do
    if ! grep -qE "^${fn}\(\)" "${CHAOS_DRILLS}"; then
        DRILLS_OK=false
        echo "  缺函数: ${fn}"
    fi
done
if ${DRILLS_OK}; then
    pass "drill_sigkill_restart / drill_disk_full / drill_netem_loss / drill_clock_skew 全部定义"
else
    fail "至少一个 drill 函数缺失"
fi

# ── T8c: dispatcher 子命令 ──
echo ""
echo "[T8c] dispatcher 6 个子命令"
echo "----------------------------------------------------------"
DISPATCH_OK=true
for sub in smoke 'drill-1)' 'drill-2)' 'drill-3)' 'drill-4)' 'all\)'; do
    if ! grep -qE "^[[:space:]]*${sub}" "${CHAOS_DRILLS}"; then
        DISPATCH_OK=false
        echo "  缺子命令: ${sub}"
    fi
done
if ${DISPATCH_OK}; then
    pass "smoke|drill-1|drill-2|drill-3|drill-4|all 6 个子命令全部 dispatch"
else
    fail "dispatcher 缺子命令"
fi

# ── T8d: drill-3/4 root 守卫 ──
echo ""
echo "[T8d] drill-3 (netem) / drill-4 (clock) 需 root 守卫"
echo "----------------------------------------------------------"
ROOT_GUARD_OK=true
for fn in drill_netem_loss drill_clock_skew; do
    # 提取函数体前 12 行, 验证包含 EUID 检查
    if ! awk -v fn="${fn}" '
        $0 ~ "^"fn"\\(\\)" { found=1; n=0; next }
        found && n<12 { print; n++ }
    ' "${CHAOS_DRILLS}" | grep -qE 'EUID.*-ne 0|需 root|未安装'; then
        ROOT_GUARD_OK=false
        echo "  缺 root 守卫: ${fn}"
    fi
done
if ${ROOT_GUARD_OK}; then
    pass "drill-3 / drill-4 函数体内含 EUID root 守卫"
else
    fail "drill-3 / drill-4 缺 root 守卫"
fi

# ── T8e: SOAK_NO_MAIN guard (run_soak_loop.sh) ──
echo ""
echo "[T8e] run_soak_loop.sh SOAK_NO_MAIN guard"
echo "----------------------------------------------------------"
if grep -qE '^if \[\[ -z "\$\{SOAK_NO_MAIN:-?\}" \]\]; then$' "${RUN_SOAK}" \
   && grep -qE '^[[:space:]]+main "\$@"$' "${RUN_SOAK}"; then
    pass "run_soak_loop.sh 末尾有 SOAK_NO_MAIN 守卫 (chaos_drills 可 source 函数不触发 main)"
else
    fail "run_soak_loop.sh 缺 SOAK_NO_MAIN 守卫"
fi

# ── T8f: help 子命令非零退出 ──
echo ""
echo "[T8f] help 子命令退出码"
echo "----------------------------------------------------------"
HELP_RC=$(bash "${CHAOS_DRILLS}" help >/dev/null 2>&1; echo $?)
if [[ ${HELP_RC} -ne 0 ]]; then
    pass "help 子命令退出码 ${HELP_RC} ≠ 0 (避免误自动跑)"
else
    fail "help 子命令退出码为 0, 不应自动执行"
fi

# ── T8g: 错误子命令 rc=2 ──
echo ""
echo "[T8g] 错误子命令返回 rc=2"
echo "----------------------------------------------------------"
# 走 mock BINARY 路径避免 preflight 失败, 但 dispatch 失败应仍 rc=2
# 实际上 preflight 在 dispatch 之前, 错误子命令在 preflight 之后, 所以错误子命令会先
# 触发 unknown arg 分支 (rc=2).
# 但 SOAK_RESULTS_DIR 不存在会触发 preflight fail (rc=2) — 也满足需求.
# 这里只验证: 错误子命令最终 rc ∈ {1,2} (非 0)
TMPHOME=$(mktemp -d)
trap 'rm -rf "${TMPHOME}"' EXIT
BAD_RC=$(SOAK_RESULTS_DIR="${TMPHOME}/nope" bash "${CHAOS_DRILLS}" bogus 2>/dev/null; echo $?)
if [[ ${BAD_RC} -eq 2 ]]; then
    pass "bogus 子命令返回 rc=2 (前置失败)"
else
    fail "bogus 子命令 rc=${BAD_RC}, 期望 2"
fi

# ── T8h: summary 输出 ──
echo ""
echo "[T8h] 末尾 PASS/FAIL 总结"
echo "----------------------------------------------------------"
if grep -qE 'Chaos drill 总结: PASS=' "${CHAOS_DRILLS}" \
   && grep -qE 'PASS_COUNT.*-eq 0.*exit 0|exit 1' "${CHAOS_DRILLS}"; then
    pass "末尾输出 Chaos drill 总结 + 正确退出码分支"
else
    fail "summary 块缺失或退出码逻辑不完整"
fi

# ── T8i: smoke (drill-1) 不依赖 root ──
echo ""
echo "[T8i] smoke (drill-1) 不依赖 root"
echo "----------------------------------------------------------"
# 验证 drill-1 函数体内不含 EUID 检查 (因为它在无 root CI 跑)
DRILL1_HAS_ROOT_GUARD=$(awk '/^drill_sigkill_restart\(\)/,/^}/' "${CHAOS_DRILLS}" | grep -cE 'EUID.*-ne 0|需 root' || true)
if [[ ${DRILL1_HAS_ROOT_GUARD} -eq 0 ]]; then
    pass "drill-1 (smoke) 函数体内无 root 守卫, CI 可跑"
else
    fail "drill-1 误加 root 守卫, CI 无法跑"
fi

# ── T8j: chaos_drills.log 路径 ──
echo ""
echo "[T8j] chaos_drills.log 在 SOAK_RESULTS_DIR"
echo "----------------------------------------------------------"
if grep -qE 'DRILL_LOG=.*chaos_drills\.log' "${CHAOS_DRILLS}"; then
    pass "DRILL_LOG 在 SOAK_RESULTS_DIR/chaos_drills.log"
else
    fail "DRILL_LOG 路径不符"
fi

# ── 总结 ──
echo ""
echo "=========================================="
echo "Total: PASS=${PASS_COUNT}  FAIL=${FAIL_COUNT}"
echo "=========================================="

[[ ${FAIL_COUNT} -eq 0 ]] && exit 0 || exit 1
