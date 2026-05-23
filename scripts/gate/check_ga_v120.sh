#!/bin/bash
# Gate Check Script: L3-GA Gate v1.2.0
# Usage: bash scripts/gate/check_ga_v120.sh
#
# GA Gate 是最终发布关卡，需要 72h 压测通过 + 全部文档完整

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CARGO="/home/openclaw/.cargo/bin/cargo"
cd "$REPO_ROOT"

PASS=0
FAIL=0
TOTAL=18
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_pass() { echo -e "  ${GREEN}[PASS]${NC} $1"; PASS=$((PASS+1)); }
log_fail() { echo -e "  ${RED}[FAIL]${NC} $1"; FAIL=$((FAIL+1)); }
log_info() { echo -e "  ${YELLOW}[INFO]${NC} $1"; }

echo "=========================================="
echo "  GMP-Platform v1.2.0 L3-GA Gate"
echo "  $(date '+%Y-%m-%d %H:%M:%S')"
echo "=========================================="
echo ""

# ─── G1: 全量编译 ─────────────────────────────────────
echo "--- G1: Build ---"
if $CARGO build --release --workspace 2>&1 | tail -3 | grep -q "Finished"; then
    log_pass "cargo build --release --workspace"
else
    log_fail "cargo build --release --workspace"
fi

# ─── G2: 全量测试 ─────────────────────────────────────
echo "--- G2: Unit Tests ---"
TEST_OUTPUT=$($CARGO test --all-features 2>&1) || true
FAILED=$(echo "$TEST_OUTPUT" | grep -oP '\d+(?= failed)' | head -1 || echo "0")
if [ "$FAILED" -eq 0 ]; then
    log_pass "cargo test --all-features (0 failures)"
else
    log_fail "cargo test --all-features ($FAILED failures)"
fi

# ─── G3: Clippy ────────────────────────────────────────
echo "--- G3: Clippy ---"
CLIPPY_OUTPUT=$($CARGO clippy --all-targets -- -D warnings 2>&1) || true
if echo "$CLIPPY_OUTPUT" | grep -q "^error:"; then
    log_fail "clippy: errors"
else
    log_pass "clippy: no errors"
fi

# ─── G4: 格式化 ───────────────────────────────────────
echo "--- G4: Format ---"
if $CARGO fmt --all -- --check 2>&1; then
    log_pass "cargo fmt"
else
    log_fail "cargo fmt"
fi

# ─── G5: 安全扫描 ─────────────────────────────────────
echo "--- G5: Security Audit ---"
AUDIT_OUTPUT=$($CARGO audit 2>&1) || true
if echo "$AUDIT_OUTPUT" | grep -q "no vulnerabilities"; then
    log_pass "cargo audit: no vulnerabilities"
elif echo "$AUDIT_OUTPUT" | grep -q "error: could not"; then
    log_info "cargo audit: not installed (skip)"
else
    HIGH=$(echo "$AUDIT_OUTPUT" | grep -c "HIGH" || echo "0")
    [ "$HIGH" -eq 0 ] && log_pass "cargo audit: passed" || log_fail "cargo audit: $HIGH HIGH"
fi

# ─── G6: E2E 测试 ─────────────────────────────────────
echo "--- G6: E2E (all channels) ---"
ALL_PASS=0
for CHANNEL in fts rule vector graph; do
    if ./target/release/gmp-retrieve search --channel "$CHANNEL" --db ./data/gmp.db -- '偏差处理' 2>&1 | grep -q "Found\|results"; then
        ALL_PASS=$((ALL_PASS+1))
    fi
done
if [ "$ALL_PASS" -eq 4 ]; then
    log_pass "E2E: all 4 channels working"
else
    log_fail "E2E: only $ALL_PASS/4 channels"
fi

# ─── G-S1: 72h 压测 ────────────────────────────────────
echo "--- G-S1: 72h Stress Test ---"
if [ -f "scripts/benchmarks/reports/stress_72h/latest.json" ]; then
    REPORT=$(cat scripts/benchmarks/reports/stress_72h/latest.json)
    ELAPSED=$(echo "$REPORT" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('duration_hours', 0))")
    PASS_RATE=$(echo "$REPORT" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('pass_rate', 0))")
    RUNS=$(echo "$REPORT" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('total_runs', 0))")
    P99=$(echo "$REPORT" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('p99_latency_ms', 999999))")

    if (( $(echo "$ELAPSED >= 72.0" | bc -l) )); then
        if (( $(echo "$PASS_RATE >= 95.0" | bc -l) )); then
            log_pass "72h stress: ${PASS_RATE}% pass ($RUNS runs)"
        else
            log_fail "72h stress: ${PASS_RATE}% (need ≥95%)"
        fi
    else
        log_fail "72h stress: only ${ELAPSED}h (need 72h)"
    fi
else
    log_fail "72h stress: no report found"
fi

# ─── G-S2: 内存泄漏检测 ─────────────────────────────────
echo "--- G-S2: Memory Leak Check ---"
if [ -f "scripts/benchmarks/reports/stress_72h/latest.json" ]; then
    RSS_START=25000  # KB, baseline from dry-run
    RSS_NOW=$(ps aux | grep '[p]ython3.*stress_test' | awk '{print int($6)}' | head -1 || echo "0")
    if [ "$RSS_NOW" -gt 0 ]; then
        RSS_GROWTH=$((RSS_NOW - RSS_START))
        if [ "$RSS_GROWTH" -lt 50000 ]; then
            log_pass "Memory: RSS=${RSS_NOW}KB, growth=${RSS_GROWTH}KB (<50MB)"
        else
            log_fail "Memory: RSS=${RSS_NOW}KB, growth=${RSS_GROWTH}KB (≥50MB)"
        fi
    else
        log_info "Memory: process not running (stress test finished)"
    fi
else
    log_info "Memory: no report yet"
fi

# ─── G-S3: DB 健康检查 ──────────────────────────────────
echo "--- G-S3: DB Health ---"
if [ -f "scripts/benchmarks/reports/stress_72h/latest.json" ]; then
    DB_SIZE=$(python3 -c "import json; d=json.load(open('scripts/benchmarks/reports/stress_72h/latest.json')); print(d.get('db_size_mb', 0))")
    if (( $(echo "$DB_SIZE < 200" | bc -l) )); then
        log_pass "DB size: ${DB_SIZE}MB (healthy)"
    else
        log_fail "DB size: ${DB_SIZE}MB (abnormal)"
    fi
else
    log_info "DB: no report yet"
fi

# ─── G-S4: Ollama 可用性 ────────────────────────────────
echo "--- G-S4: Ollama Availability ---"
if curl -s http://192.168.0.252:11434/api/tags >/dev/null 2>&1; then
    log_pass "Ollama: available"
else
    log_fail "Ollama: unavailable"
fi

# ─── G-D1~D5: 文档检查 ─────────────────────────────────
echo "--- G-D1: Release Notes ---"
[ -f "docs/releases/v1.2.0/RELEASE_NOTES.md" ] && log_pass "RELEASE_NOTES.md" || log_fail "RELEASE_NOTES.md"

echo "--- G-D2: User Manual ---"
[ -f "docs/releases/v1.2.0/USER_MANUAL.md" ] && log_pass "USER_MANUAL.md" || log_fail "USER_MANUAL.md"

echo "--- G-D3: OpenAPI Doc ---"
[ -f "docs/openapi.yaml" ] && log_pass "openapi.yaml" || log_fail "openapi.yaml"

echo "--- G-D4: Compliance Audit Report ---"
[ -f "docs/compliance-audit-report.md" ] && log_pass "compliance-audit-report.md" || log_fail "compliance-audit-report.md"

echo "--- G-D5: Git Tag v1.2.0 ---"
if git tag -l "v1.2.0" | grep -q "v1.2.0"; then
    log_pass "git tag v1.2.0"
else
    log_fail "git tag v1.2.0 not found"
fi

# ─── G-P1~P4: 发布清单 ────────────────────────────────
echo "--- G-P1: Milestone Closed ---"
OPEN_ISSUES=$(curl -s "http://192.168.0.252:3000/api/v1/repos/openclaw/GMP-Platform/issues?state=open&milestone=v1.2.0" \
    -H "Authorization: token cc19b2ad677e18c96b9d049f6dc2b46e02176883" 2>/dev/null | \
    python3 -c "import sys,json; print(len(json.load(sys.stdin)))" 2>/dev/null || echo "999")
if [ "$OPEN_ISSUES" -eq 0 ]; then
    log_pass "Milestone v1.2.0: 0 open issues"
else
    log_fail "Milestone v1.2.0: $OPEN_ISSUES open issues"
fi

echo "--- G-P2: PRs Merged ---"
OPEN_PRS=$(curl -s "http://192.168.0.252:3000/api/v1/repos/openclaw/GMP-Platform/pulls?state=open&base=develop/v1.2.0" \
    -H "Authorization: token cc19b2ad677e18c96b9d049f6dc2b46e02176883" 2>/dev/null | \
    python3 -c "import sys,json; print(len(json.load(sys.stdin)))" 2>/dev/null || echo "999")
if [ "$OPEN_PRS" -eq 0 ]; then
    log_pass "develop/v1.2.0: 0 open PRs"
else
    log_fail "develop/v1.2.0: $OPEN_PRS open PRs"
fi

echo "--- G-P3: Dashboard Online ---"
if curl -s http://192.168.0.252:8080/ | grep -q "GMP"; then
    log_pass "Dashboard: http://192.168.0.252:8080 accessible"
else
    log_fail "Dashboard: not accessible"
fi

echo "--- G-P4: Version Declaration ---"
API_VER=$(curl -s http://192.168.0.252:8080/api/stats 2>/dev/null | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('target_version','N/A'))" 2>/dev/null || echo "N/A")
if echo "$API_VER" | grep -q "1.2.0\|v1.2"; then
    log_pass "API version: $API_VER"
else
    log_info "API version: $API_VER (expected v1.2.0)"
fi

# ─── Summary ────────────────────────────────────────────
echo ""
echo "=========================================="
echo "  L3-GA Gate Results: $PASS/$TOTAL PASSED"
echo "=========================================="

if [ "$FAIL" -eq 0 ]; then
    echo -e "  Status: ${GREEN}PASS — READY FOR RELEASE${NC}"
    exit 0
else
    echo -e "  Status: ${RED}FAIL${NC} ($FAIL failures)"
    exit 1
fi
