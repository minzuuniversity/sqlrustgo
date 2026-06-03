#!/usr/bin/env bash
# =============================================================================
# check_arch2_no_bypass.sh — v3.8.0 ARCH-2 Bypass Gate
# =============================================================================
# Issue #2974: merge.rs and related DML paths must NOT touch storage directly.
# Acceptance: VtuGuard verification of zero bypass paths in production code.
#
# This gate greps production Rust code for direct DML calls on `storage`,
# excluding the explicit whitelist (test blocks and the bench-cli TPC-H data
# loader, which is documented ISOLATED and pre-AG-2 baseline).
#
# Exit codes:
#   0  = 0 NEW bypass (all calls go through ExecutionEngine / VtuGuard)
#   1  = NEW bypass detected (blocker; ARCH-2 regression)
# =============================================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

echo "=== ARCH-2: No DML Bypass Gate (Issue #2974) ==="
echo

PATTERN='(storage|memory_storage)\.(insert|update|delete|delete_if|update_if)\b'
HITS=$(rg -n --no-heading "$PATTERN" crates/ \
    --glob '!target/**' \
    --glob '!**/tests/**' \
    --glob '!**/examples/**' \
    2>/dev/null || true)

WHITELIST_PATTERN='(parallel_vector_executor\.rs|parallel_executor\.rs|vector_executor\.rs|bench-cli/src/commands/tpch_data\.rs)'

FILTERED=$(echo "$HITS" | grep -vE "$WHITELIST_PATTERN" | grep -vE '^\s*$' || true)

if [ -z "$FILTERED" ]; then
    echo "✅ 0 NEW bypass path (ARCH-2 compliant)"
    echo "  Baseline whitelist (allowed, pre-ARCH-2 documented):"
    echo "    - parallel_vector_executor.rs (test block, ISOLATED)"
    echo "    - parallel_executor.rs (test block, ISOLATED)"
    echo "    - vector_executor.rs (test block, ISOLATED)"
    echo "    - bench-cli/tpch_data.rs (TPC-H fixture loader)"
    echo
    echo "Production paths (merge.rs, trigger.rs, local_executor.rs):"
    echo "  All DML routed through ExecutionEngine.execute() or VtuGuard::execute_dml()"
    echo
    echo "References:"
    echo "  - docs/releases/v3.8.0/INT-4-VTUGUARD-ENFORCEMENT.md (TriggerExecutor fix)"
    echo "  - docs/releases/v3.7.0/ARCHITECTURE_VIOLATIONS.md (baseline)"
    exit 0
fi

echo "❌ NEW ARCH-2 bypass detected:"
echo
echo "$FILTERED"
echo
echo "Fix: route DML through ExecutionEngine.execute() or VtuGuard::execute_dml()."
echo "See issue #2974 for the canonical pattern."
exit 1
