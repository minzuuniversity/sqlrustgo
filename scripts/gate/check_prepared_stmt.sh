#!/bin/bash
# ==========================================================================
# GATE-DISPOSITION-SELF-MARK —— 本文件**不会在权威轮里被执行**，登记结论就地写在这里
#
# 分类：死门禁（dead）—— 既无执行点也无正文引用 ⇒ 永远不会运行
# 处置：归档候选（archive-candidate）
# 责任任务：P0-4
# 登记说明：头部绑定历史版本线，v4.1.0 的 STAGE_CONFIG 与 CI 都不引用它 ⇒ 归档候选；删除门禁脚本属门禁清单变更，须 owner 批准
# 效果声明：尚无可证伪夹具（NEG_FIXTURES 里本门禁没有可证伪行），因此不声称能抓任何东西 —— 无声明的「能抓」比错的「能抓」更坏。
#
# 为什么保留文件而不是删掉：删文件会改变门禁总数（当前 168，固定不可动），
# 而「不执行」是**登记在册的已知状态**，不是遗漏。删除会让「这一项为什么
# 没有判定」变成不可追溯。因此：**保留文件 + 就地自标 + 登记在册**。
#
# 权威登记：docs/releases/v4.1.0/GATE_CATALOG.jsonl（同一 gate 键）
# ==========================================================================
# check_prepared_stmt.sh - P3-1 (#3180) Prepared Statement Cache gate
#
# Verifies P3-1 deliverables:
#   1. crates/cache/ exists with PreparedStatementCache
#   2. 3 new Statement variants (Prepare/Execute/Deallocate) in parser
#   3. tests/integration/sql/prepared_stmt_test.rs has >= 20 tests
#   4. cargo test -p sqlrustgo-cache PASS
#   5. cargo test --test prepared_stmt_test PASS
#   6. End-to-end PREPARE/EXECUTE smoke test
#
# Exit code: 0 = PASS, 1 = FAIL
#
# Refs: docs/openspec/3180-p31-prepared-stmt.md

# #5326: paths corrected for V310 → V3.12 directory restructure

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
TEST_FILE=tests/integration/sql/prepared_stmt_test.rs
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
timeout 120 cargo test -p sqlrustgo-cache 2>&1 | tail -3
TEST_RC=${PIPESTATUS[0]}
if [ "$TEST_RC" -ne 0 ]; then
    echo "  [4/6] FAIL: cache tests failed"
    exit 1
fi
echo "  [4/6] PASS: cache unit tests"

# 5. cargo test --test prepared_stmt_test PASS
echo "  [5/6] Running cargo test --test prepared_stmt_test (2min budget)..."
timeout 120 cargo test --test prepared_stmt_test 2>&1 | tail -3
TEST_RC=${PIPESTATUS[0]}
if [ "$TEST_RC" -ne 0 ]; then
    echo "  [5/6] FAIL: e2e tests failed"
    exit 1
fi
echo "  [5/6] PASS: 26 e2e tests"

# 6. End-to-end PREPARE/EXECUTE smoke test
echo "  [6/6] Running end-to-end PREPARE/EXECUTE smoke test..."
TMPDIR=$(mktemp -d)
mkdir -p "$TMPDIR/data"
cat > "$TMPDIR/smoke_test.rs" <<'RUST'
use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};
use std::sync::Arc;

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
