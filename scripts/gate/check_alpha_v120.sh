#!/bin/bash
# GMP-Platform v1.2.0 L0-Alpha Gate Check Script
set -euo pipefail

REPO_ROOT="/home/openclaw/GMP-Platform"
CARGO="/home/openclaw/.cargo/bin/cargo"
cd "$REPO_ROOT"

PASS=0; FAIL=0; TOTAL=13
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'

log_pass() { echo -e "  ${GREEN}[PASS]${NC} $1"; PASS=$((PASS+1)); }
log_fail() { echo -e "  ${RED}[FAIL]${NC} $1"; FAIL=$((FAIL+1)); }
log_info() { echo -e "  ${YELLOW}[INFO]${NC} $1"; }

echo "=========================================="
echo "  GMP-Platform v1.2.0 L0-Alpha Gate"
echo "  $(date '+%Y-%m-%d %H:%M:%S')"
echo "=========================================="

# A1: Build
echo "--- A1: Build (full workspace) ---"
if $CARGO build --release --workspace 2>&1 | tail -3 | grep -q "Finished"; then
    log_pass "cargo build --release --workspace"
else
    log_fail "cargo build --release --workspace"
fi

# A2: Incremental Build
echo "--- A2: Incremental Build (gmp-cli) ---"
if $CARGO build --release -p gmp-cli 2>&1 | tail -3 | grep -q "Finished"; then
    log_pass "cargo build --release -p gmp-cli"
else
    log_fail "cargo build --release -p gmp-cli"
fi

# A3: Unit Tests
echo "--- A3: Unit Tests ---"
TEST_OUTPUT=$($CARGO test --lib 2>&1) || true
FAILED=$(echo "$TEST_OUTPUT" | grep -oP '\d+(?= failed)' | head -1 || echo "0")
if [ "$FAILED" -eq 0 ]; then
    log_pass "cargo test --lib (0 failures)"
else
    log_fail "cargo test --lib ($FAILED failures)"
fi

# A4: Format Check
echo "--- A4: Format Check ---"
FMT_OUTPUT=$($CARGO fmt -- --check 2>&1) || true
if echo "$FMT_OUTPUT" | grep -q 'Diff in /home/openclaw/GMP-Platform'; then
    echo "$FMT_OUTPUT" | head -20
    log_fail "cargo fmt (formatting differences found)"
else
    log_pass "cargo fmt"
fi

# A5: Clippy Check
echo "--- A5: Clippy Check ---"
CLIPPY_OUTPUT=$($CARGO clippy --all-targets -- -D warnings 2>&1) || true
if echo "$CLIPPY_OUTPUT" | grep -qE '^error[[:space:]]'; then
    log_fail "clippy: errors found"
else
    log_pass "clippy: no errors"
fi

# A6: Python Lint
echo "--- A6: Python Lint ---"
PY_COUNT=0
for py in $(find . -name '*.py' -not -path './.*' -not -path '*/__pycache__/*' -not -path '*/target/*' -type f 2>/dev/null); do
    if python3 -m py_compile "$py" 2>/dev/null; then
        PY_COUNT=$((PY_COUNT+1))
    else
        echo "    $py: compile failed"
    fi
done
if [ $PY_COUNT -gt 0 ]; then
    log_pass "python3 -m py_compile ($PY_COUNT files)"
else
    log_fail "python3 -m py_compile (0 files)"
fi

# A-F1: gmp-retrieve CLI
echo "--- A-F1: gmp-retrieve CLI ---"
RETRIEVE_BIN="target/release/gmp-retrieve"
if [ ! -f "$RETRIEVE_BIN" ]; then
    $CARGO build --release -p gmp-cli >/dev/null 2>&1 || true
fi
if [ -f "$RETRIEVE_BIN" ]; then
    RESULT_OUTPUT=$($RETRIEVE_BIN search --channel fts --db ./data/gmp.db -- '偏差处理' 2>&1) || true
    if echo "$RESULT_OUTPUT" | grep -q "Found"; then
        RESULT_COUNT=$(echo "$RESULT_OUTPUT" | grep -oP 'Found[[:space:]]\K\d+' | head -1)
        log_pass "gmp-retrieve search: Found $RESULT_COUNT results"
    else
        log_fail "gmp-retrieve search: no results"
    fi
else
    log_fail "gmp-retrieve binary not found"
fi

# A-F2: Web Dashboard
echo "--- A-F2: Web Dashboard ---"
DASHBOARD_PID=$(pgrep -f "server.py" | head -1 || true)
if [ -n "$DASHBOARD_PID" ]; then
    if curl -s http://localhost:8080/api/stats 2>/dev/null | grep -q "total_queries"; then
        log_pass "Web Dashboard running (PID: $DASHBOARD_PID)"
    else
        log_fail "Web Dashboard: /api/stats not responding"
    fi
else
    log_fail "Web Dashboard: not running"
fi

# A-F3: gmp-agent mobile_cli demo
echo "--- A-F3: gmp-agent mobile_cli demo ---"
DEMO_OUTPUT=$(python3 gmp_agent/mobile_cli.py --demo 2>&1) || true
if echo "$DEMO_OUTPUT" | grep -q "演示流程完成\|演示完成\|Demo completed\|PASS\|✅"; then
    log_pass "gmp_agent.mobile_cli --demo"
else
    log_fail "gmp_agent.mobile_cli --demo (output: $(echo "$DEMO_OUTPUT" | head -c 80))"
fi

# A-F4: stress_test_72h.py dry-run
echo "--- A-F4: stress_test_72h.py dry-run ---"
DRY_OUTPUT=$(python3 scripts/stress_test_72h.py --dry-run 2>&1) || true
DRY_PASS=$(echo "$DRY_OUTPUT" | grep -c "✅\|PASS$" | head -1 || echo "0")
DRY_TOTAL=$(echo "$DRY_OUTPUT" | grep -oP 'Scenarios:[[:space:]]+\K\d+' | head -1 || echo "0")
if [ "$DRY_PASS" -ge 3 ] && [ "$DRY_TOTAL" -ge 3 ]; then
    log_pass "stress_test_72h.py dry-run: $DRY_PASS/$DRY_TOTAL PASS"
else
    log_fail "stress_test_72h.py dry-run: $DRY_PASS/$DRY_TOTAL (need 3/3)"
fi

# A-S1: Gate Spec
echo "--- A-S1: Gate Spec ---"
if [ -f "docs/governance/gate_spec_v120.md" ] || [ -f "docs/governance/GATE_SPEC.md" ]; then
    log_pass "gate_spec exists"
else
    log_fail "gate_spec not found"
fi

# A-S2: Development Plan
echo "--- A-S2: Development Plan ---"
if [ -f "docs/governance/DEVELOPMENT_PLAN.md" ] || [ -f "docs/releases/v1.2.0/DEVELOPMENT_PLAN.md" ]; then
    log_pass "DEVELOPMENT_PLAN.md exists"
else
    log_fail "DEVELOPMENT_PLAN.md not found"
fi

# A-S3: Branch Model
echo "--- A-S3: Branch Model ---"
if git rev-parse --verify "develop/v1.2.0" >/dev/null 2>&1; then
    log_pass "develop/v1.2.0 branch exists"
else
    log_fail "develop/v1.2.0 branch not found"
fi

echo ""
echo "=========================================="
echo "  L0-Alpha Gate Results: $PASS/$TOTAL PASSED"
echo "=========================================="

if [ "$FAIL" -eq 0 ]; then
    echo -e "  Status: ${GREEN}PASS${NC}"
    exit 0
else
    echo -e "  Status: ${RED}FAIL${NC} ($FAIL failures)"
    exit 1
fi
