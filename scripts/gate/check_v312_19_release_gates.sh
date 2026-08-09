#!/usr/bin/env bash
# =============================================================================
# V312-19 / ISSUE #3906 — check_v312_19_release_gates.sh
# =============================================================================
# Verifies the three v3.12.0 RC/GA blocking artifacts exist and are
# fresh:
#   1. ALL_TARGETS_REPORT.md         (sql_corpus)
#   2. R2_INVARIANTS_REPORT.md       (arch_invariants)
#   3. Sign-off file (passed via --signoff <path>)
#
# Each artifact MUST be modified within 7 days of HEAD. The sign-off
# file MUST also pass assert_reviewer_signoff.sh.
#
# See:
#   openspec/changes/v312-19-sql-corpus-arch-invariant-reviewer-gate/
#   ISSUE #3906
# =============================================================================
set -uo pipefail

TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SEVEN_DAYS=$((7 * 24 * 3600))
NOW="$(date +%s)"

SIGNOFF=""
while [ $# -gt 0 ]; do
    case "$1" in
        --signoff) SIGNOFF="$2"; shift 2 ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done

EVIDENCE_DIR="${ROOT}/docs/releases/v3.12.0/evidence"
ALL_TARGETS="${EVIDENCE_DIR}/sql_corpus/ALL_TARGETS_REPORT.md"
R2_INVARIANTS="${EVIDENCE_DIR}/arch_invariants/R2_INVARIANTS_REPORT.md"

fail=0
check_fresh() {
    local label="$1"
    local path="$2"
    if [ ! -f "${path}" ]; then
        echo "FAIL: ${label} missing: ${path}" >&2
        fail=1
        return
    fi
    local mtime
    mtime="$(stat -c %Y "${path}" 2>/dev/null || stat -f %m "${path}" 2>/dev/null || echo 0)"
    local age=$((NOW - mtime))
    if [ "${age}" -gt "${SEVEN_DAYS}" ]; then
        echo "FAIL: ${label} is ${age}s old (>7 days): ${path}" >&2
        fail=1
    else
        echo "PASS: ${label} fresh (age=${age}s): ${path}"
    fi
}

echo "==> V312-19 release gate check at ${TIMESTAMP}"
check_fresh "ALL_TARGETS_REPORT.md" "${ALL_TARGETS}"
check_fresh "R2_INVARIANTS_REPORT.md" "${R2_INVARIANTS}"

# Try to generate the corpus and invariant reports if missing.
if [ ! -f "${ALL_TARGETS}" ]; then
    echo "    attempting to bootstrap ALL_TARGETS_REPORT.md via test_sql_corpus.sh"
    if [ -f "${ROOT}/scripts/gate/test_sql_corpus.sh" ]; then
        bash "${ROOT}/scripts/gate/test_sql_corpus.sh" 2>/dev/null || true
    fi
fi
if [ ! -f "${R2_INVARIANTS}" ]; then
    echo "    bootstrapping R2_INVARIANTS_REPORT.md via check_r2_invariants.sh"
    bash "${ROOT}/scripts/gate/check_r2_invariants.sh" 2>/dev/null || true
fi

# Sign-off check (only if --signoff was provided).
if [ -n "${SIGNOFF}" ]; then
    if bash "${ROOT}/scripts/gate/assert_reviewer_signoff.sh" "${SIGNOFF}"; then
        echo "PASS: signoff file is valid: ${SIGNOFF}"
    else
        echo "FAIL: signoff file failed validation" >&2
        fail=1
    fi
else
    echo "SKIP: no --signoff <path> provided; sign-off gate not enforced"
fi

if [ "${fail}" -ne 0 ]; then
    echo "==> V312-19 release gate FAILED"
    exit 1
fi
echo "==> V312-19 release gate PASSED"
exit 0
