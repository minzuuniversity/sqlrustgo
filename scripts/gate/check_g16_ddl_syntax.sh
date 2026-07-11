#!/bin/bash
# check_g16_ddl_syntax.sh - G16 DDL Syntax Gate (v3.10.0 PR1)
#
# Closes Issue #3727 (V310-06 PR1): CREATE / DROP / USE database AST
# plus parser support and at least 50 positive + negative parse tests.
#
# Verifies:
#   1. Token::Database, Token::Use exist in crates/parser/src/token.rs
#   2. Statement::CreateDatabase / DropDatabase / UseDatabase variants exist
#      in crates/parser/src/parser.rs
#   3. CreateDatabaseStatement / DropDatabaseStatement structs are defined
#   4. parse_create_database / parse_drop_database / parse_use_database
#      functions are implemented in the parser
#   5. The ddl_database_tests module exists in parser.rs and contains
#      at least 50 #[test] functions
#   6. Running cargo test -p sqlrustgo-parser ddl_database_tests passes
#      with >= 50 tests, 0 failures
#   7. The full sqlrustgo-parser test suite has no regression
#
# Exit code: 0 = PASS, 1 = FAIL
#
# Refs: docs/releases/v3.10.0/plans/V310_DEVELOPMENT_PLAN.md §PR1 / Issue #3727

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

PARSER_SRC="crates/parser/src/parser.rs"
TOKEN_SRC="crates/parser/src/token.rs"

echo "=== G16 Gate: DDL Syntax (CREATE/DROP DATABASE, USE) ==="

# ── 1. Tokens ────────────────────────────────────────────────────────────────
grep -q '^[[:space:]]*Database,[[:space:]]*$' "$TOKEN_SRC" || {
    echo "  [1/7] FAIL: Token::Database missing in token.rs"
    exit 1
}
grep -q '^[[:space:]]*Use,[[:space:]]*$' "$TOKEN_SRC" || {
    echo "  [1/7] FAIL: Token::Use missing in token.rs"
    exit 1
}
echo "  [1/7] PASS: Token::Database and Token::Use present"

# ── 2. Statement variants ────────────────────────────────────────────────────
grep -q 'CreateDatabase(CreateDatabaseStatement)' "$PARSER_SRC" || {
    echo "  [2/7] FAIL: Statement::CreateDatabase variant missing"
    exit 1
}
grep -q 'DropDatabase(DropDatabaseStatement)' "$PARSER_SRC" || {
    echo "  [2/7] FAIL: Statement::DropDatabase variant missing"
    exit 1
}
grep -q 'UseDatabase(String)' "$PARSER_SRC" || {
    echo "  [2/7] FAIL: Statement::UseDatabase variant missing"
    exit 1
}
echo "  [2/7] PASS: Statement variants for CREATE/DROP DATABASE, USE present"

# ── 3. Struct definitions ───────────────────────────────────────────────────
grep -q '^pub struct CreateDatabaseStatement' "$PARSER_SRC" || {
    echo "  [3/7] FAIL: CreateDatabaseStatement struct missing"
    exit 1
}
grep -q '^pub struct DropDatabaseStatement' "$PARSER_SRC" || {
    echo "  [3/7] FAIL: DropDatabaseStatement struct missing"
    exit 1
}
echo "  [3/7] PASS: Create/DropDatabaseStatement structs defined"

# ── 4. Parser functions ─────────────────────────────────────────────────────
grep -q 'fn parse_create_database' "$PARSER_SRC" || {
    echo "  [4/7] FAIL: parse_create_database fn missing"
    exit 1
}
grep -q 'fn parse_drop_database' "$PARSER_SRC" || {
    echo "  [4/7] FAIL: parse_drop_database fn missing"
    exit 1
}
grep -q 'fn parse_use_database' "$PARSER_SRC" || {
    echo "  [4/7] FAIL: parse_use_database fn missing"
    exit 1
}
echo "  [4/7] PASS: parse_create_database / parse_drop_database / parse_use_database present"

# ── 5. ≥50 tests in ddl_database_tests module ───────────────────────────────
if ! grep -q '^mod ddl_database_tests' "$PARSER_SRC"; then
    echo "  [5/7] FAIL: ddl_database_tests module missing in parser.rs"
    exit 1
fi
TEST_COUNT=$(grep -c '^[[:space:]]*#\[test\][[:space:]]*$' "$PARSER_SRC" || true)
# We approximate by counting test functions in the ddl_database_tests block.
# A precise awk that scopes to that module:
DDL_TEST_COUNT=$(awk '
    /^mod ddl_database_tests[[:space:]]*\{/ { in_mod=1; next }
    in_mod && /^\}[[:space:]]*$/ { in_mod=0 }
    in_mod && /^[[:space:]]*#\[test\][[:space:]]*$/ { c++ }
    END { print c+0 }
' "$PARSER_SRC")
if [ "$DDL_TEST_COUNT" -lt 50 ]; then
    echo "  [5/7] FAIL: ddl_database_tests has only $DDL_TEST_COUNT tests (need >= 50)"
    exit 1
fi
echo "  [5/7] PASS: ddl_database_tests contains $DDL_TEST_COUNT tests (>= 50)"

# ── 6. cargo test for the module passes ─────────────────────────────────────
DDL_OUTPUT=$(cargo test --lib -p sqlrustgo-parser ddl_database_tests 2>&1)
DDL_EXIT=$?
if [ $DDL_EXIT -ne 0 ]; then
    echo "  [6/7] FAIL: cargo test ddl_database_tests exit $DDL_EXIT"
    echo "$DDL_OUTPUT" | tail -10
    exit 1
fi
DDL_RESULT_LINE=$(echo "$DDL_OUTPUT" | grep -E "test result: ok.*[0-9]+ passed" | head -1)
if [ -z "$DDL_RESULT_LINE" ]; then
    echo "  [6/7] FAIL: no 'test result: ok' found in ddl_database_tests output"
    echo "$DDL_OUTPUT" | tail -10
    exit 1
fi
DDL_PASSED=$(echo "$DDL_RESULT_LINE" | grep -oE "[0-9]+ passed" | grep -oE "[0-9]+")
DDL_FAILED=$(echo "$DDL_RESULT_LINE" | grep -oE "[0-9]+ failed" | grep -oE "[0-9]+" || echo 0)
if [ "${DDL_FAILED:-0}" -ne 0 ]; then
    echo "  [6/7] FAIL: $DDL_FAILED tests failed in ddl_database_tests"
    exit 1
fi
if [ "${DDL_PASSED:-0}" -lt 50 ]; then
    echo "  [6/7] FAIL: only $DDL_PASSED tests passed (need >= 50)"
    exit 1
fi
echo "  [6/7] PASS: cargo test ddl_database_tests: ${DDL_PASSED} passed, 0 failed"

# ── 7. Full parser suite has no regression ───────────────────────────────────
FULL_OUTPUT=$(cargo test --lib -p sqlrustgo-parser 2>&1)
FULL_EXIT=$?
if [ $FULL_EXIT -ne 0 ]; then
    echo "  [7/7] FAIL: full parser suite cargo test exit $FULL_EXIT"
    echo "$FULL_OUTPUT" | tail -10
    exit 1
fi
FULL_RESULT=$(echo "$FULL_OUTPUT" | grep -E "test result: ok" | tail -1)
if [ -z "$FULL_RESULT" ]; then
    echo "  [7/7] FAIL: no 'test result: ok' in full parser output"
    echo "$FULL_OUTPUT" | tail -10
    exit 1
fi
FULL_FAILED=$(echo "$FULL_RESULT" | grep -oE "[0-9]+ failed" | grep -oE "[0-9]+" || echo 0)
if [ "${FULL_FAILED:-0}" -ne 0 ]; then
    echo "  [7/7] FAIL: ${FULL_FAILED} tests failed in full parser suite"
    exit 1
fi
FULL_PASSED=$(echo "$FULL_RESULT" | grep -oE "[0-9]+ passed" | grep -oE "[0-9]+")
echo "  [7/7] PASS: full parser suite: ${FULL_PASSED} passed, 0 failed"

echo "=== G16 DDL Syntax Gate: ALL 7 CHECKS PASS ==="
echo "Issue #3727 (V310-06 PR1) CREATE/DROP DATABASE, USE — closure criterion met."
exit 0
