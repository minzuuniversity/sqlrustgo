#!/usr/bin/env bash
#
# Coverage Recovery Script - v3.4.0 Trust Convergence
#
# 改进：控制并行任务数、内存消耗、CPU 负载
# 支持分 crate 独立运行、增量 coverage

set -euo pipefail

# ============================================================================
# 默认配置
# ============================================================================
MAX_JOBS=4              # 最大并行任务数
MAX_MEMORY_GB=2        # 每个 cargo 任务最大内存（GB）
MAX_LOAD=80            # CPU 负载阈值（%）
TARGET_COVERAGE=70      # 目标覆盖率（%）
OUTPUT_DIR="target/coverage"
CRATES=()              # 空 = 所有 crate

# ============================================================================
# 工具函数
# ============================================================================

log_info()  { echo "[INFO]  $*"; }
log_warn()  { echo "[WARN]  $*" >&2; }
log_error() { echo "[ERROR] $*" >&2; }
log_pass()  { echo "[PASS]  $*"; }
log_fail()  { echo "[FAIL]  $*" >&2; }

# 获取系统 CPU 核心数
get_cpu_cores() {
    nproc 2>/dev/null || grep -c ^processor /proc/cpuinfo 2>/dev/null || 4
}

# 获取当前 CPU 负载
get_load() {
    uptime | awk -F'load average:' '{print $2}' | awk '{print $1}' | sed 's/,//'
}

# 获取可用内存（MB）
get_free_memory_mb() {
    free -m 2>/dev/null | awk '/Mem:/ {print $7}' || echo 4096
}

# 等待 CPU 负载降到阈值以下
wait_for_load() {
    local max_load=$1
    local waited=0
    while true; do
        local load=$(get_load)
        local int_load=${load%.*}

        # 去掉小数点后取整比较
        if [ "$int_load" -lt "$max_load" ]; then
            break
        fi

        if [ $waited -gt 300 ]; then
            log_warn "等待超过 5 分钟，降低并发"
            return 1
        fi

        log_info "CPU 负载 ${load}，等待降低..."
        sleep 10
        waited=$((waited + 10))
    done
    return 0
}

# 检查并等待资源
check_resources() {
    local cores=$(get_cpu_cores)
    local free_mem=$(get_free_memory_mb)
    local required_mem=$((MAX_MEMORY_GB * 1024))

    log_info "系统资源: ${cores} 核心, ${free_mem}MB 可用内存"

    # 如果内存不足，减少并发
    if [ "$free_mem" -lt "$required_mem" ]; then
        log_warn "可用内存 ${free_mem}MB < 要求 ${required_mem}MB，减少并发"
        MAX_JOBS=1
    fi

    # 如果核心数少于期望并发，调整
    if [ "$cores" -lt "$MAX_JOBS" ]; then
        log_info "CPU 核心 ${cores} < 期望并发 ${MAX_JOBS}，调整为 ${cores}"
        MAX_JOBS=$cores
    fi
}

# 等待资源可用
wait_for_resources() {
    local max_load=$1
    local max_mem_gb=$2
    local waited=0

    while true; do
        local load=$(get_load)
        local free_mem=$(get_free_memory_mb)
        local required_mem=$((max_mem_gb * 1024))

        local load_ok=true
        local mem_ok=true

        local int_load=${load%.*}

        if [ "$int_load" -ge "$max_load" ]; then
            load_ok=false
        fi

        if [ "$free_mem" -lt "$required_mem" ]; then
            mem_ok=false
        fi

        if $load_ok && $mem_ok; then
            return 0
        fi

        if [ $waited -gt 600 ]; then
            log_error "等待资源超时（10分钟）"
            return 1
        fi

        log_info "等待资源: CPU负载=${load} (目标<${max_load}), 内存=${free_mem}MB (需要>${required_mem}MB)"
        sleep 15
        waited=$((waited + 15))
    done
}

# ============================================================================
# Coverage 分析
# ============================================================================

# 运行单个 crate 的 coverage
run_crate_coverage() {
    local crate=$1
    local output=$2

    log_info "分析 crate: ${crate}"

    # 检查资源
    wait_for_resources $MAX_LOAD $MAX_MEMORY_GB || {
        log_warn "资源等待超时，继续执行（可能低性能）"
    }

    # 运行 llvm-cov
    if cargo llvm-cov -p "$crate" --lcov > "${output}/${crate}.lcov" 2>&1; then
        log_pass "crate ${crate}: coverage 完成"
        return 0
    else
        log_fail "crate ${crate}: coverage 失败"
        return 1
    fi
}

# 运行所有 crate 的 coverage
run_all_coverage() {
    mkdir -p "$OUTPUT_DIR"

    # 查找所有有测试的 crate
    local crates=$(cargo metadata --format-version 1 --no-deps 2>/dev/null | \
        python3 -c "
import json, sys
data = json.load(sys.stdin)
for pkg in data.get('packages', []):
    if pkg.get('manifest_path', '').startswith('$PWD/crates/'):
        print(pkg.get('name'))
" 2>/dev/null || echo "")

    if [ -z "$crates" ]; then
        log_warn "未找到 crate，回退到 workspace coverage"
        run_workspace_coverage
        return
    fi

    # 串行运行每个 crate（避免资源竞争）
    local failed=0
    for crate in $crates; do
        if ! run_crate_coverage "$crate" "$OUTPUT_DIR"; then
            failed=$((failed + 1))
        fi
    done

    if [ $failed -gt 0 ]; then
        log_warn "${failed} 个 crate coverage 失败"
    else
        log_pass "所有 crate coverage 完成"
    fi
}

# Workspace coverage（低资源模式）
run_workspace_coverage() {
    mkdir -p "$OUTPUT_DIR"

    log_info "运行 workspace coverage（并发数=${MAX_JOBS}）"

    wait_for_resources $MAX_LOAD $MAX_MEMORY_GB || true

    # 使用 --jobs 控制并发
    if cargo llvm-cov --workspace --lcov \
        --jobs "$MAX_JOBS" \
        --html \
        --output-dir "$OUTPUT_DIR/html" \
        > "$OUTPUT_DIR/workspace.lcov" 2>&1; then
        log_pass "workspace coverage 完成"
    else
        log_fail "workspace coverage 失败"
    fi
}

# ============================================================================
# 报告生成
# ============================================================================

generate_report() {
    local target=${1:-$TARGET_COVERAGE}

    log_info "生成 coverage 报告..."

    # 使用 cargo llvm-cov report 生成汇总
    cargo llvm-cov report --json > "$OUTPUT_DIR/coverage.json" 2>/dev/null || {
        log_warn "无法生成 JSON 报告"
    }

    # 解析并显示关键指标
    if [ -f "$OUTPUT_DIR/coverage.json" ]; then
        python3 << 'EOF'
import json

with open("target/coverage/coverage.json") as f:
    data = json.load(f)

for entry in data.get("data", []):
    for file in entry.get("files", []):
        name = file.get("filename", "")
        if any(x in name for x in ["executor", "wal", "audit", "gmp", "transaction"]):
            cov = file.get("summary", {}).get("percent_covered", 0)
            print(f"{cov:5.1f}% {name}")
EOF
    fi

    # 检查是否达标
    local total=$(cargo llvm-cov report 2>/dev/null | grep "^TOTAL" | awk '{print $4}' | sed 's/%//')
    if [ -n "$total" ]; then
        if [ "${total%.*}" -ge "$target" ]; then
            log_pass "整体覆盖率 ${total}% >= 目标 ${target}%"
            return 0
        else
            log_fail "整体覆盖率 ${total}% < 目标 ${target}%"
            return 1
        fi
    fi
}

# ============================================================================
# 主流程
# ============================================================================

show_help() {
    cat << 'EOF'
Usage: bash scripts/run_coverage.sh [OPTIONS]

Coverage Recovery 脚本 - 控制资源使用的 coverage 测试

OPTIONS:
    -j, --jobs N        最大并行任务数（默认 4）
    -m, --memory N      每个任务最大内存 GB（默认 2）
    -l, --load N        CPU 负载阈值 %（默认 80）
    -t, --target N      目标覆盖率 %（默认 70）
    -c, --crate NAME    只测试指定 crate（可重复）
    -w, --workspace     只运行 workspace coverage（不使用 llvm-cov）
    -h, --help          显示帮助

EXAMPLES:
    # 标准运行（4并发）
    bash scripts/run_coverage.sh

    # 低资源模式（2并发，内存限制 1GB）
    bash scripts/run_coverage.sh -j 2 -m 1

    # 高性能模式（8并发）
    bash scripts/run_coverage.sh -j 8 -t 75

    # 只测试 executor crate
    bash scripts/run_coverage.sh -c sqlrustgo-executor

    # 验证覆盖率是否达标
    bash scripts/run_coverage.sh --verify

ENVIRONMENT:
    CARGO_BUILD_JOBS   覆盖 -j 参数
    CARGO_PROFILE_DEV_OVERFLOW_CHECKS=false  禁用 overflow checks（提升性能）
EOF
}

main() {
    local verify_only=false
    local workspace_mode=false

    # 解析参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            -j|--jobs)       MAX_JOBS="$2"; shift 2 ;;
            -m|--memory)     MAX_MEMORY_GB="$2"; shift 2 ;;
            -l|--load)       MAX_LOAD="$2"; shift 2 ;;
            -t|--target)     TARGET_COVERAGE="$2"; shift 2 ;;
            -c|--crate)      CRATES+=("$2"); shift 2 ;;
            -w|--workspace)  workspace_mode=true; shift ;;
            -h|--help)       show_help; exit 0 ;;
            --verify)        verify_only=true; shift ;;
            *)               log_error "未知参数: $1"; show_help; exit 1 ;;
        esac
    done

    log_info "Coverage Recovery v3.4.0"
    log_info "配置: 并发=${MAX_JOBS}, 内存=${MAX_MEMORY_GB}GB, 负载<${MAX_LOAD}%, 目标=${TARGET_COVERAGE}%"

    if $verify_only; then
        generate_report
        exit $?
    fi

    # 检查资源
    check_resources

    # 运行 coverage
    if [ ${#CRATES[@]} -gt 0 ]; then
        mkdir -p "$OUTPUT_DIR"
        for crate in "${CRATES[@]}"; do
            run_crate_coverage "$crate" "$OUTPUT_DIR"
        done
    elif $workspace_mode; then
        run_workspace_coverage
    else
        # 默认：先 workspace，如果失败再逐个 crate
        run_workspace_coverage || run_all_coverage
    fi

    # 生成报告
    generate_report
}

main "$@"
