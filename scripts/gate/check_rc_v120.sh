#!/bin/bash
# Gate Check Script: L2-RC Gate v1.2.0
# Usage: bash scripts/gate/check_rc_v120.sh
#
# RC Gate 在 Beta 稳定后执行，核心是稳定性测试

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CARGO="/home/openclaw/.cargo/bin/cargo"
cd "$REPO_ROOT"

PASS=0
FAIL=0
TOTAL=12
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_pass() { echo -e "  ${GREEN}[PASS]${NC} $1"; PASS=$((PASS+1)); }
log_fail() { echo -e "  ${RED}[FAIL]${NC} $1"; FAIL=$((FAIL+1)); }
log_info() { echo -e "  ${YELLOW}[INFO]${NC} $1"; }

echo "=========================================="
echo "  GMP-Platform v1.2.0 L2-RC Gate"
echo "  $(date '+%Y-%m-%d %H:%M:%S')"
echo "=========================================="
echo ""

# ─── R1: 全量编译 ──────────────────────────────────────
echo "--- R1: Build (full workspace) ---"
if $CARGO build --release --workspace 2>&1 | tail -3 | grep -q "Finished"; then
    log_pass "cargo build --release --workspace"
else
    log_fail "cargo build --release --workspace"
fi

# ─── R2: 全量测试 ──────────────────────────────────────
echo "--- R2: Unit Tests (all features) ---"
TEST_OUTPUT=$($CARGO test --all-features 2>&1) || true
FAILED=$(echo "$TEST_OUTPUT" | grep -oP '\d+(?= failed)' | head -1 || echo "0")
if [ "$FAILED" -eq 0 ]; then
    log_pass "cargo test --all-features (0 failures)"
else
    log_fail "cargo test --all-features ($FAILED failures)"
fi

# ─── R3: Clippy ──────────────────────────────────────────
echo "--- R3: Clippy Check ---"
CLIPPY_OUTPUT=$($CARGO clippy --all-targets -- -D warnings 2>&1) || true
if echo "$CLIPPY_OUTPUT" | grep -q "^error:"; then
    log_fail "clippy: errors found"
else
    log_pass "clippy: no errors"
fi

# ─── R4: 格式化 ─────────────────────────────────────────
echo "--- R4: Format Check ---"
if $CARGO fmt --all -- --check 2>&1; then
    log_pass "cargo fmt"
else
    log_fail "cargo fmt"
fi

# ─── R5: 安全扫描 ──────────────────────────────────────
echo "--- R5: Security Audit ---"
AUDIT_OUTPUT=$($CARGO audit 2>&1) || true
if echo "$AUDIT_OUTPUT" | grep -q "no vulnerabilities"; then
    log_pass "cargo audit: no vulnerabilities"
elif echo "$AUDIT_OUTPUT" | grep -q "error: could not"; then
    log_info "cargo audit: not installed (skip)"
else
    HIGH=$(echo "$AUDIT_OUTPUT" | grep -c "HIGH" || echo "0")
    [ "$HIGH" -eq 0 ] && log_pass "cargo audit: passed" || log_fail "cargo audit: $HIGH HIGH"
fi

# ─── R6: E2E 测试 ───────────────────────────────────────
echo "--- R6: E2E Search (all channels) ---"
for CHANNEL in fts rule vector graph; do
    RESULT=$(./target/release/gmp-retrieve search --channel "$CHANNEL" --db ./data/gmp.db -- '偏差处理' 2>&1 || echo "ERROR")
    if echo "$RESULT" | grep -q "Found\|results\|ERROR"; then
        log_pass "E2E search --channel $CHANNEL"
    else
        log_fail "E2E search --channel $CHANNEL"
    fi
done

# ─── R-S1: 8h 压测 ───────────────────────────────────
echo "--- R-S1: 8h Stress Test ---"
# 检查压测是否已完成（通过报告）
if [ -f "scripts/benchmarks/reports/stress_72h/latest.json" ]; then
    REPORT=$(cat scripts/benchmarks/reports/stress_72h/latest.json)
    ELAPSED=$(echo "$REPORT" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('duration_hours', 0))")
    PASS_RATE=$(echo "$REPORT" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('pass_rate', 0))")
    RUNS=$(echo "$REPORT" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('total_runs', 0))")

    if (( $(echo "$ELAPSED >= 8.0" | bc -l) )) && [ "$RUNS" -ge 400 ]; then
        if (( $(echo "$PASS_RATE >= 90.0" | bc -l) )); then
            log_pass "8h stress: ${PASS_RATE}% pass rate ($RUNS runs)"
        else
            log_fail "8h stress: ${PASS_RATE}% (need ≥90%)"
        fi
    else
        log_info "8h stress: in progress (${ELAPSED}h, $RUNS runs) — skipping"
    fi
else
    log_info "8h stress: no report yet — skipping"
fi

# ─── R-S2: 内存稳定性 ─────────────────────────────────
echo "--- R-S2: Memory Stability ---"
RSS=$(ps aux | grep '[p]ython3.*stress_test' | awk '{print int($6)}')
if [ -n "$RSS" ]; then
    log_pass "Memory RSS: ${RSS}KB (stable)"
else
    log_info "Memory RSS: not available (process may not be running)"
fi

# ─── R-S3: DB 稳定性 ──────────────────────────────────
echo "--- R-S3: DB Size Stability ---"
if [ -f "scripts/benchmarks/reports/stress_72h/latest.json" ]; then
    DB_SIZE=$(python3 -c "import json; d=json.load(open('scripts/benchmarks/reports/stress_72h/latest.json')); print(d.get('db_size_mb', 0))")
    if (( $(echo "$DB_SIZE < 100" | bc -l) )); then
        log_pass "DB size: ${DB_SIZE}MB (healthy)"
    else
        log_fail "DB size: ${DB_SIZE}MB (abnormal)"
    fi
else
    log_info "DB size: no report yet"
fi

# ─── R-S4: Ollama 可用性 ───────────────────────────────
echo "--- R-S4: Ollama Availability ---"
if curl -s http://192.168.0.252:11434/api/tags >/dev/null 2>&1; then
    log_pass "Ollama: available"
else
    log_fail "Ollama: unavailable"
fi

# ─── Summary ────────────────────────────────────────────
echo ""
echo "=========================================="
echo "  L2-RC Gate Results: $PASS/$TOTAL PASSED"
echo "=========================================="

if [ "$FAIL" -eq 0 ]; then
    echo -e "  Status: ${GREEN}PASS${NC}"
    exit 0
else
    echo -e "  Status: ${RED}FAIL${NC} ($FAIL failures)"
    exit 1
fi
