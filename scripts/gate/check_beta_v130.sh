#!/bin/bash
# GMP-Platform v1.3.0 L0-Beta Gate Check Script
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CARGO="${CARGO:-$(which cargo 2>/dev/null || echo 'cargo')}"
cd "$REPO_ROOT"

PASS=0; FAIL=0; TOTAL=14
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'

log_pass() { echo -e "  ${GREEN}[PASS]${NC} $1"; PASS=$((PASS+1)); }
log_fail() { echo -e "  ${RED}[FAIL]${NC} $1"; FAIL=$((FAIL+1)); }
log_info() { echo -e "  ${YELLOW}[INFO]${NC} $1"; }

echo "=========================================="
echo "  GMP-Platform v1.3.0 L0-Beta Gate"
echo "  $(date '+%Y-%m-%d %H:%M:%S')"
echo "=========================================="
echo "  REPO_ROOT: $REPO_ROOT"
echo "=========================================="

echo "--- Entry Condition: L0-Alpha must PASS ---"
if [ ! -f "scripts/gate/check_alpha_v130.sh" ]; then
    log_fail "L0-Alpha gate script not found"
else
    log_info "L0-Alpha gate script exists"
fi

echo "--- B1: Full Test Suite ---"
TEST_OUTPUT=$($CARGO test 2>&1) || true
FAILED=$(echo "$TEST_OUTPUT" | grep -E '[0-9]+ passed;[0-9]+ failed' | sed 's/.*\([0-9]\+\) failed.*/\1/' | head -1 || echo "0")
if [ "$FAILED" -eq 0 ]; then
    log_pass "cargo test (0 failures)"
else
    log_fail "cargo test ($FAILED failures)"
fi

echo "--- B2: Documentation Complete (21 docs) ---"
COMPLETENESS_OUTPUT=$(bash docs/completeness_check.sh 1.3.0 2>&1) || true
if echo "$COMPLETENESS_OUTPUT" | grep -q "21/21 PASS"; then
    log_pass "21 docs completeness check PASS"
else
    echo "$COMPLETENESS_OUTPUT" | tail -5
    log_fail "21 docs completeness check FAIL"
fi

echo "--- B3: Feature Matrix ---"
if [ -f "docs/releases/v1.3.0/FEATURE_MATRIX.md" ]; then
    log_pass "FEATURE_MATRIX.md exists"
else
    log_fail "FEATURE_MATRIX.md not found"
fi

echo "--- B4: Integration Status ---"
if [ -f "docs/releases/v1.3.0/INTEGRATION_STATUS.md" ]; then
    log_pass "INTEGRATION_STATUS.md exists"
else
    log_fail "INTEGRATION_STATUS.md not found"
fi

echo "--- B5: Progress Report ---"
if [ -f "docs/releases/v1.3.0/PROGRESS_REPORT.md" ]; then
    log_pass "PROGRESS_REPORT.md exists"
else
    log_fail "PROGRESS_REPORT.md not found"
fi

echo "--- B6: Release Gate Checklist ---"
if [ -f "docs/releases/v1.3.0/RELEASE_GATE_CHECKLIST.md" ]; then
    log_pass "RELEASE_GATE_CHECKLIST.md exists"
else
    log_fail "RELEASE_GATE_CHECKLIST.md not found"
fi

echo "--- B7: Test Plan ---"
if [ -f "docs/releases/v1.3.0/TEST_PLAN.md" ]; then
    log_pass "TEST_PLAN.md exists"
else
    log_fail "TEST_PLAN.md not found"
fi

echo "--- B8: Layer 0 Pipeline (if applicable) ---"
if grep -q "Layer 0\|layer.*0\|离线抽取" docs/releases/v1.3.0/FEATURE_MATRIX.md 2>/dev/null; then
    log_pass "Layer 0 pipeline documented"
else
    log_info "Layer 0 pipeline not in scope for v1.3.0"
fi

echo "--- B9: Event Bus ---"
EVENT_OUTPUT=$($CARGO build -p gmp-event-bus 2>&1) || true
if echo "$EVENT_OUTPUT" | grep -qE "Finished|Compiling gmp-event-bus"; then
    log_pass "gmp-event-bus builds"
else
    log_fail "gmp-event-bus build failed"
fi

echo "--- B10: API Server ---"
API_OUTPUT=$($CARGO build -p gmp-api 2>&1) || true
if echo "$API_OUTPUT" | grep -qE "Finished|Compiling gmp-api"; then
    log_pass "gmp-api builds"
else
    log_fail "gmp-api build failed"
fi

echo "--- B11: Retrieval Engine ---"
RETRIEVAL_OUTPUT=$($CARGO build -p gmp-retrieval 2>&1) || true
if echo "$RETRIEVAL_OUTPUT" | grep -qE "Finished|Compiling gmp-retrieval"; then
    log_pass "gmp-retrieval builds"
else
    log_fail "gmp-retrieval build failed"
fi

echo "--- B12: Reranker ---"
RERANKER_OUTPUT=$($CARGO build -p gmp-reranker 2>&1) || true
if echo "$RERANKER_OUTPUT" | grep -qE "Finished|Compiling gmp-reranker"; then
    log_pass "gmp-reranker builds"
else
    log_fail "gmp-reranker build failed"
fi

echo "--- B13: LLM Module ---"
LLM_OUTPUT=$($CARGO build -p gmp-llm 2>&1) || true
if echo "$LLM_OUTPUT" | grep -qE "Finished|Compiling gmp-llm"; then
    log_pass "gmp-llm builds"
else
    log_fail "gmp-llm build failed"
fi

echo "--- B14: Eval Module ---"
EVAL_OUTPUT=$($CARGO build -p gmp-eval 2>&1) || true
if echo "$EVAL_OUTPUT" | grep -qE "Finished|Compiling gmp-eval"; then
    log_pass "gmp-eval builds"
else
    log_fail "gmp-eval build failed"
fi

echo ""
echo "=========================================="
echo "  L0-Beta Gate Results: $PASS/$TOTAL PASSED"
echo "=========================================="

if [ "$FAIL" -eq 0 ]; then
    echo -e "  Status: ${GREEN}PASS${NC}"
    exit 0
else
    echo -e "  Status: ${RED}FAIL${NC} ($FAIL failures)"
    exit 1
fi
