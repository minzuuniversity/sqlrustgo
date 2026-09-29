#!/usr/bin/env bash
# v4.1.0 Alpha Quality Gate.
#
# This script verifies quality and governance blockers. It is deliberately
# separate from the Alpha Entry check so an entry PASS cannot be mistaken for
# feature completion or gate quality.
#
# Modeled on scripts/gate/check_alpha_quality_v3.12.0.sh per STAGE_CONFIG ALPHA
# stage required_gates (replaces v3.12.0 hard-coded paths with v4.1.0 ones).

set -uo pipefail

cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

PASS=0
TOTAL=0
BLOCKERS=0
SKIPPED=0
LOG_DIR="docs/releases/v4.1.0/logs"
mkdir -p "$LOG_DIR"

check() {
  local label="$1"
  local cmd="$2"
  local log="$LOG_DIR/alpha_quality_${label}_$(git rev-parse --short HEAD 2>/dev/null || echo unknown)_$(date +%Y%m%d_%H%M%S).log"
  TOTAL=$((TOTAL + 1))
  printf '  [%s] ' "$label"
  # Fast-mode sentinel: command literal "false" means the check was intentionally
  # skipped, not failed. This allows ALPHA_QUALITY_FAST_TEST=1 to be a regression
  # fixture for BETA-gate hygiene without producing false BLOCKERS.
  if [ "${ALPHA_QUALITY_FAST_TEST:-0}" = "1" ] && [ "$cmd" = "false" ]; then
    echo "SKIP (fast-mode fixture; production runs required)"
    SKIPPED=$((SKIPPED + 1))
    return 0
  fi
  if eval "$cmd" >"$log" 2>&1; then
    hash=$(sha256sum "$log" 2>/dev/null | cut -d' ' -f1 || echo unavailable)
    echo "PASS (log=$log sha256=$hash)"
    PASS=$((PASS + 1))
  else
    hash=$(sha256sum "$log" 2>/dev/null | cut -d' ' -f1 || echo unavailable)
    echo "FAIL (log=$log sha256=$hash)"
    sed 's/^/    /' "$log" | tail -30
    BLOCKERS=$((BLOCKERS + 1))
  fi
}

echo "=== v4.1.0 Alpha Quality Gate ==="
echo "Branch: $(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown) @ $(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
echo "Boundary: hard quality gates; registered exclusions are visible debt, not PASS evidence."
echo ""
echo "Note: check_docs_consistency.sh is a version-agnostic cross-version check"
echo "(covers v3.0.0..v3.12.0 only). It is run as part of alpha ENTRY (E2) not"
echo "alpha QUALITY. Pre-existing failures in that gate (VERSION_HISTORY.md"
echo "current version + v3.12.0 CHANGELOG duplicate commit 355b5a3837) are"
echo "not v4.1.0 regressions and are tracked separately per open_issues §3."
echo ""

ANTI_FAB_CMD="bash scripts/gate/check_anti_fabrication.sh"
ANTI_IGNORE_CMD="bash scripts/gate/check_anti_ignore_gate.sh"
ARCH_INVARIANTS_CMD="bash scripts/gate/check_arch_invariants.sh"
ARCH3_CMD="bash scripts/gate/check_arch3_no_bypass.sh"
COVERAGE_CMD="bash scripts/gate/check_coverage_v312.sh"
SQL_CORPUS_CMD="bash scripts/gate/check_sql_corpus_gate.sh"
BUILD_CMD="cargo build --all-features"
TEST_CMD="cargo test --all-features --lib"
FMT_CMD="cargo fmt --check"
echo ""

echo "--- Q1: Build & Format ---"
check "Q1_BUILD" "$BUILD_CMD"
check "Q1_FMT" "$FMT_CMD"

echo ""
echo "--- Q2: Anti-Fabrication Policy ---"
check "Q2_ANTI_FAB" "$ANTI_FAB_CMD"

echo ""
echo "--- Q3: Anti-Ignore Gate ---"
check "Q3_ANTI_IGNORE" "$ANTI_IGNORE_CMD"

echo ""
echo "--- Q4: Architecture Invariants ---"
check "Q4_ARCH_INVARIANTS" "$ARCH_INVARIANTS_CMD"
check "Q4_ARCH3_NO_BYPASS" "$ARCH3_CMD"

echo ""
echo "--- Q5: SQL Corpus Gate ---"
check "Q5_SQL_CORPUS" "$SQL_CORPUS_CMD"

echo ""
echo "--- Q6: Test Suite Compile ---"
check "Q6_TEST_LIB" "$TEST_CMD"

# Q7 coverage is NOT included in alpha quality per the v3.12.0 alpha template
# (see scripts/gate/check_alpha_quality_v3.12.0.sh). Coverage measurement
# has known instability for cross-version L1_8 framework (cargo llvm-cov
# can miss extracting per-crate percentages, returning 58% / 85% alternately).
# v4.1.0 alpha-quality follows v3.12.0 alpha-quality scope. Coverage >= 50%
# is a STAGE.yaml alpha_to_beta exit criterion (tracked separately).
echo ""
echo "--- (Q7 Coverage deferred to alpha_to_beta per STAGE.yaml) ---"
echo "=== v4.1.0 Alpha Quality Summary ==="
echo "PASS: $PASS / $TOTAL"
echo "SKIPPED: $SKIPPED"
echo "BLOCKERS: $BLOCKERS"

if [ "$BLOCKERS" -gt 0 ]; then
    echo "STATUS: ALPHA QUALITY BLOCKED"
    exit 1
fi
echo "STATUS: ALPHA QUALITY PASS"
exit 0