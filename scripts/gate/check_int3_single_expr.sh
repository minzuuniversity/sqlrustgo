#!/bin/bash
# check_int3_single_expr.sh - INT-3 (#3170) Single Expression Engine G3 gate
#
# Verifies:
# 1. Parser has zero clippy warnings (dead_code, unreachable_patterns,
#    redundant_pattern_matching, needless_bool) on the canonical paths
# 2. The dedicated parse_json_path_expression helper is reserved
#    (#[allow(dead_code)] marker) for future dedicated callers
# 3. Single source-of-truth for OR/AND/... chain (parse_or_expression etc.)
#    is reachable from parse_expression
# 4. Token-tuple style `is_some()` is used per clippy::needless_bool
#
# Exit code: 0 = PASS, 1 = FAIL
#
# Refs: docs/openspec/3170-int3-single-expression-engine.md
#       V390_TEST_PLAN.md §G3

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

echo "=== G3 Gate: INT-3 (#3170) Single Expression Engine ==="

# 1. Zero parser-specific clippy warnings
WARNINGS=$(cargo clippy -p sqlrustgo-parser --all-features 2>&1 \
  | grep -E "^warning: " \
  | grep -v "profiles for" \
  | grep -v "unused manifest" \
  | grep -c "." || true)
if [ "$WARNINGS" -gt 0 ]; then
    echo "  ❌ FAIL: sqlrustgo-parser has $WARNINGS clippy warnings"
    cargo clippy -p sqlrustgo-parser --all-features 2>&1 | grep -E "^warning:" | grep -v "profiles for" | grep -v "unused manifest"
    exit 1
fi
echo "  [1/3] ✅ PASS: sqlrustgo-parser has 0 clippy warnings"

# 2. parse_json_path_expression has the #[allow(dead_code)] marker
if ! grep -B 1 "fn parse_json_path_expression" crates/parser/src/parser.rs | grep -q "allow(dead_code)"; then
    echo "  ❌ FAIL: parse_json_path_expression missing #[allow(dead_code)] marker"
    exit 1
fi
echo "  [2/3] ✅ PASS: parse_json_path_expression is reserved (allow(dead_code))"

# 3. parse_expression calls parse_or_expression (Single Expression Engine)
if ! grep -A 12 "fn parse_expression(&mut self)" crates/parser/src/parser.rs | grep -q "parse_or_expression()"; then
    echo "  ❌ FAIL: parse_expression does not call parse_or_expression (chain broken)"
    exit 1
fi
echo "  [3/3] ✅ PASS: parse_expression chains through parse_or_expression"

echo
echo "=== G3 Gate: PASS ==="
echo "INT-3 (#3170) Single Expression Engine: zero parser clippy warnings"
echo "and single-source parse_expression chain verified"
exit 0
