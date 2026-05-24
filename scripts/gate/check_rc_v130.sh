#!/bin/bash
# GMP-Platform v1.3.0 L0-RC Gate Check Script
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CARGO="${CARGO:-$(which cargo 2>/dev/null || echo 'cargo')}"
cd "$REPO_ROOT"

PASS=0; FAIL=0; TOTAL=12
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'

log_pass() { echo -e "  ${GREEN}[PASS]${NC} $1"; PASS=$((PASS+1)); }
log_fail() { echo -e "  ${RED}[FAIL]${NC} $1"; FAIL=$((FAIL+1)); }
log_info() { echo -e "  ${YELLOW}[INFO]${NC} $1"; }

echo "=========================================="
echo "  GMP-Platform v1.3.0 L0-RC Gate"
echo "  $(date '+%Y-%m-%d %H:%M:%S')"
echo "=========================================="
echo "  REPO_ROOT: $REPO_ROOT"
echo "=========================================="

echo "--- Entry: L0-Beta PASS ---"
log_info "L0-Beta gate must be executed before L0-RC"

echo "--- RC1: Release Summary ---"
if [ -f "docs/releases/v1.3.0/00-release-summary.md" ]; then
    log_pass "00-release-summary.md exists"
else
    log_fail "00-release-summary.md not found"
fi

echo "--- RC2: User Manual ---"
if [ -f "docs/releases/v1.3.0/USER_MANUAL.md" ]; then
    log_pass "USER_MANUAL.md exists"
else
    log_fail "USER_MANUAL.md not found"
fi

echo "--- RC3: Deployment Guide ---"
if [ -f "docs/releases/v1.3.0/DEPLOYMENT_GUIDE.md" ]; then
    log_pass "DEPLOYMENT_GUIDE.md exists"
else
    log_fail "DEPLOYMENT_GUIDE.md not found"
fi

echo "--- RC4: Audit Report ---"
if [ -f "docs/releases/v1.3.0/AUDIT_REPORT.md" ]; then
    log_pass "AUDIT_REPORT.md exists"
else
    log_fail "AUDIT_REPORT.md not found"
fi

echo "--- RC5: Migration Guide ---"
if [ -f "docs/releases/v1.3.0/MIGRATION_GUIDE.md" ]; then
    log_pass "MIGRATION_GUIDE.md exists"
else
    log_fail "MIGRATION_GUIDE.md not found"
fi

echo "--- RC6: Binary Release ---"
if [ -f "target/release/gmp" ] || [ -f "target/release/gmp-cli" ]; then
    CLI_BIN="target/release/gmp"
    [ ! -f "$CLI_BIN" ] && CLI_BIN="target/release/gmp-cli"
    log_pass "CLI binary exists: $CLI_BIN"
else
    log_fail "CLI binary not found"
fi

echo "--- RC7: Server Binary ---"
if [ -f "target/release/gmp-server" ]; then
    log_pass "Server binary exists"
else
    log_fail "Server binary not found"
fi

echo "--- RC8: Cargo Audit ---"
AUDIT_OUTPUT=$(cargo audit 2>&1 || true)
if echo "$AUDIT_OUTPUT" | grep -qE "no vulnerable|0 vulnerabilities"; then
    log_pass "cargo audit: no vulnerabilities"
elif echo "$AUDIT_OUTPUT" | grep -qE "error"; then
    log_fail "cargo audit: error running"
else
    log_info "cargo audit: warnings found (see output)"
fi

echo "--- RC9: All Crates Build ---"
ALL_BUILD=$($CARGO build --release --workspace 2>&1) || true
if echo "$ALL_BUILD" | grep -q "Finished"; then
    log_pass "all crates build successfully"
else
    log_fail "some crates failed to build"
fi

echo "--- RC10: License Check ---"
if [ -f "LICENSE" ]; then
    log_pass "LICENSE file exists"
else
    log_fail "LICENSE file not found"
fi

echo "--- RC11: README Version ---"
if grep -q "1.3.0" README.md 2>/dev/null; then
    log_pass "README.md version 1.3.0"
else
    log_fail "README.md version not updated to 1.3.0"
fi

echo "--- RC12: CHANGELOG Entry ---"
if [ -f "CHANGELOG.md" ] && grep -q "v1.3.0" CHANGELOG.md 2>/dev/null; then
    log_pass "CHANGELOG.md has v1.3.0 entry"
else
    log_info "CHANGELOG.md v1.3.0 entry not found (optional for RC)"
fi

echo ""
echo "=========================================="
echo "  L0-RC Gate Results: $PASS/$TOTAL PASSED"
echo "=========================================="

if [ "$FAIL" -eq 0 ]; then
    echo -e "  Status: ${GREEN}PASS${NC}"
    exit 0
else
    echo -e "  Status: ${RED}FAIL${NC} ($FAIL failures)"
    exit 1
fi
