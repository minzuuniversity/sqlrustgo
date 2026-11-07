#!/bin/bash
# check_sem1_savepoint.sh - SEM-1 (#3172) Savepoint G5 gate
#
# Verifies:
# 1. SavepointOp enum is exported from sqlrustgo_parser
# 2. SavepointStatement AST variant exists in Statement enum
# 3. 3 parse_*_savepoint functions exist in parser.rs
# 4. 3 TransactionManager methods exist: savepoint, rollback_to_savepoint, release_savepoint
# 5. SAVEPOINT **statement path** complete (lexer → parser → savepoint_wiring → engine_dml)
# 6. ActiveTransaction has savepoint_manager field
# 7. sem1_savepoint_test **actually runs and passes** (target sem1_savepoint_test)
# 8. SavepointStatement dispatcher calls execute_savepoint (execution_engine_methods.rs)
#
# 2026-11-08（Owner 批准 GA-SEM1 判据修改）：第 5/7/8 项原判据都查**过期路径**，
# 三条恒红假红；现已改为查真实位置并**真跑**测试。反向变异证据见
# scripts/gate/test_sem1_statement_path.sh。
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

# 5. SAVEPOINT **statement path** (not a method name)
#    2026-11-08（Owner 批准 GA-SEM1 判据修改）：原判据是
#    `grep -q "fn execute_savepoint" src/execution_engine.rs` —— 那是**实现从未用过**的方法名
#    （SAVEPOINT 走的是 lexer → parser dispatcher → src/savepoint_wiring.rs → engine_dml 的 undo 接线），
#    所以该门禁在 develop 上恒红（假红，已由归档 run 20261107-ga-gates-001 实测）。
#    改判据为「语句路径存在」，并把它做成**可带 root 调用的函数** ⇒ 反向变异有载体
#    （夹具 scripts/gate/test_sem1_statement_path.sh 会造一个缺接线的假树 ⇒ 必须判红）。
check_savepoint_statement_path() {
  local root="${1:-$PROJECT_ROOT}"
  local missing=0
  local -a required=(
    "crates/parser/src/lexer.rs|Token::Savepoint|词法：SAVEPOINT 关键字"
    "crates/parser/src/parser.rs|Statement::SavepointStatement|语法：SAVEPOINT 语句变体"
    "src/savepoint_wiring.rs|pub fn record_insert_undo|引擎接线：插入 undo 记录"
    "src/savepoint_wiring.rs|pub fn record_delete_undo|引擎接线：删除 undo 记录"
    "src/savepoint_wiring.rs|pub fn record_update_undo|引擎接线：更新 undo 记录"
    "src/engine_dml.rs|record_insert_undo(|引擎使用点：插入撤销"
    "src/engine_dml.rs|record_delete_undo(|引擎使用点：删除撤销"
    "src/engine_dml.rs|record_update_undo(|引擎使用点：更新撤销"
  )
  local item file sym desc
  for item in "${required[@]}"; do
    file="${item%%|*}"; sym="${item#*|}"; sym="${sym%%|*}"; desc="${item##*|}"
    if ! grep -q -- "$sym" "$root/$file" 2>/dev/null; then
      echo "  ❌ FAIL: SAVEPOINT 语句路径缺 ${desc}（${file} 找不到 ${sym}）"
      missing=$((missing + 1))
    fi
  done
  return "$missing"
}
if ! check_savepoint_statement_path "$PROJECT_ROOT"; then
    echo "  ❌ FAIL: SAVEPOINT statement path incomplete (see missing items above)"
    exit 1
fi
echo "  [5/8] ✅ PASS: SAVEPOINT statement path complete (lexer → parser → savepoint_wiring → engine_dml)"

# 6. ActiveTransaction savepoint_manager field
if ! grep -q "savepoint_manager" crates/transaction/src/transaction_manager.rs; then
    echo "  ❌ FAIL: ActiveTransaction.savepoint_manager field missing"
    exit 1
fi
echo "  [6/8] ✅ PASS: ActiveTransaction has savepoint_manager field"

# 7. sem1_savepoint_test **真的存在并真的跑过**
#    2026-11-08（Owner 批准 GA-SEM1 判据修改）：原判据查的是 `tests/sem1_savepoint_test.rs`
#    —— 这个**路径早已不存在**（真实位置 tests/integration/transaction/sem1_savepoint_test.rs，
#    Cargo.toml 里的 target 名 = sem1_savepoint_test）；而且它**只查文件存在**，
#    却在文件头宣称「6 个 sem1_savepoint_test 通过」⇒ 判据远弱于它声称的口径。
#    现在改成：查真实路径 + **真的跑一遍**并报告实测通过数（不写死个数）。
SEM1_TEST="tests/integration/transaction/sem1_savepoint_test.rs"
if [ ! -f "$SEM1_TEST" ]; then
    echo "  ❌ FAIL: $SEM1_TEST not found"
    exit 1
fi
echo "  [7/8] ⏳ running: cargo test --test sem1_savepoint_test"
SEM1_LOG="$(mktemp "${TMPDIR:-/Volumes/workspace/dev}/sem1test.XXXXXX")"
cargo test --test sem1_savepoint_test > "$SEM1_LOG" 2>&1
SEM1_RC=$?
if [ "$SEM1_RC" -ne 0 ]; then
    echo "  ❌ FAIL: cargo test --test sem1_savepoint_test rc=$SEM1_RC"
    tail -20 "$SEM1_LOG"
    rm -f "$SEM1_LOG"
    exit 1
fi
SEM1_PASSED="$(grep -oE '^test result: ok\. [0-9]+ passed' "$SEM1_LOG" | grep -oE '[0-9]+ passed' | grep -oE '[0-9]+' | head -1)"
rm -f "$SEM1_LOG"
echo "  [7/8] ✅ PASS: sem1_savepoint_test 通过（实测 passed=${SEM1_PASSED:-?}）"

# 8. SavepointStatement 的**调用点**（dispatcher）
#    2026-11-08：原判据查 `src/execution_engine.rs` 里的 `Statement::SavepointStatement`
#    —— 那个文件里没有；真实 dispatcher 在 `src/execution_engine_methods.rs`：
#    `Statement::SavepointStatement { ref name, op } => self.execute_savepoint(name, op),`
#    ⇒ 判据改为查**真实调用点**（并要求它真的调用 execute_savepoint，不是只提到类型名）。
if ! grep -q "Statement::SavepointStatement" src/execution_engine_methods.rs; then
    echo "  ❌ FAIL: SavepointStatement not dispatched in src/execution_engine_methods.rs"
    exit 1
fi
if ! grep -q "execute_savepoint(name, op)" src/execution_engine_methods.rs; then
    echo "  ❌ FAIL: SavepointStatement arm does not call execute_savepoint"
    exit 1
fi
echo "  [8/8] ✅ PASS: SavepointStatement dispatched → execute_savepoint (execution_engine_methods.rs)"

echo
echo "=== G5 Gate: PASS ==="
echo "SEM-1 (#3172) Savepoint: AST + parser + TransactionManager + executor + tests verified"
exit 0
