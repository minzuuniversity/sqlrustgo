#!/usr/bin/env bash
# scripts/gate/check_execution_semantics.sh
# Verify no execution path bypasses the canonical ExecutionEngine contract.
# See docs/governance/EXECUTION_SEMANTICS.md (SEM-1).
#
# Exit 0 if all checks pass, non-zero if any violation found.

set -eo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$REPO_ROOT"

violations=0

# Check 1: No direct storage trait method (insert/update/delete) calls
# outside execution_engine.rs. We look for ->Result<...> style call sites
# typical of the storage trait (no .unwrap() in the chained call).
#
# Known legacy/exception paths (tracked for follow-up):
#   - trigger.rs: trigger body DML (covered by INT-4 / VtuGuard)
#   - vector_executor.rs / parallel_vector_executor.rs: vector index internals
#     (not row-table storage; pending I-12 follow-up)
#   - local_executor.rs / parallel_executor.rs: NOT in mod tree (dead code)
#   - test code blocks: any .unwrap() chained is test-only setup
echo "Check 1: forbidden direct storage trait calls outside execution_engine.rs..."
forbidden_calls=$(grep -rn -E "\.(insert|update|delete)\(" \
    --include="*.rs" \
    crates/executor/src/ \
    src/ \
    2>/dev/null \
    | grep -E "storage\.|self\.storage\." \
    | grep -v "execution_engine.rs" \
    | grep -v "trigger\.rs" \
    | grep -v "vector_executor\.rs" \
    | grep -v "parallel_vector_executor\.rs" \
    | grep -v "stored_proc\.rs" \
    | grep -v "local_executor\.rs" \
    | grep -v "parallel_executor\.rs" \
    | grep -v "// allowed:" \
    | grep -v "fn test_" \
    | grep -v "mod tests" \
    || true)
if [[ -n "$forbidden_calls" ]]; then
    echo "  VIOLATION: direct storage trait calls found:"
    echo "$forbidden_calls" | head -10
    violations=$((violations + 1))
else
    echo "  OK: no direct storage trait calls in executor code"
fi

# Check 2: TransactionManager::begin/commit/rollback must only be called
# from execution_engine.rs (engine is sole TX owner).
echo ""
echo "Check 2: TransactionManager lifecycle only via ExecutionEngine..."
tx_calls=$(grep -rn -E "TransactionManager::(begin|commit|rollback)|tx_manager\.(begin|commit|rollback)" \
    --include="*.rs" \
    crates/ \
    src/ \
    2>/dev/null \
    | grep -v "execution_engine.rs" \
    | grep -v "transaction_manager.rs" \
    | grep -v "tests/" \
    | grep -v "// allowed:" \
    || true)
if [[ -n "$tx_calls" ]]; then
    echo "  VIOLATION: TX lifecycle called outside ExecutionEngine:"
    echo "$tx_calls" | head -10
    violations=$((violations + 1))
else
    echo "  OK: TX lifecycle isolated to ExecutionEngine"
fi

# Check 3: No use of legacy local_executor.rs from any in-tree module
echo ""
echo "Check 3: no use of legacy local_executor (excluded from mod tree)..."
legacy_uses=$(grep -rn "use crate::local_executor\|crate::local_executor::" \
    --include="*.rs" \
    crates/ \
    2>/dev/null \
    || true)
if [[ -n "$legacy_uses" ]]; then
    echo "  VIOLATION: legacy local_executor used:"
    echo "$legacy_uses" | head -5
    violations=$((violations + 1))
else
    echo "  OK: legacy local_executor not used"
fi

# Check 4: No use of parallel_executor (also excluded)
echo ""
echo "Check 4: no use of parallel_executor (excluded from mod tree)..."
parallel_uses=$(grep -rn "use crate::parallel_executor\|crate::parallel_executor::" \
    --include="*.rs" \
    crates/ \
    2>/dev/null \
    || true)
if [[ -n "$parallel_uses" ]]; then
    echo "  VIOLATION: parallel_executor used:"
    echo "$parallel_uses" | head -5
    violations=$((violations + 1))
else
    echo "  OK: parallel_executor not used"
fi

echo ""
if [[ $violations -eq 0 ]]; then
    echo "EXECUTION_SEMANTICS: PASS"
    exit 0
else
    echo "EXECUTION_SEMANTICS: FAIL ($violations violation(s))"
    echo "See docs/governance/EXECUTION_SEMANTICS.md"
    exit 1
fi
