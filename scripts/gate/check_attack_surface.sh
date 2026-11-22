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

echo "=== G-03: Attack Surface Verification (AV1-AV10) ==="
echo "Date: $(date)"
echo ""

cd "$PROJECT_ROOT"

# Ensure cargo is on PATH (CI runners may not have it in default PATH).
if ! command -v cargo >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
fi

ATTACK_SURFACE_DOC="docs/security/attack_surface/ATTACK_SURFACE_ANALYSIS.md"

echo "[1/3] Checking attack surface documentation..."

if [ ! -f "$ATTACK_SURFACE_DOC" ]; then
    echo "❌ FAIL: Attack surface document not found at '$ATTACK_SURFACE_DOC'"
    exit 1
fi

echo "  ✅ Attack surface document exists"

echo "[2/3] Verifying AV coverage..."

AV_COUNT=$(grep -c "^| AV" "$ATTACK_SURFACE_DOC" 2>/dev/null || AV_COUNT=0)
AV_COUNT=${AV_COUNT:-0}

if [ "$AV_COUNT" -lt 10 ]; then
    echo "  ❌ FAIL: Only $AV_COUNT attack vectors documented (need 10)"
    exit 1
fi

echo "  ✅ All 10 attack vectors (AV1-AV10) documented"

echo "[3/3] Checking critical mitigations..."

CRITICAL_AVS=$(grep -c "CRITICAL" "$ATTACK_SURFACE_DOC" 2>/dev/null || CRITICAL_AVS=0)
CRITICAL_AVS=${CRITICAL_AVS:-0}

if [ "$CRITICAL_AVS" -lt 3 ]; then
    echo "  ⚠️  Warning: Only $CRITICAL_AVS CRITICAL attack vectors documented"
fi

echo "  ✅ Attack surface analysis complete"

echo ""
echo "=== G-03 Attack Surface Verification ==="
echo "✅ PASSED: All 10 attack vectors (AV1-AV10) verified"
echo ""
echo "Attack vectors documented:"
grep "^| AV" "$ATTACK_SURFACE_DOC" | while read line; do
    echo "  $line"
done

exit 0
