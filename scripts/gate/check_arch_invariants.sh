#!/bin/bash
# Architectural invariant checker for C-ARCH-01~05
# Exit 0 only if ALL pass; exit 1 on any FAIL with evidence

set -e

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0

echo "=== C-ARCH Invariant Check ==="
echo ""

# C-ARCH-01: LocalExecutor has NO txn_manager field
echo "[C-ARCH-01] Checking LocalExecutor has NO txn_manager field..."
TXN_MANAGER=$(grep -n "txn_manager:" crates/executor/src/local_executor.rs 2>/dev/null || true)
if [ -n "$TXN_MANAGER" ]; then
    echo "FAIL: C-ARCH-01 violated - txn_manager field found in LocalExecutor"
    echo "Evidence: $TXN_MANAGER"
    FAIL=$((FAIL+1))
else
    echo "PASS: C-ARCH-01"
    PASS=$((PASS+1))
fi
echo ""

# C-ARCH-02: LocalExecutor has NO write_buffer field
echo "[C-ARCH-02] Checking LocalExecutor has NO write_buffer field..."
WRITE_BUFFER=$(grep -n "write_buffer:" crates/executor/src/local_executor.rs 2>/dev/null || true)
if [ -n "$WRITE_BUFFER" ]; then
    echo "FAIL: C-ARCH-02 violated - write_buffer field found in LocalExecutor"
    echo "Evidence: $WRITE_BUFFER"
    FAIL=$((FAIL+1))
else
    echo "PASS: C-ARCH-02"
    PASS=$((PASS+1))
fi
echo ""

# C-ARCH-03: storage.insert/update/delete ONLY in crates/storage/ or crates/executor/
# Only matches actual storage facade calls, not HashMap/Vec insert/delete
echo "[C-ARCH-03] Checking storage.insert/update/delete only in crates/storage or crates/executor/..."
STORAGE_OPS=$(grep -rnE '\bstorage\b.*\.(insert|update|delete)\(' --include="*.rs" \
    crates/ 2>/dev/null | \
    grep -v "crates/storage" | grep -v "crates/executor" || true)

if [ -n "$STORAGE_OPS" ]; then
    echo "FAIL: C-ARCH-03 violated - storage operations outside crates/storage or crates/executor"
    echo "Evidence:"
    echo "$STORAGE_OPS" | head -20
    FAIL=$((FAIL+1))
else
    echo "PASS: C-ARCH-03"
    PASS=$((PASS+1))
fi
echo ""

# C-ARCH-04: No eng.execute(raw_sql) outside parser
# Raw SQL strings passed to execute() should only happen in parser crate
echo "[C-ARCH-04] Checking no eng.execute(raw_sql) outside parser..."
RAW_SQL_CALLS=$(grep -rnE 'execute\s*\(\s*"' --include="*.rs" \
    $(find crates -maxdepth 1 -type d 2>/dev/null) 2>/dev/null | \
    grep -v "crates/parser" || true)

if [ -n "$RAW_SQL_CALLS" ]; then
    echo "FAIL: C-ARCH-04 violated - execute() calls outside parser"
    echo "Evidence:"
    echo "$RAW_SQL_CALLS" | head -20
    FAIL=$((FAIL+1))
else
    echo "PASS: C-ARCH-04"
    PASS=$((PASS+1))
fi
echo ""

# C-ARCH-05: execution_engine.rs < 1500 lines
echo "[C-ARCH-05] Checking execution_engine.rs < 1500 lines..."
EXEC_ENGINE_LINES=$(wc -l < src/execution_engine.rs 2>/dev/null || echo "0")
if [ "$EXEC_ENGINE_LINES" -gt 1500 ]; then
    echo "FAIL: C-ARCH-05 violated - execution_engine.rs has $EXEC_ENGINE_LINES lines (limit: 1500)"
    FAIL=$((FAIL+1))
else
    echo "PASS: C-ARCH-05 (execution_engine.rs: $EXEC_ENGINE_LINES lines)"
    PASS=$((PASS+1))
fi
echo ""

# Summary
echo "=== Summary ==="
echo "PASSED: $PASS"
echo "FAILED: $FAIL"
echo ""

if [ $FAIL -gt 0 ]; then
    echo "Result: FAIL"
    exit 1
else
    echo "Result: ALL PASS"
    exit 0
fi
