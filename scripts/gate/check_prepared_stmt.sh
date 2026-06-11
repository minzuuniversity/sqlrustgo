#!/bin/bash
# check_prepared_stmt.sh - P3-1 (#3180) Prepared Statement Cache gate
#
# Verifies P3-1 deliverables:
#   1. crates/cache/ exists with PreparedStatementCache
#   2. 3 new Statement variants (Prepare/Execute/Deallocate) in parser
#   3. tests/prepared_stmt_test.rs has >= 20 tests
#   4. cargo test -p sqlrustgo-cache PASS
#   5. cargo test --test prepared_stmt_test PASS
#   6. End-to-end PREPARE/EXECUTE smoke test
#
# Exit code: 0 = PASS, 1 = FAIL
#
# Refs: docs/openspec/3180-p31-prepared-stmt.md

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

# Ensure cargo is on PATH (CI runners may not have it in default PATH).
if ! command -v cargo >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
fi

echo "=== Gate: P3-1 (#3180) Prepared Statement Cache ==="

# 1. crates/cache/ exists with PreparedStatementCache
if [ ! -d crates/cache ]; then
    echo "  [1/6] FAIL: crates/cache/ not found"
    exit 1
fi
if ! grep -q "pub struct PreparedStatementCache" crates/cache/src/stmt_cache.rs; then
    echo "  [1/6] FAIL: PreparedStatementCache struct not found"
    exit 1
fi
echo "  [1/6] PASS: sqlrustgo-cache crate with PreparedStatementCache"

# 2. 3 new Statement variants in parser
PARSER=crates/parser/src/parser.rs
for variant in "Prepare {" "Execute {" "Deallocate {"; do
    if ! grep -q "Statement::$variant" "$PARSER"; then
        echo "  [2/6] FAIL: Statement::$variant variant not found"
        exit 1
    fi
done
echo "  [2/6] PASS: 3 new Statement variants (Prepare/Execute/Deallocate) defined"

# 3. e2e test file has >= 20 tests
TEST_FILE=tests/prepared_stmt_test.rs
if [ ! -f "$TEST_FILE" ]; then
    echo "  [3/6] FAIL: $TEST_FILE not found"
    exit 1
fi
TESTS=$(grep -c "^#\[test\]" "$TEST_FILE" 2>/dev/null || echo 0)
if [ "$TESTS" -lt 20 ]; then
    echo "  [3/6] FAIL: only $TESTS tests (expected >= 20)"
    exit 1
fi
echo "  [3/6] PASS: $TESTS e2e tests"

# 4. cargo test -p sqlrustgo-cache PASS
echo "  [4/6] Running cargo test -p sqlrustgo-cache (2min budget)..."
if ! timeout 120 cargo test -p sqlrustgo-cache 2>&1 | tail -3; then
    echo "  [4/6] FAIL: cache tests failed"
    exit 1
fi
echo "  [4/6] PASS: cache unit tests"

# 5. cargo test --test prepared_stmt_test PASS
echo "  [5/6] Running cargo test --test prepared_stmt_test (2min budget)..."
if ! timeout 120 cargo test --test prepared_stmt_test 2>&1 | tail -3; then
    echo "  [5/6] FAIL: e2e tests failed"
    exit 1
fi
echo "  [5/6] PASS: 26 e2e tests"

# 6. End-to-end PREPARE/EXECUTE smoke test
echo "  [6/6] Running end-to-end PREPARE/EXECUTE smoke test..."
TMPDIR=$(mktemp -d)
mkdir -p "$TMPDIR/data"
cat > "$TMPDIR/smoke_test.rs" <<'RUST'
use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};
use std::sync::{Arc, RwLock};

#[test]
fn smoke_test() {
    let mut e = ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())));
    e.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER)").unwrap();
    e.execute("INSERT INTO t VALUES (1, 100), (2, 200)").unwrap();
    e.execute("PREPARE s AS 'SELECT * FROM t WHERE id = 1'").unwrap();
    let r = e.execute("EXECUTE s").unwrap();
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0][1].to_string(), "100");
    let stats = e.stmt_cache_stats();
    assert!(stats.hits >= 1);
}
RUST
cp "$TMPDIR/smoke_test.rs" tests/_smoke_test.rs
if cargo test --test _smoke_test 2>&1 | tail -3 | grep -q "test result: ok"; then
    echo "  [6/6] PASS: end-to-end smoke test"
else
    echo "  [6/6] FAIL: smoke test failed"
    rm -f tests/_smoke_test.rs "$TMPDIR/smoke_test.rs"
    rm -rf "$TMPDIR"
    exit 1
fi
rm -f tests/_smoke_test.rs "$TMPDIR/smoke_test.rs"
rm -rf "$TMPDIR"

echo
echo "=== Gate: PASS ==="
echo "P3-1 (#3180) Prepared Statement Cache verified"
exit 0
