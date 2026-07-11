#!/usr/bin/env bash
# check_c3_rollback.sh — verify ACID ROLLBACK correctness on MemoryStorage backend
#
# Implements the gate evidence for issue #3724 [V310-03] C-3a/C-3b acceptance.
# Runs the 5 named tests from V310_DEVELOPMENT_PLAN.md §1.3:
#   - transaction_rollback_undoes_dml    (C-3a)
#   - transaction_update_then_rollback  (C-3a)
#   - transaction_commit_persists_dml   (regression guard)
#   - transaction_delete_then_commit    (regression guard)
#   - failed_insert_does_not_corrupt_table (regression guard)
#
# Usage:
#   ./audit/check_c3_rollback.sh           # run from repo root
#   ./audit/check_c3_rollback.sh --verbose # show full cargo output
#
# Exit codes:
#   0 - all tests pass
#   1 - one or more tests failed
set -u
#
# Output:
#   audit/c3-acid-rollback.log (append-only)

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG="$REPO_ROOT/audit/c3-acid-rollback.log"
TESTS=(
    "transaction_rollback_undoes_dml"
    "transaction_update_then_rollback"
    "transaction_commit_persists_dml"
    "transaction_delete_then_commit"
    "failed_insert_does_not_corrupt_table"
)

mkdir -p "$(dirname "$LOG")"

cd "$REPO_ROOT"

{
    echo "=========================================="
    echo "C-3 ACID Rollback gate run"
    echo "Date: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
    echo "Host: $(uname -srm)"
    echo "Branch: $(git rev-parse --abbrev-ref HEAD)"
    echo "Commit: $(git rev-parse HEAD)"
    echo "Tests: ${#TESTS[@]}"
    echo "=========================================="
} >> "$LOG"

verbose=0
if [[ "${1:-}" == "--verbose" ]]; then
    verbose=1
fi

overall=0

for test in "${TESTS[@]}"; do
    echo "--- Running $test ---" | tee -a "$LOG"
    if [[ $verbose -eq 1 ]]; then
        if ! cargo test --test dml_integration_test -- "$test" --nocapture 2>&1 | tee -a "$LOG"; then
            overall=1
        fi
    else
        if ! cargo test --test dml_integration_test -- "$test" 2>&1 | tee -a "$LOG" | grep -q "^test $test \.\.\. ok$"; then
            overall=1
            echo "FAIL: $test" | tee -a "$LOG"
        else
            echo "PASS: $test" | tee -a "$LOG"
        fi
    fi
done

echo "" | tee -a "$LOG"
echo "==========================================" | tee -a "$LOG"
if [[ $overall -eq 0 ]]; then
    echo "RESULT: PASS (${#TESTS[@]}/${#TESTS[@]} tests)" | tee -a "$LOG"
else
    echo "RESULT: FAIL" | tee -a "$LOG"
fi
echo "Log: $LOG" | tee -a "$LOG"
echo "==========================================" | tee -a "$LOG"

exit $overall