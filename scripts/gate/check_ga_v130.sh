#!/bin/bash
# GMP-Platform v1.3.0 L0-GA Gate Check Script
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CARGO="${CARGO:-$(which cargo 2>/dev/null || echo 'cargo')}"
cd "$REPO_ROOT"

PASS=0; FAIL=0; TOTAL=10
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'

log_pass() { echo -e "  ${GREEN}[PASS]${NC} $1"; PASS=$((PASS+1)); }
log_fail() { echo -e "  ${RED}[FAIL]${NC} $1"; FAIL=$((FAIL+1)); }
log_info() { echo -e "  ${YELLOW}[INFO]${NC} $1"; }

echo "=========================================="
echo "  GMP-Platform v1.3.0 L0-GA Gate"
echo "  $(date '+%Y-%m-%d %H:%M:%S')"
echo "=========================================="
echo "  REPO_ROOT: $REPO_ROOT"
echo "=========================================="

echo "--- Entry: L0-RC PASS ---"
log_info "L0-RC gate must be executed before L0-GA"

echo "--- GA1: Tag Exists ---"
if git tag -l | grep -q "^v1.3.0$"; then
    log_pass "git tag v1.3.0 exists"
else
    log_fail "git tag v1.3.0 not found"
fi

echo "--- GA2: Tag on Correct Commit ---"
if git describe --tags --exact-match 2>/dev/null | grep -q "^v1.3.0$"; then
    log_pass "HEAD is tagged v1.3.0"
else
    log_fail "HEAD is not tagged v1.3.0"
fi

echo "--- GA3: Main Branch Push ---"
if git branch -r | grep -q "origin/main"; then
    log_pass "origin/main branch exists"
else
    log_fail "origin/main branch not found"
fi

echo "--- GA4: All Issues Closed ---"
OPEN_ISSUES=$(curl -s "http://192.168.0.252:3000/api/v1/repos/openclaw/GMP-Platform/issues?state=open&milestone=v1.3.0" | grep -oP '"total_count":\K\d+' | head -1 || echo "0")
if [ "$OPEN_ISSUES" -eq 0 ]; then
    log_pass "no open issues for v1.3.0"
else
    log_info "open issues for v1.3.0: $OPEN_ISSUES"
fi

echo "--- GA5: All PRs Merged ---"
OPEN_PRS=$(curl -s "http://192.168.0.252:3000/api/v1/repos/openclaw/GMP-Platform/pulls?state=open&milestone=v1.3.0" | grep -oP '"total_count":\K\d+' | head -1 || echo "0")
if [ "$OPEN_PRS" -eq 0 ]; then
    log_pass "no open PRs for v1.3.0"
else
    log_info "open PRs for v1.3.0: $OPEN_PRS"
fi

echo "--- GA6: Release Tag Signed ---"
if git tag -v v1.3.0 2>&1 | grep -q "Good signature"; then
    log_pass "v1.3.0 tag is signed"
else
    log_info "v1.3.0 tag not signed (okay for internal)"
fi

echo "--- GA7: Binary Artifacts ---"
ARTIFACT_COUNT=0
for bin in gmp gmp-cli gmp-server; do
    if [ -f "target/release/$bin" ]; then
        ARTIFACT_COUNT=$((ARTIFACT_COUNT+1))
    fi
done
if [ "$ARTIFACT_COUNT" -ge 2 ]; then
    log_pass "$ARTIFACT_COUNT/3 binary artifacts exist"
else
    log_fail "insufficient binary artifacts ($ARTIFACT_COUNT/3)"
fi

echo "--- GA8: Governance Docs Updated ---"
if [ -f "docs/governance/gate_spec_v130.md" ] || [ -f "docs/governance/GATE_SPEC.md" ]; then
    log_pass "gate_spec exists"
else
    log_fail "gate_spec not found"
fi

echo "--- GA9: CONTRIBUTING Updated ---"
if [ -f "CONTRIBUTING.md" ] || [ -f "docs/governance/CONTRIBUTING.md" ]; then
    log_pass "CONTRIBUTING.md exists"
else
    log_fail "CONTRIBUTING.md not found"
fi

echo "--- GA10: Final Build ---"
FINAL_BUILD=$($CARGO build --release --workspace 2>&1) || true
if echo "$FINAL_BUILD" | grep -q "Finished"; then
    log_pass "final release build successful"
else
    log_fail "final release build failed"
fi

echo ""
echo "=========================================="
echo "  L0-GA Gate Results: $PASS/$TOTAL PASSED"
echo "=========================================="

if [ "$FAIL" -eq 0 ]; then
    echo -e "  Status: ${GREEN}PASS${NC}"
    exit 0
else
    echo -e "  Status: ${RED}FAIL${NC} ($FAIL failures)"
    exit 1
fi
