#!/usr/bin/env bash
#
# check_v312_stage_boundary.sh -- v3.12 branch/stage boundary guard.
#
# This gate prevents v3.13 planning or release artifacts from becoming part of
# the v3.12 branch before v3.12 Beta/RC/GA has an explicit user-approved
# transition or deferral decision.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

FAIL=0
CHECKS=0

pass() {
    CHECKS=$((CHECKS + 1))
    printf "  [PASS] %s\n" "$1"
}

fail() {
    CHECKS=$((CHECKS + 1))
    FAIL=$((FAIL + 1))
    printf "  [FAIL] %s\n" "$1"
    printf "         %s\n" "$2"
}

echo "=== v3.12 Stage Boundary Guard ==="
echo "Branch: $(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown) @ $(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
echo ""

if grep -q '^version: "v3.12.0"' docs/releases/v3.12.0/STAGE.yaml; then
    pass "STAGE.yaml declares version v3.12.0"
else
    fail "STAGE.yaml version" "docs/releases/v3.12.0/STAGE.yaml must declare version: \"v3.12.0\""
fi

if grep -q '^current_stage: "ALPHA"' docs/releases/v3.12.0/STAGE.yaml; then
    pass "STAGE.yaml current_stage is ALPHA"
else
    fail "STAGE.yaml current_stage" "v3.12 is not allowed to skip Beta/RC by document drift; update this only via a stage transition PR with gate evidence."
fi

if [ ! -d docs/releases/v3.13.0 ]; then
    pass "No v3.13 release directory in v3.12 branch"
else
    fail "v3.13 release directory present" "docs/releases/v3.13.0 belongs on develop/v3.13.0, not develop/v3.12.0. Keep only explicit v3.12 deferral notes in v3.12 docs."
fi

if grep -q '当前开发版.*v3.12.0' README.md; then
    pass "README current development version is v3.12.0"
else
    fail "README current development version" "README must not describe v3.13.0 as the current development version on develop/v3.12.0."
fi

if grep -q 'v3.13.0-ALPHA' README.md; then
    fail "README v3.13 ALPHA badge" "v3.12 branch README must not show a v3.13 ALPHA badge."
else
    pass "README has no v3.13 ALPHA badge"
fi

if grep -q 'STAGE_GOVERNANCE_REMEDIATION_2026-08-18.md' docs/releases/v3.12.0/README.md; then
    pass "v3.12 README links stage governance remediation report"
else
    fail "v3.12 README remediation link" "docs/releases/v3.12.0/README.md must link the stage governance remediation report."
fi

echo ""
echo "Checks: $CHECKS"
echo "Failures: $FAIL"

if [ "$FAIL" -eq 0 ]; then
    echo "STATUS: V312_STAGE_BOUNDARY_PASS"
    exit 0
fi

echo "STATUS: V312_STAGE_BOUNDARY_BLOCKED"
exit 1
