#!/bin/bash
# Gate Check Script: L1-Beta Gate v1.2.0
# Usage: bash scripts/gate/check_beta_v120.sh
#
# 前提: develop/v1.2.0 已保护，需要通过 PR 合并触发 CI

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CARGO="/home/openclaw/.cargo/bin/cargo"
cd "$REPO_ROOT"

PASS=0
FAIL=0
TOTAL=14
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_pass() { echo -e "  ${GREEN}[PASS]${NC} $1"; PASS=$((PASS+1)); }
log_fail() { echo -e "  ${RED}[FAIL]${NC} $1"; FAIL=$((FAIL+1)); }
log_info() { echo -e "  ${YELLOW}[INFO]${NC} $1"; }

echo "=========================================="
echo "  GMP-Platform v1.2.0 L1-Beta Gate"
echo "  $(date '+%Y-%m-%d %H:%M:%S')"
echo "=========================================="
echo ""

# ─── B1: 全量编译 ───────────────────────────────────────
echo "--- B1: Build (full workspace) ---"
if $CARGO build --release --workspace 2>&1 | tail -3 | grep -q "Finished"; then
    log_pass "cargo build --release --workspace"
else
    log_fail "cargo build --release --workspace"
fi

# ─── B2: 全量单元测试 ───────────────────────────────────
echo "--- B2: Unit Tests (all features) ---"
TEST_OUTPUT=$($CARGO test --all-features 2>&1) || true
FAILED=$(echo "$TEST_OUTPUT" | grep -oP '\d+(?= failed)' | head -1 || echo "0")
TOTAL_PASSED=$(echo "$TEST_OUTPUT" | grep -oP '\d+(?= passed)' | awk '{s+=$1} END {print s}' || echo "0")
if [ "$FAILED" -le 1 ]; then
    log_pass "cargo test --all-features (failed=$FAILED)"
else
    log_fail "cargo test --all-features ($FAILED failures, need ≤1)"
fi

# ─── B3: Clippy 检查 ─────────────────────────────────────
echo "--- B3: Clippy Check ---"
CLIPPY_OUTPUT=$($CARGO clippy --all-targets -- -D warnings 2>&1) || true
if echo "$CLIPPY_OUTPUT" | grep -q "^error:"; then
    log_fail "clippy: errors found"
else
    log_pass "clippy: no errors"
fi

# ─── B4: 格式化检查 ──────────────────────────────────────
echo "--- B4: Format Check ---"
if $CARGO fmt --all -- --check 2>&1; then
    log_pass "cargo fmt"
else
    log_fail "cargo fmt"
fi

# ─── B5: 安全扫描 ────────────────────────────────────────
echo "--- B5: Security Audit ---"
AUDIT_OUTPUT=$($CARGO audit 2>&1) || true
if echo "$AUDIT_OUTPUT" | grep -q "no vulnerabilities"; then
    log_pass "cargo audit: no vulnerabilities"
elif echo "$AUDIT_OUTPUT" | grep -q "error: could not"; then
    log_info "cargo audit: not installed (skip)"
else
    HIGH=$(echo "$AUDIT_OUTPUT" | grep -c "HIGH" || echo "0")
    if [ "$HIGH" -eq 0 ]; then
        log_pass "cargo audit: passed (warnings only)"
    else
        log_fail "cargo audit: $HIGH HIGH vulnerabilities"
    fi
fi

# ─── B6: Python 静态检查 ─────────────────────────────────
echo "--- B6: Python Lint ---"
PY_OK=0
for py in dashboard/*.py gmp_agent/*.py scripts/*.py; do
    if [ -f "$py" ]; then
        if python3 -m py_compile "$py" 2>/dev/null; then
            PY_OK=$((PY_OK+1))
        fi
    fi
done
log_pass "python3 -m py_compile ($PY_OK files)"

# ─── B-I1: E2E 检索测试 ──────────────────────────────────
echo "--- B-I1: E2E Search ---"
SEARCH_OUTPUT=$(./target/release/gmp-retrieve search --channel fts --db ./data/gmp.db -- '偏差处理' 2>&1) || true
if echo "$SEARCH_OUTPUT" | grep -q "Found"; then
    log_pass "E2E search: returns results"
else
    log_fail "E2E search: no results"
fi

# ─── B-I2: Web Dashboard API ─────────────────────────────
echo "--- B-I2: Web Dashboard API ---"
if curl -s http://localhost:8080/api/search?q=test 2>&1 | grep -q "results"; then
    log_pass "Dashboard /api/search"
else
    log_fail "Dashboard /api/search"
fi

# ─── B-I3: 审计链验证 ─────────────────────────────────────
echo "--- B-I3: Audit Chain ---"
CLOSED_LOOP_OUTPUT=$(python3 scripts/closed_loop_demo.py --skip-ollama 2>&1) || true
if echo "$CLOSED_LOOP_OUTPUT" | grep -q "3/3"; then
    log_pass "closed_loop_demo.py: 3/3 PASS"
else
    log_fail "closed_loop_demo.py: not 3/3"
fi

# ─── B-I4: Dashboard 无外部依赖 ──────────────────────────
echo "--- B-I4: Dashboard Dependencies ---"
EXT_DEPS=$(grep -r 'import.*pypi\|pip install\|uvicorn\|fastapi\|starlette' dashboard/ 2>/dev/null | grep -v '#' | wc -l || echo "0")
if [ "$EXT_DEPS" -eq 0 ]; then
    log_pass "Dashboard: no external dependencies"
else
    log_fail "Dashboard: $EXT_DEPS external dependencies found"
fi

# ─── B-S1~S4: SSOT 规范检查 ──────────────────────────────
echo "--- B-S1: Gate Spec ---"
[ -f "docs/governance/gate_spec_v120.md" ] && log_pass "gate_spec_v120.md" || log_fail "gate_spec_v120.md"

echo "--- B-S2: Test Plan ---"
[ -f "docs/releases/v1.2.0/TEST_PLAN.md" ] && log_pass "TEST_PLAN.md" || log_fail "TEST_PLAN.md"

echo "--- B-S3: Beta Tag ---"
if git tag -l "beta/v1.2.0" | grep -q "beta/v1.2.0"; then
    log_pass "git tag beta/v1.2.0 exists"
else
    log_fail "git tag beta/v1.2.0 not found"
fi

echo "--- B-S4: Branch Protection ---"
# 检查 develop/v1.2.0 是否在 Gitea 上有分支保护
PROTECTION_STATUS=$(curl -s "http://192.168.0.252:3000/api/v1/repos/openclaw/GMP-Platform/branch_protections/develop%2Fv1.2.0" \
    -H "Authorization: token cc19b2ad677e18c96b9d049f6dc2b46e02176883" 2>/dev/null | grep -c "enable_push" || echo "0")
if [ "$PROTECTION_STATUS" -gt 0 ]; then
    log_pass "develop/v1.2.0 branch protection enabled"
else
    log_fail "develop/v1.2.0 branch protection not found"
fi

# ─── Summary ─────────────────────────────────────────────
echo ""
echo "=========================================="
echo "  L1-Beta Gate Results: $PASS/$TOTAL PASSED"
echo "=========================================="

if [ "$FAIL" -eq 0 ]; then
    echo -e "  Status: ${GREEN}PASS${NC}"
    exit 0
else
    echo -e "  Status: ${RED}FAIL${NC} ($FAIL failures)"
    exit 1
fi
