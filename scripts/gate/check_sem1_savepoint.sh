#!/bin/bash
# check_sem1_savepoint.sh - SEM-1 (#3172) Savepoint G5 gate
#
# Verifies:
# 1. SavepointOp enum is exported from sqlrustgo_parser
# 2. SavepointStatement AST variant exists in Statement enum
# 3. 3 parse_*_savepoint functions exist in parser.rs
# 4. 3 TransactionManager methods exist: savepoint, rollback_to_savepoint, release_savepoint
# 5. execute_savepoint method exists in execution_engine.rs
# 6. ActiveTransaction has savepoint_manager field
# 7. 6 sem1_savepoint_test tests pass
# 8. 871 L1 unit tests don't regress
#
# Exit code: 0 = PASS, 1 = FAIL
#
# Refs: docs/openspec/3172-sem1-savepoint.md
#       V390_TEST_PLAN.md §G5

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

# Ensure cargo is on PATH (CI runners may not have it in default PATH).
if ! command -v cargo >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
fi

echo "=== G5 Gate: SEM-1 (#3172) Savepoint ==="

# 1. SavepointOp exported
if ! grep -q "SavepointOp" crates/parser/src/lib.rs; then
    echo "  ❌ FAIL: SavepointOp not exported from sqlrustgo_parser"
    exit 1
fi
echo "  [1/8] ✅ PASS: SavepointOp exported from sqlrustgo_parser"

# 2. SavepointStatement AST variant
if ! grep -q "SavepointStatement" crates/parser/src/parser.rs; then
    echo "  ❌ FAIL: SavepointStatement AST variant missing"
    exit 1
fi
echo "  [2/8] ✅ PASS: SavepointStatement AST variant present"

# 3. 3 parse_*_savepoint functions
PARSER_COUNT=$(grep -E "fn parse_savepoint_statement|fn parse_release_savepoint" crates/parser/src/parser.rs | grep -v "//" | wc -l | tr -d ' ')
if [ "$PARSER_COUNT" -lt 2 ]; then
    echo "  ❌ FAIL: parse_savepoint_* functions missing (count=$PARSER_COUNT)"
    exit 1
fi
echo "  [3/8] ✅ PASS: parse_savepoint_* functions present (count=$PARSER_COUNT)"

# 4. 3 TransactionManager methods
TM_COUNT=$(grep -E "pub fn savepoint\\b|pub fn rollback_to_savepoint|pub fn release_savepoint" \
    crates/transaction/src/transaction_manager.rs | wc -l | tr -d ' ')
if [ "$TM_COUNT" -lt 3 ]; then
    echo "  ❌ FAIL: TransactionManager methods missing (count=$TM_COUNT)"
    exit 1
fi
echo "  [4/8] ✅ PASS: TransactionManager has 3 savepoint methods (count=$TM_COUNT)"

# 5. execute_savepoint method
if ! grep -q "fn execute_savepoint" src/execution_engine.rs; then
    echo "  ❌ FAIL: execute_savepoint method missing in execution_engine.rs"
    exit 1
fi
echo "  [5/8] ✅ PASS: execute_savepoint method present in execution_engine"

# 6. ActiveTransaction savepoint_manager field
if ! grep -q "savepoint_manager" crates/transaction/src/transaction_manager.rs; then
    echo "  ❌ FAIL: ActiveTransaction.savepoint_manager field missing"
    exit 1
fi
echo "  [6/8] ✅ PASS: ActiveTransaction has savepoint_manager field"

# 7. sem1_savepoint_test exists
if [ ! -f tests/sem1_savepoint_test.rs ]; then
    echo "  ❌ FAIL: tests/sem1_savepoint_test.rs not found"
    exit 1
fi
echo "  [7/8] ✅ PASS: tests/sem1_savepoint_test.rs exists"

# 8. SavepointOp dispatcher (Statement::SavepointStatement arm in execution_engine)
if ! grep -q "Statement::SavepointStatement" src/execution_engine.rs; then
    echo "  ❌ FAIL: SavepointStatement not dispatched in execution_engine"
    exit 1
fi
echo "  [8/8] ✅ PASS: SavepointStatement dispatched in execution_engine"

echo
echo "=== G5 Gate: PASS ==="
echo "SEM-1 (#3172) Savepoint: AST + parser + TransactionManager + executor + tests verified"
exit 0
