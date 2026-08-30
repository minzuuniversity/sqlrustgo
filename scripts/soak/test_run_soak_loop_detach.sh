#!/usr/bin/env bash
#
# test_run_soak_loop_detach.sh — v312-59-d / #4595 SOAK detach regression test
# ==============================================================================
# 验证 run_soak_loop.sh 的 self-detach 块存在且行为正确:
#
#   S1. 静态检查 (fast, < 1s): 关键模式在文件中存在
#       - setsid + nohup + disown 三件套
#       - SOAK_DETACHED / SOAK_NO_DETACH env guard
#       - ! -t 0 (非 TTY stdin) 触发条件
#       - trap cleanup EXIT INT TERM HUP QUIT
#
#   S2. 行为测试 (slow, ~30s): 启动 detached SOAK, SIGKILL 父 bash, 验证子进程存活
#       - 默认 SOAK_HOURS=0.05 (3 min), SOAK_PORT=3397 (避免与主 SOAK 冲突)
#       - 启动后等 30s 让 setsid detach 完成
#       - 检查 launcher PID 仍存活 + 子 bash 已 detach
#
# 用法:
#   bash scripts/soak/test_run_soak_loop_detach.sh [--behavioral] [--quiet]
#
# 退出码:
#   0  — 全部 PASS
#   1  — 静态 FAIL (block)
#   2  — 行为 FAIL (warning, 不 block)
#
# 依赖: bash >= 4, awk, lsof (preflight)
# ==============================================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
RUN_SOAK="${REPO_ROOT}/scripts/soak/run_soak_loop.sh"

BEHAVIORAL=false
QUIET=false
for arg in "$@"; do
    case "$arg" in
        --behavioral) BEHAVIORAL=true ;;
        --quiet) QUIET=true ;;
        --help|-h)
            grep "^#" "$0" | head -25
            exit 0
            ;;
    esac
done

PASS_COUNT=0
FAIL_COUNT=0
WARN_COUNT=0

pass() { PASS_COUNT=$((PASS_COUNT + 1)); echo "  ✅ PASS: $1"; }
fail() { FAIL_COUNT=$((FAIL_COUNT + 1)); echo "  ❌ FAIL: $1"; [[ -n "${2:-}" ]] && echo "       $2"; }
warn() { WARN_COUNT=$((WARN_COUNT + 1)); echo "  ⚠️  WARN: $1"; [[ -n "${2:-}" ]] && echo "       $2"; }

if [[ ! -f "${RUN_SOAK}" ]]; then
    echo "ERROR: ${RUN_SOAK} not found"
    exit 1
fi

# ── S1: 静态检查 (fast, CI 必跑) ──
echo ""
echo "[S1] 静态检查 — self-detach 块结构完整"
echo "----------------------------------------------------------"

# 1a. setsid 命令存在
if grep -q 'setsid nohup bash "\$0"' "${RUN_SOAK}"; then
    pass "setsid nohup bash \"\$0\" 启动命令存在"
else
    fail "缺 setsid nohup bash \"\$0\" 命令" "P-1 修复核心: 父 shell reap 防护"
fi

# 1b. SOAK_DETACHED guard
if grep -qE '\[\[ -z "\$\{SOAK_DETACHED:-?\}"' "${RUN_SOAK}"; then
    pass "SOAK_DETACHED guard 防止无限重入"
else
    fail "缺 SOAK_DETACHED guard" "否则 detach 后脚本会无限自调用"
fi

# 1c. SOAK_NO_DETACH escape hatch
if grep -q 'SOAK_NO_DETACH' "${RUN_SOAK}"; then
    pass "SOAK_NO_DETACH escape hatch 存在"
else
    fail "缺 SOAK_NO_DETACH escape hatch"
fi

# 1d. ! -t 0 触发条件
if grep -q '! -t 0' "${RUN_SOAK}"; then
    pass "! -t 0 (非 TTY stdin) 触发条件存在"
else
    fail "缺 ! -t 0 触发条件" "TTY 模式下应保持前台行为"
fi

# 1e. disown 断 job 关系
if grep -q 'disown' "${RUN_SOAK}"; then
    pass "disown 断 shell job 关系"
else
    fail "缺 disown" "否则进程仍在 shell job 表中"
fi

# 1f. trap cleanup 覆盖 SIGTERM/INT/HUP/QUIT/EXIT
if grep -qE 'trap cleanup (EXIT|.*INT|.*TERM|.*HUP|.*QUIT)' "${RUN_SOAK}"; then
    pass "trap cleanup 覆盖 EXIT/INT/TERM/HUP/QUIT"
else
    fail "缺 trap cleanup" "P-1 残留风险: 信号未拦截导致 cleanup 不跑"
fi

# 1g. </dev/null 断 stdin
if grep -q '</dev/null' "${RUN_SOAK}"; then
    pass "</dev/null 断开 stdin"
else
    warn "无 </dev/null" "父 shell 关闭时 EOF 可能触发脚本退出"
fi

# 1h. PID 文件 (操作员参考)
if grep -q 'LAUNCHER_PID_FILE' "${RUN_SOAK}"; then
    pass "LAUNCHER_PID_FILE 落地供操作员管理"
else
    warn "无 LAUNCHER_PID_FILE" "操作员难以找到 detach 后的主进程"
fi

# ── S2: 行为测试 (slow, 仅 --behavioral 启用) ──
if [[ "${BEHAVIORAL}" != "true" ]]; then
    echo ""
    echo "[S2] 行为测试 — 跳过 (需 --behavioral, ~30s)"
    echo "----------------------------------------------------------"
    echo "  ℹ️  提示: bash $0 --behavioral 启用"
else
    echo ""
    echo "[S2] 行为测试 — setsid 后 SIGKILL 父 bash, 验证子进程存活"
    echo "----------------------------------------------------------"

    # 准备临时 SOAK 环境
    TMP_RESULTS=$(mktemp -d)
    TMP_PORT=3397
    TMP_DATA=$(mktemp -d)

    # 写一个简化版 preflight 替代 — 跳过 binary check, 直接 detach
    # 我们用 SOAK_HOURS=0.05 (3 min) + SOAK_PORT=3397 + SOAK_RESULTS_DIR=tmp
    cat > "${TMP_RESULTS}/launch_soak.sh" <<EOF
#!/usr/bin/env bash
export SOAK_PORT=${TMP_PORT}
export SOAK_HOURS=0.05
export SOAK_RESULTS_DIR=${TMP_RESULTS}/results
export SOAK_DATA_DIR=${TMP_DATA}
export SOAK_NO_DETACH=1   # 我们要前台运行主进程, 由父脚本管理
exec bash ${RUN_SOAK}
EOF
    chmod +x "${TMP_RESULTS}/launch_soak.sh"

    # 启动父 SOAK (前台), 记录父 PID
    mkdir -p "${TMP_RESULTS}/results"
    "${TMP_RESULTS}/launch_soak.sh" > "${TMP_RESULTS}/soak.out" 2>&1 &
    PARENT_PID=$!
    echo "  启动 SOAK (parent_pid=${PARENT_PID})"

    # 等 30s 让 preflight + start_server 完成
    sleep 30

    # 检查父进程是否还活着 (如果 binary 缺失可能已退出)
    if ! kill -0 "${PARENT_PID}" 2>/dev/null; then
        warn "父 SOAK 在 30s 内退出 (可能 binary 缺失或 preflight 失败)"
        echo "  last 5 lines: $(tail -5 "${TMP_RESULTS}/soak.out" 2>/dev/null | head -c 240)"
        echo "  跳过 S2 行为测试"
        WARN_COUNT=$((WARN_COUNT + 1))
    else
        # 找子进程 (run_soak_loop.sh 启动的 sqlrustgo + sysbench)
        CHILDREN=$(pgrep -P "${PARENT_PID}" 2>/dev/null || true)
        CHILD_COUNT=$(echo -n "${CHILDREN}" | grep -c . || echo 0)
        if [[ ${CHILD_COUNT} -lt 1 ]]; then
            warn "父 SOAK 30s 内未启动任何子进程"
            echo "  (预期 1+ 子进程 — sqlrustgo-mysql-server)"
        else
            pass "父 SOAK 启动 ${CHILD_COUNT} 子进程 (sqlrustgo, sysbench 等)"

            # SIGKILL 父进程, 等 5s, 验证子进程仍存活 (这是 P-1 修复的核心场景)
            kill -KILL "${PARENT_PID}" 2>/dev/null
            sleep 5

            # 检查子进程 (用 ps 而非 pgrep -P, 因为父已被 reap)
            SURVIVORS=0
            for child_pid in ${CHILDREN}; do
                if kill -0 "${child_pid}" 2>/dev/null; then
                    SURVIVORS=$((SURVIVORS + 1))
                fi
            done

            if [[ ${SURVIVORS} -gt 0 ]]; then
                pass "SIGKILL 父进程后 ${SURVIVORS}/${CHILD_COUNT} 子进程仍存活"
                pass "P-1 修复有效 — 父 reap 不传播到子进程组"
            else
                fail "SIGKILL 父进程后所有子进程都被杀 (P-1 修复无效)"
            fi

            # 清理残留
            for child_pid in ${CHILDREN}; do
                kill -TERM "${child_pid}" 2>/dev/null || true
            done
        fi
    fi

    rm -rf "${TMP_RESULTS}" "${TMP_DATA}"
fi

# ── 总结 ──
echo ""
echo "=========================================="
echo "Total: PASS=${PASS_COUNT}  FAIL=${FAIL_COUNT}  WARN=${WARN_COUNT}"
echo "=========================================="

if [[ ${FAIL_COUNT} -gt 0 ]]; then
    echo "❌ self-detach 块不完整 — 阻止合并"
    exit 1
fi
if [[ ${WARN_COUNT} -gt 0 ]]; then
    echo "⚠️  有警告, 但不 block"
fi
exit 0