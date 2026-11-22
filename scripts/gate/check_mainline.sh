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
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$REPO_ROOT"

# Ensure cargo is on PATH (CI runners may not have it in default PATH).
if ! command -v cargo >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
fi

echo "=== Running Mainline Path Check ==="
echo "Verifies DML operations go through Transaction + WAL"
echo ""

ERRORS=0

echo "[1/3] Checking executor/src/ for DML without txn/wal..."

# Find direct storage mutation calls in executor that don't go through txn/wal
while IFS= read -r file; do
    if grep -q "storage\.\(insert\|update\|delete\)" "$file" 2>/dev/null; then
        if ! grep -L "txn\|transaction\|wal" "$file" > /dev/null 2>&1; then
            echo "❌ F1: $file - DML without txn/wal"
            ERRORS=$((ERRORS + 1))
        fi
    fi
done < <(find crates/executor/src -name "*.rs" -type f)

echo "[2/3] Checking isolated modules not used in mainline..."

# Check that isolated modules aren't imported in production paths
ISOLATED=("parallel_executor" "vec_simd" "expr-legacy" "local_executor_dml")
for module in "${ISOLATED[@]}"; do
    if grep -r "use sqlrustgo_$module\|mod $module" crates/*/src/*.rs 2>/dev/null | grep -v "test\|tests" > /dev/null; then
        echo "❌ F2: Isolated module '$module' used in production"
        ERRORS=$((ERRORS + 1))
    fi
done

echo "[3/3] Checking WAL integration in executors..."

# Check that main executors have WAL
MAIN_EXECUTORS=(
    "crates/executor/src/transactional_executor.rs"
    "crates/executor/src/executor.rs"
)
for exec in "${MAIN_EXECUTORS[@]}"; do
    if [ -f "$exec" ]; then
        if ! grep -q "wal\|Wal\|WalStorage" "$exec" 2>/dev/null; then
            echo "❌ R1: $exec - missing WAL integration"
            ERRORS=$((ERRORS + 1))
        fi
    fi
done

echo ""
if [ $ERRORS -eq 0 ]; then
    echo "✅ Mainline path check passed!"
    echo "All DML goes through Transaction + WAL"
    exit 0
else
    echo "❌ Found $ERRORS mainline violation(s)"
    exit 1
fi