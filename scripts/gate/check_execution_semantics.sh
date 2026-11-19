#!/usr/bin/env bash
# ==========================================================================
# GATE-DISPOSITION-SELF-MARK —— 本文件**不会在权威轮里被执行**，登记结论就地写在这里
#
# 分类：死门禁（dead）—— 既无执行点也无正文引用 ⇒ 永远不会运行
# 处置：待接入执行点（needs-execution-point）
# 责任任务：P0-4
# 登记说明：判据可在仓内自足执行，但不在任何阶段清单/CI 执行点 ⇒ P0-2 定义执行点后必须挂上，否则撤声称
# 效果声明：尚无可证伪夹具（NEG_FIXTURES 里本门禁没有可证伪行），因此不声称能抓任何东西 —— 无声明的「能抓」比错的「能抓」更坏。
#
# 为什么保留文件而不是删掉：删文件会改变门禁总数（当前 168，固定不可动），
# 而「不执行」是**登记在册的已知状态**，不是遗漏。删除会让「这一项为什么
# 没有判定」变成不可追溯。因此：**保留文件 + 就地自标 + 登记在册**。
#
# 权威登记：docs/releases/v4.1.0/GATE_CATALOG.jsonl（同一 gate 键）
# ==========================================================================
# scripts/gate/check_execution_semantics.sh
# Verify no execution path bypasses the canonical ExecutionEngine contract.
# See docs/governance/EXECUTION_SEMANTICS.md (SEM-1).
#
# Exit 0 if all checks pass, non-zero if any violation found.

set -eo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$REPO_ROOT"

# Ensure cargo is on PATH (CI runners may not have it in default PATH).
if ! command -v cargo >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
fi

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
