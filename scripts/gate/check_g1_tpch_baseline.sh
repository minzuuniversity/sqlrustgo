#!/usr/bin/env bash
# Gate G1: TPC-H 22/22 baseline.
#
# Verifies that TPC-H Q1..Q22 all PASS and the combined deterministic
# result hash matches the v3.8.0 baseline. See:
#   - openspec/changes/g1-tpch-baseline/proposal.md
#   - openspec/changes/g1-tpch-baseline/design.md
#   - docs/releases/v3.9.0/plans/V390_TEST_PLAN.md §G1
#   - issue #3186
#
# Exit codes:
#   0  PASS — all four sub-checks pass
#   1  FAIL — at least one sub-check failed
#   2  DRIFT-acceptable (e.g., baseline placeholder pending)
#
# Usage:
#   bash scripts/gate/check_g1_tpch_baseline.sh         # full run
#   bash scripts/gate/check_g1_tpch_baseline.sh --dry-run
#
# Environment:
#   TPCH_DATA_DIR     path to TPC-H .tbl files (default: ~/sqlrustgo-tpch/data)
#   TPCH_TIMEOUT_S    per-query timeout seconds (default: 300)

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

HASH_FILE="${REPO_ROOT}/tests/tpch_hashes_v380.json"
PYTHON_SCRIPT="${REPO_ROOT}/scripts/gate/tpch_hash_compare.py"

DRY_RUN=0
if [[ "${1:-}" == "--dry-run" ]]; then
    DRY_RUN=1
fi

color_red()   { printf '\033[0;31m%s\033[0m' "$*"; }
color_green() { printf '\033[0;32m%s\033[0m' "$*"; }
color_yellow() { printf '\033[0;33m%s\033[0m' "$*"; }

step() { echo ""; echo "=== $* ==="; }
pass() { echo "  $(color_green PASS): $*"; }
fail() { echo "  $(color_red FAIL): $*"; }
warn() { echo "  $(color_yellow WARN): $*"; }

if [[ "${DRY_RUN}" == "1" ]]; then
    step "G1 dry-run"
    echo "  step 1: cargo test --test tpch_gate_test  (must be 22/22 PASS)"
    echo "  step 2: cargo test --test tpch_full_22_test  (must be 22/22 PASS)"
    echo "  step 3: cargo test --test tpch_hash_test  (must match baseline)"
    echo "  step 4: python3 ${PYTHON_SCRIPT} --check <hash-from-${HASH_FILE##*/}>  (must exit 0)"
    echo "  hash_file: ${HASH_FILE}"
    echo "  python:    ${PYTHON_SCRIPT}"
    exit 0
fi

OVERALL_RC=0
HASH_SHORT=""

# ---------------------------------------------------------------------------
# 0. Preflight
# ---------------------------------------------------------------------------
step "G1 preflight"

if ! command -v python3 >/dev/null 2>&1; then
    fail "python3 not on PATH"
    exit 1
fi
if ! command -v cargo >/dev/null 2>&1; then
    fail "cargo not on PATH"
    exit 1
fi
if [[ ! -f "${PYTHON_SCRIPT}" ]]; then
    fail "Python helper not found: ${PYTHON_SCRIPT}"
    exit 1
fi
if [[ ! -f "${HASH_FILE}" ]]; then
    fail "Hash baseline not found: ${HASH_FILE}"
    exit 1
fi

EXPECTED_HASH="$(python3 -c "
import json, sys
try:
    with open('${HASH_FILE}') as f:
        d = json.load(f)
    h = d.get('tpc_h_hash_sha256', '')
    if not isinstance(h, str) or len(h) != 64:
        sys.exit('hash missing or wrong length')
    print(h)
except Exception as e:
    sys.exit(f'read error: {e}')
" 2>/dev/null)" || {
    fail "could not parse ${HASH_FILE} (placeholder or malformed?)"
    warn "  action: run \\`python3 ${PYTHON_SCRIPT} --capture\\` and commit the hash"
    exit 2
}

HASH_SHORT="${EXPECTED_HASH:0:8}"
pass "hash file parsed: ${HASH_SHORT}..."

# ---------------------------------------------------------------------------
# 1. tpch_gate_test (22/22 inline)
# ---------------------------------------------------------------------------
step "G1 step 1/4: cargo test --test tpch_gate_test"
if timeout 1800 cargo test --test tpch_gate_test 2>&1 | tail -5; then
    pass "tpch_gate_test executed (see output above for the 22/22 line)"
else
    fail "tpch_gate_test exited non-zero (see output above)"
    OVERALL_RC=1
fi

# ---------------------------------------------------------------------------
# 2. tpch_full_22_test (22/22 sf=0.1)
# ---------------------------------------------------------------------------
step "G1 step 2/4: cargo test --test tpch_full_22_test"
if timeout 1800 env TPCH_FORCE=1 cargo test --test tpch_full_22_test 2>&1 | tail -10; then
    pass "tpch_full_22_test executed"
else
    fail "tpch_full_22_test exited non-zero (see output above)"
    OVERALL_RC=1
fi

# ---------------------------------------------------------------------------
# 3. tpch_hash_test (Rust regression test)
# ---------------------------------------------------------------------------
step "G1 step 3/4: cargo test --test tpch_hash_test"
if timeout 1800 cargo test --test tpch_hash_test 2>&1 | tail -10; then
    pass "tpch_hash_test executed"
else
    fail "tpch_hash_test exited non-zero"
    OVERALL_RC=1
fi

# ---------------------------------------------------------------------------
# 4. Python --check (independent verification)
# ---------------------------------------------------------------------------
step "G1 step 4/4: python3 tpch_hash_compare.py --check ${HASH_SHORT}..."
if timeout 1800 python3 "${PYTHON_SCRIPT}" --check "${EXPECTED_HASH}" 2>&1; then
    pass "python3 --check passed (hash matches baseline)"
else
    fail "python3 --check failed (hash mismatch)"
    OVERALL_RC=1
fi

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
echo ""
if [[ "${OVERALL_RC}" -eq 0 ]]; then
    echo "G1 PASS: TPC-H 22/22 baseline verified, hash=${HASH_SHORT}..."
else
    echo "G1 FAIL: at least one sub-check failed (see above)"
fi
exit "${OVERALL_RC}"
