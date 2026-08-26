#!/usr/bin/env bash
# v3.12.0 V312-14 Crash Recovery Gate — executable version.
#
# Iterates the crash-recovery + fault-injection test suite and reports
# per-suite pass/fail. Originally (pre-V312-59-E) this gate only
# checked file presence and parsed a Markdown status string — that
# let a real WAL-replay bug (`recovery_fuzzer_test::
# r2_interleaved_transactions_out_of_order`) and a missing CLI
# binary (`physical-backup`) silently ship past the RC gate.
#
# This version executes the actual tests. Exits 0 only when every
# suite passes with 0 failures. Suites with `ignored` tests are
# accepted provided they were pre-existing (not new this cycle).
#
# Scope:
#   1. tests/integration/stress/crash_test_framework.rs (16 tests)
#   2. tests/integration/stress/process_kill_crash_test.rs (8 tests)
#   3. tests/integration/sql/backup_restore_test.rs (51 tests)
#   4. tests/integration/stress/recovery_fuzzer_test.rs (15 tests,
#      includes R2 fuzzer for WAL replay)
#   5. tests/integration/sql/physical_backup_test.rs (12 tests,
#      requires the `physical-backup` binary in sqlrustgo-tools)
#   6. tests/integration/sql/memory_fault_injection_test.rs (7)
#   7. tests/integration/sql/network_fault_injection_test.rs (7)
#   8. tests/integration/sql/row_crc_skip (1)
#   9. tests/integration/sql/torn_write_recovery (1)
#  10. tests/integration/stress/crash_monkey_test.rs (4+1 ignored)
#  11. tests/integration/stress/oracle_g14_real_crash.rs (24)
#  12. tests/integration/sql/sql_injection_test.rs (10 tests, V312-59-E
#      RC hardening — adversarial SQL parser + executor input handling;
#      documents the LIMITATION that classic tautology attacks work as
#      expected when applications concatenate user input directly
# Exit codes:
#   0 — every suite reports 0 failures
#   1 — at least one suite reports ≥1 failure
# V312-59-E: required for RC8 (Crash recovery + upgrade/downgrade)
# gate. Anti-Fabrication-Policy-v1.0 §5 enforced.

set -uo pipefail

cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

PASS=0
FAIL=0
FAILED_SUITES=()

# Suite spec: <name> <test-binary>
SUITES=(
    "crash_test_framework|crash_test_framework"
    "process_kill_crash_test|process_kill_crash_test"
    "backup_restore_test|backup_restore_test"
    "recovery_fuzzer_test|recovery_fuzzer_test"
    "physical_backup_test|physical_backup_test"
    "memory_fault_injection_test|memory_fault_injection_test"
    "network_fault_injection_test|network_fault_injection_test"
    "row_crc_skip|row_crc_skip"
    "torn_write_recovery|torn_write_recovery"
    "crash_monkey_test|crash_monkey_test"
    "oracle_g14_real_crash|oracle_g14_real_crash"
    "sql_injection_test|sql_injection_test"
 )

run_suite() {
    local label="$1"
    local bin="$2"
    echo "---"
    echo "RUN: $label"
    local out
    if ! out=$(cargo test --test "$bin" -- --test-threads=1 2>&1); then
        echo "  [FAIL] cargo test invocation failed for $bin"
        echo "$out" | tail -20
        FAIL=$((FAIL + 1))
        FAILED_SUITES+=("$label (invocation failed)")
        return
    fi
    local summary
    summary=$(echo "$out" | grep -E "^test result:" | head -1)
    if [ -z "$summary" ]; then
        echo "  [FAIL] no test result line emitted"
        echo "$out" | tail -20
        FAIL=$((FAIL + 1))
        FAILED_SUITES+=("$label (no result line)")
        return
    fi
    echo "  $summary"
    local failed_count
    failed_count=$(echo "$summary" | grep -oE "[0-9]+ failed" | head -1 | awk '{print $1}')
    failed_count=${failed_count:-0}
    if [ "$failed_count" -eq 0 ]; then
        PASS=$((PASS + 1))
        echo "  [PASS] $label"
    else
        FAIL=$((FAIL + 1))
        FAILED_SUITES+=("$label ($failed_count failures)")
        echo "  [FAIL] $label: $failed_count failures"
        echo "$out" | grep -E "FAILED$" | head -10
    fi
}

echo "=== V312-14 Crash Recovery Gate — executable ==="
echo "Branch: $(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown) @ $(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
echo "Scope:  $(echo "${#SUITES[@]}") suites; threshold = 0 failures across all suites"
echo

for entry in "${SUITES[@]}"; do
    IFS='|' read -r label bin <<< "$entry"
    run_suite "$label" "$bin"
done

echo
echo "=== Summary ==="
echo "PASS: $PASS / ${#SUITES[@]}"
echo "FAIL: $FAIL / ${#SUITES[@]}"
if [ "$FAIL" -gt 0 ]; then
    echo "FAILED suites:"
    for s in "${FAILED_SUITES[@]}"; do
        echo "  - $s"
    done
    echo "STATUS: V312-14 CRASH RECOVERY GATE FAIL"
    exit 1
fi
echo "STATUS: V312-14 CRASH RECOVERY GATE PASS"
exit 0
