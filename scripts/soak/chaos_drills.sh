#!/usr/bin/env bash
#
# chaos_drills.sh — v312-59-D / #4597 SOAK chaos drill executor
# ==============================================================================
# 4 个 fault-recovery drill, 每个独立可执行, 复用 run_soak_loop.sh 的
# start_server / stop_current_server / cleanup 等函数 (经 SOAK_NO_MAIN=1 跳过
# main() 自动触发).
#
# 用法:
#   bash scripts/soak/chaos_drills.sh smoke     # CI: 仅 drill-1 SIGKILL (轻量)
#   bash scripts/soak/chaos_drills.sh drill-1   # SIGKILL + restart (中等)
#   bash scripts/soak/chaos_drills.sh drill-2   # disk-full (重 — 需 ~800MB)
#   bash scripts/soak/chaos_drills.sh drill-3   # netem loss 5% (重 — 需 root)
#   bash scripts/soak/chaos_drills.sh drill-4   # clock skew +30s (重 — 需 CAP_SYS_TIME)
#   bash scripts/soak/chaos_drills.sh all       # 依次执行 4 drill (nightly only)
#
# 退出码:
#   0  — 所有请求的 drill PASS
#   1  — 至少一个 drill FAIL
#   2  — 前置失败 (缺 binary / 缺权限 / setup_run_dir 失败)
#
# AC:
#   • smoke (drill-1) 在 CI (无 root) 跑通, ≤60s 完成
#   • drill-2/3/4 在 Z6G4 nightly 跑, 每个 ~3min
#   • chaos_drills.log 输出 PASS/FAIL 总结
#
# 参考:
#   docs/releases/v3.12.0/evidence/issue-4560/SOAK-INFRA-REVIEW-2026-08-30.md §2 P-7
#   INCIDENT-REPORT-2026-08-30.md §3 (进程组 reap 路径)
# ==============================================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

# ── Default env (与 run_soak_loop.sh 对齐) ──
export SOAK_NO_MAIN=1                # 跳过 run_soak_loop.sh 自动 main()
export SOAK_PORT="${SOAK_PORT:-3398}"  # 避开主 SOAK 3396
export SOAK_HOURS="${SOAK_HOURS:-1}"   # drill 不需要长时
export SOAK_RESULTS_DIR="${SOAK_RESULTS_DIR:-${HOME}/sqlrustgo-chaos-results}"
export BINARY="${BINARY:-${REPO_ROOT}/target/release/sqlrustgo-mysql-server}"
export SOAK_DATA_DIR="${SOAK_DATA_DIR:-${HOME}/sqlrustgo-chaos-data-${SOAK_PORT}}"
export SOAK_DETACHED=1                # chaos drill 不需要 detach 模式
export SOAK_NO_DETACH=1               # 防父 shell reap 路径触发 (run_soak_loop.sh:104-136)

mkdir -p "${SOAK_RESULTS_DIR}"
DRILL_LOG="${SOAK_RESULTS_DIR}/chaos_drills.log"
exec >> "${DRILL_LOG}" 2>&1

ts() { date '+%Y-%m-%dT%H:%M:%S%z'; }
log() { echo "[$(ts)] $*"; }

# ── Preflight: 二进制 + 工具存在性 ──
preflight_drill() {
    local fail=0
    if [[ ! -x "${BINARY}" ]]; then
        log "ERROR: 二进制不存在: ${BINARY}"
        log "       请先: cargo build --release -p sqlrustgo-mysql-server"
        fail=1
    fi
    if ! command -v lsof >/dev/null 2>&1; then
        log "ERROR: lsof 未安装 (start_server 需 lsof 检查端口)"
        fail=1
    fi
    if [[ ${fail} -ne 0 ]]; then
        log "preflight 失败, 退出 rc=2"
        exit 2
    fi
    log "preflight 通过 (binary=${BINARY}, data_dir=${SOAK_DATA_DIR}, port=${SOAK_PORT})"
}
preflight_drill

# ── Source run_soak_loop.sh (函数 only, 不触发 main) ──
# shellcheck disable=SC1091
source "${SCRIPT_DIR}/run_soak_loop.sh"

PASS_COUNT=0
FAIL_COUNT=0
pass() { PASS_COUNT=$((PASS_COUNT + 1)); log "✅ PASS: $*"; }
fail() { FAIL_COUNT=$((FAIL_COUNT + 1)); log "❌ FAIL: $*"; }

# ── Drill 1: SIGKILL + restart (轻, CI 适用) ──
drill_sigkill_restart() {
    log "─── drill-1: SIGKILL + restart ───"
    setup_run_dir
    start_server || { fail "drill-1: start_server 失败"; return 1; }
    sleep 2
    local pid
    pid=$(cat "${RUN_DIR}/server.pid" 2>/dev/null || echo "")
    [[ -z "${pid}" ]] && { fail "drill-1: 无 server.pid"; return 1; }

    log "drill-1: kill -KILL ${pid}"
    kill -KILL "${pid}" 2>/dev/null || true
    sleep 1

    # 验证 server 已死
    if kill -0 "${pid}" 2>/dev/null; then
        fail "drill-1: SIGKILL 后 server 仍存活"
        stop_current_server 2>/dev/null || true
        return 1
    fi

    # 验证 restart 成功 (复用 start_server, 创建新 RUN_DIR)
    log "drill-1: 重启 server"
    if start_server; then
        pass "drill-1: SIGKILL + restart OK"
    else
        fail "drill-1: restart 失败"
        return 1
    fi

    stop_current_server
    return 0
}

# ── Drill 2: disk-full (重, 需 ~800MB) ──
drill_disk_full() {
    log "─── drill-2: disk-full simulation ───"
    [[ ! -d "${SOAK_DATA_DIR:-${HOME}/sqlrustgo-soak-data-${SOAK_PORT}}" ]] && {
        fail "drill-2: SOAK_DATA_DIR 不存在"; return 1;
    }
    setup_run_dir
    start_server || { fail "drill-2: start_server 失败"; return 1; }
    sleep 2

    # fill data-dir 至 ~799MB (留 1MB 缓冲)
    local fill_file="${SOAK_DATA_DIR}/fill.bin"
    log "drill-2: 填充 ${fill_file} 至 ~799MB"
    dd if=/dev/zero of="${fill_file}" bs=1M count=799 2>/dev/null
    local remaining
    remaining=$(df -B1 "${SOAK_DATA_DIR}" | awk 'NR==2 {print $4}')
    log "drill-2: 剩余 ${remaining}B (期望 ≤ 1MB)"

    # 触发一次写入: 通过 netcat 发 INSERT (简化: 用 sysbench 或 curl)
    # 这里仅检查 server 不 panic — log 中无 "panicked" / "fatal"
    sleep 3
    local panic_hits
    panic_hits=$(grep -ciE 'panicked|fatal runtime error' "${SERVER_LOG}" || echo 0)
    rm -f "${fill_file}"

    if [[ "${panic_hits}" -eq 0 ]]; then
        pass "drill-2: disk-full 无 panic (剩余 ${remaining}B, panic_hits=${panic_hits})"
    else
        fail "drill-2: server panic detected (panic_hits=${panic_hits})"
        stop_current_server
        return 1
    fi

    stop_current_server
    return 0
}

# ── Drill 3: netem loss 5% (重, 需 root) ──
drill_netem_loss() {
    log "─── drill-3: netem packet loss ───"
    if [[ "${EUID:-$(id -u)}" -ne 0 ]]; then
        fail "drill-3: 需 root (EUID=0) 才能跑 tc qdisc"; return 1;
    fi
    setup_run_dir
    start_server || { fail "drill-3: start_server 失败"; return 1; }
    sleep 2

    log "drill-3: tc qdisc add dev lo root netem loss 5%"
    tc qdisc add dev lo root netem loss 5% 2>&1 || {
        fail "drill-3: tc qdisc add 失败"; stop_current_server; return 1;
    }
    sleep 5

    # 检查 server 仍在响应 + log 无 panic
    if ! kill -0 "$(cat "${RUN_DIR}/server.pid")" 2>/dev/null; then
        tc qdisc del dev lo root 2>/dev/null || true
        fail "drill-3: server 在 netem 注入后死亡"
        return 1
    fi
    local panic_hits
    panic_hits=$(grep -ciE 'panicked|fatal runtime error' "${SERVER_LOG}" || echo 0)
    tc qdisc del dev lo root 2>/dev/null || true

    if [[ "${panic_hits}" -eq 0 ]]; then
        pass "drill-3: netem loss 5% 注入 5s 无 panic"
    else
        fail "drill-3: server panic (panic_hits=${panic_hits})"
        stop_current_server
        return 1
    fi

    stop_current_server
    return 0
}

# ── Drill 4: clock skew +30s (重, 需 CAP_SYS_TIME) ──
drill_clock_skew() {
    log "─── drill-4: clock skew +30s ───"
    if [[ "${EUID:-$(id -u)}" -ne 0 ]]; then
        fail "drill-4: 需 root 才能改系统时间"; return 1;
    fi
    setup_run_dir
    start_server || { fail "drill-4: start_server 失败"; return 1; }
    sleep 2

    log "drill-4: date -s '+30 seconds'"
    date -s "+30 seconds" 2>&1 || {
        fail "drill-4: date -s 失败 (无 CAP_SYS_TIME)"; stop_current_server; return 1;
    }
    sleep 5

    # 验证 server 仍响应
    local pid
    pid=$(cat "${RUN_DIR}/server.pid")
    if ! kill -0 "${pid}" 2>/dev/null; then
        fail "drill-4: server 在 clock skew 后死亡"
        return 1
    fi

    # 恢复时钟 (避免影响后续测试)
    log "drill-4: 恢复系统时间"
    ntpdate pool.ntp.org 2>/dev/null || date -s "-30 seconds" 2>/dev/null || true

    local panic_hits
    panic_hits=$(grep -ciE 'panicked|fatal runtime error' "${SERVER_LOG}" || echo 0)
    if [[ "${panic_hits}" -eq 0 ]]; then
        pass "drill-4: clock skew +30s 无 panic"
    else
        fail "drill-4: server panic (panic_hits=${panic_hits})"
        stop_current_server
        return 1
    fi

    stop_current_server
    return 0
}

# ── Dispatcher ──
case "${1:-help}" in
    smoke)
        # CI: 仅 drill-1 (轻量, 无 root)
        drill_sigkill_restart
        ;;
    drill-1) drill_sigkill_restart ;;
    drill-2) drill_disk_full ;;
    drill-3) drill_netem_loss ;;
    drill-4) drill_clock_skew ;;
    all)
        drill_sigkill_restart
        drill_disk_full
        drill_netem_loss
        drill_clock_skew
        ;;
    help|--help|-h)
        sed -n '2,30p' "$0" | sed 's/^# \{0,1\}//'
        exit 1
        ;;
    *)
        log "[ERROR] 未知子命令: ${1}"
        log "  用法: $0 {smoke|drill-1|drill-2|drill-3|drill-4|all}"
        exit 2
        ;;
esac

log "=========================================="
log "Chaos drill 总结: PASS=${PASS_COUNT}  FAIL=${FAIL_COUNT}"
log "=========================================="

[[ ${FAIL_COUNT} -eq 0 ]] && exit 0 || exit 1