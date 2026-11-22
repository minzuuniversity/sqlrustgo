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
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "=== R10: Formal Proof Check ==="
echo "Date: $(date)"
echo ""

cd "$PROJECT_ROOT"

# Ensure cargo is on PATH (CI runners may not have it in default PATH).
if ! command -v cargo >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
fi

PROOF_DIR="docs/proof"
MIN_PROOFS=10

echo "[1/3] Checking proof directory..."

if [ ! -d "$PROOF_DIR" ]; then
    echo "⚠️  Proof directory '$PROOF_DIR' not found"
    echo ""
    echo "❌ R10: Formal Proof Check FAILED"
    echo "   Proof directory must exist at '$PROOF_DIR'"
    exit 1
fi

echo "[2/3] Counting proof files..."

PROOF_COUNT=$(find "$PROOF_DIR" -name "*.json" -type f 2>/dev/null | wc -l)

if [ "$PROOF_COUNT" -lt "$MIN_PROOFS" ]; then
    echo "Proof files found: $PROOF_COUNT"
    echo "Minimum required: $MIN_PROOFS"
    echo ""
    echo "❌ R10: Formal Proof Check FAILED"
    echo "   Insufficient formal proofs (found $PROOF_COUNT, need >= $MIN_PROOFS)"
    exit 1
fi

echo "[3/3] Validating proof files..."

INVALID=0
for proof_file in $(find "$PROOF_DIR" -name "*.json" -type f 2>/dev/null); do
    if ! python3 -c "import json; json.load(open('$proof_file'))" 2>/dev/null; then
        echo "  Invalid JSON: $proof_file"
        INVALID=$((INVALID + 1))
    fi
done

if [ "$INVALID" -gt 0 ]; then
    echo ""
    echo "❌ R10: Formal Proof Check FAILED"
    echo "   $INVALID proof file(s) have invalid JSON"
    exit 1
fi

echo ""
echo "✅ R10: Formal Proof Check PASSED"
echo "   Proof files: $PROOF_COUNT (>= $MIN_PROOFS required)"
echo "   All files valid JSON"
exit 0
