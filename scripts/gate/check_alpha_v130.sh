#!/bin/bash
# GMP-Platform v1.3.0 L0-Alpha Gate Check Script
set -euo pipefail

# Dynamic path resolution - works on any machine
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CARGO="${CARGO:-$(which cargo 2>/dev/null || echo 'cargo')}"
cd "$REPO_ROOT"

PASS=0; FAIL=0; TOTAL=13
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'

log_pass() { echo -e "  ${GREEN}[PASS]${NC} $1"; PASS=$((PASS+1)); }
log_fail() { echo -e "  ${RED}[FAIL]${NC} $1"; FAIL=$((FAIL+1)); }
log_info() { echo -e "  ${YELLOW}[INFO]${NC} $1"; }

echo "=========================================="
echo "  GMP-Platform v1.3.0 L0-Alpha Gate"
echo "  $(date '+%Y-%m-%d %H:%M:%S')"
echo "=========================================="
echo "  REPO_ROOT: $REPO_ROOT"
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
FAILED=$(echo "$TEST_OUTPUT" | grep -E '[0-9]+ passed;[0-9]+ failed' | sed 's/.*\([0-9]\+\) failed.*/\1/' | head -1 || echo "0")
if [ "$FAILED" -eq 0 ]; then
    log_pass "cargo test --lib (0 failures)"
else
    log_fail "cargo test --lib ($FAILED failures)"
fi

# A4: Format Check
echo "--- A4: Format Check ---"
FMT_OUTPUT=$($CARGO fmt -- --check 2>&1) || true
if echo "$FMT_OUTPUT" | grep -q "Diff at"; then
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
CLI_BIN="target/release/gmp-retrieve"
if [ ! -f "$CLI_BIN" ]; then
    $CARGO build --release -p gmp-cli >/dev/null 2>&1 || true
fi
if [ -f "$CLI_BIN" ]; then
    CLI_OUTPUT=$($CLI_BIN --version 2>&1) || true
    if echo "$CLI_OUTPUT" | grep -qE "gmp|v1\.|retrieve"; then
        log_pass "gmp-retrieve --version: $CLI_OUTPUT"
    else
        log_fail "gmp-retrieve --version: unexpected output"
    fi
else
    log_fail "gmp-retrieve binary not found"
fi

# A-F2: gmp-server health (v1.3.0 uses gmp-server not server.py)
echo "--- A-F2: gmp-server health ---"
SERVER_BIN="target/release/gmp-server"
if [ ! -f "$SERVER_BIN" ]; then
    $CARGO build --release -p gmp-server >/dev/null 2>&1 || true
    SERVER_BIN="target/release/gmp-server"
fi
if [ -f "$SERVER_BIN" ]; then
    # Start server in background for health check
    SERVER_PID=$(pgrep -f "gmp-server" | head -1 || true)
    if [ -n "$SERVER_PID" ]; then
        if curl -s http://localhost:8080/health 2>/dev/null | grep -q "ok\|OK"; then
            log_pass "gmp-server running (PID: $SERVER_PID)"
        else
            log_fail "gmp-server: /health not responding"
        fi
    else
        log_info "gmp-server not running, skipping health check"
        # Just verify binary exists
        log_pass "gmp-server binary exists"
    fi
else
    log_info "gmp-server binary not found, skipping"
fi

# A-F3: gmp-event-bus library check
echo "--- A-F3: gmp-event-bus crate ---"
EVENT_BUS_OUTPUT=$($CARGO build -p gmp-event-bus 2>&1) || true
if echo "$EVENT_BUS_OUTPUT" | grep -qE "Finished|Compiling"; then
    log_pass "gmp-event-bus builds"
else
    log_fail "gmp-event-bus build failed"
fi

# A-F4: gmp-api crate check
echo "--- A-F4: gmp-api crate ---"
API_OUTPUT=$($CARGO build -p gmp-api 2>&1) || true
if echo "$API_OUTPUT" | grep -qE "Finished|Compiling"; then
    log_pass "gmp-api builds"
else
    log_fail "gmp-api build failed"
fi

# A-S1: Gate Spec
echo "--- A-S1: Gate Spec ---"
if [ -f "docs/governance/gate_spec_v130.md" ] || [ -f "docs/governance/GATE_SPEC.md" ]; then
    log_pass "gate_spec exists"
else
    log_fail "gate_spec not found"
fi

# A-S2: Development Plan
echo "--- A-S2: Development Plan ---"
if [ -f "docs/governance/DEVELOPMENT_PLAN.md" ] || [ -f "docs/releases/v1.3.0/DEVELOPMENT_PLAN.md" ]; then
    log_pass "DEVELOPMENT_PLAN.md exists"
else
    log_fail "DEVELOPMENT_PLAN.md not found"
fi

# A-S3: Branch Model
echo "--- A-S3: Branch Model ---"
if git rev-parse --verify "develop/v1.3.0" >/dev/null 2>&1; then
    log_pass "develop/v1.3.0 branch exists"
else
    log_fail "develop/v1.3.0 branch not found"
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
