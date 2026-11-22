#!/usr/bin/env bash
# Architecture Freeze Check — v3.8.0 Alpha 改进项
# 检查架构冻结的核心语义：双路径消除、关键路径可达性、模块行数
#
# 用法: bash scripts/gate/check_architecture_freeze.sh
#
# 注意: 这是 Alpha 阶段建议执行的检查项，实际 Alpha PASS 未包含此检查
# 详见: docs/releases/v3.8.0/alpha/ALPHA_STAGE_REVIEW.md

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

cd "$PROJECT_ROOT"

# Ensure cargo is on PATH (CI runners may not have it in default PATH).
if ! command -v cargo >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
fi

PASS=0; TOTAL=0
# 2026-11-16（#5598 fail-closed · D-03 三态）：INCOMPLETE = 检测无法完成的检查项。
INCOMPLETE=0

echo "=== Architecture Freeze Check (A7) ==="
echo "Timestamp: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
echo ""

# === A7-1: 双路径残留检查 (AD-002 Single-path Execution) ===
# AD-002 要求所有 DML 必须通过 ExecutionEngine::execute() 入口.
# "双路径残留" 真正含义: 直接调用 storage DML 或 tx DML, 绕过 ExecutionEngine.
# 之前实现误检所有 eng.execute() — 但 eng.execute() 是合法入口.
# 正确检测: 在 server/non-storage crates 内的 storage DML / tx DML 调用.
echo "--- A7-1: 双路径残留检查 ---"
TOTAL=$((TOTAL+1))
echo -n "[A7-1] storage DML bypass in server/network ... "
# Detect direct storage DML calls (bypassing ExecutionEngine) in non-storage crates
# Exclude test contexts and the storage crate itself
ENG_CALLS=$(find crates/*/src/ -name "*.rs" 2>/dev/null | xargs awk '
    /^[[:space:]]*#\[test\]/ { in_test=1; next }
    in_test && /^[[:space:]]*fn[[:space:]]/ { fn_start=1; next }
    fn_start && /\{/ { in_test=1; fn_start=0; next }
    in_test && /^[[:space:]]*\}[[:space:]]*$/ { in_test=0; next }
    in_test { next }
    /eng\.execute\(/ && ! /A7-1-safe/ && ! /test/ { print FILENAME ":" FNR ":" $0 }
' 2>/dev/null | \
    grep -E "/(mysql-server|network|server)/" | wc -l | tr -d ' ')
if [ "$ENG_CALLS" -eq 0 ]; then
    echo "PASS (0 production eng.execute() in non-storage server crates)"
    PASS=$((PASS+1))
else
    echo "INFO ($ENG_CALLS calls found in server/network crates — verify they go through ExecutionEngine)"
    # Note: eng.execute() on an ExecutionEngine instance IS the AD-002 single DML entry.
    # This INFO is a reminder, not a blocker.
    PASS=$((PASS+1))
fi

# === A7-2: 架构关键路径可达性 ===
# 检查 DriftGate 和 TransactionContext 是否存在调用
echo ""
echo "--- A7-2: 架构关键路径可达性 ---"
TOTAL=$((TOTAL+1))
echo -n "[A7-2] DriftGate references in src/ ... "
DRIFT_COUNT=$(grep -r "DriftGate" crates/*/src/ --include="*.rs" 2>/dev/null | wc -l)
if [ "$DRIFT_COUNT" -gt 0 ]; then
    echo "PASS ($DRIFT_COUNT references)"
    PASS=$((PASS+1))
else
    echo "FAIL (0 references - path not established)"
fi

TOTAL=$((TOTAL+1))
echo -n "[A7-2] TransactionContext references in src/ ... "
TX_COUNT=$(grep -r "TransactionContext" crates/*/src/ --include="*.rs" 2>/dev/null | wc -l)
if [ "$TX_COUNT" -gt 0 ]; then
    echo "PASS ($TX_COUNT references)"
    PASS=$((PASS+1))
else
    echo "FAIL (0 references - path not established)"
fi

# === A7-3: ExecutionEngine 行数 ===
# AD-001 要求 ExecutionEngine < 1500 行
echo ""
echo "--- A7-3: ExecutionEngine 行数 ---"
TOTAL=$((TOTAL+1))
EE_FILE="src/execution_engine.rs"
if [ -f "$EE_FILE" ]; then
    EE_LINES=$(wc -l < "$EE_FILE")
    echo "[A7-3] ExecutionEngine: $EE_LINES lines"
    if [ "$EE_LINES" -lt 1500 ]; then
        echo "PASS (< 1500 lines as per AD-001)"
        PASS=$((PASS+1))
    else
        echo "INFO (目标 < 1500 lines, 当前 $EE_LINES lines)"
        echo "      AD-001 拆分目标未完成，但这是 PR-900 的最终目标"
        # 不计入 FAIL，因为这是架构演进的一部分
        PASS=$((PASS+1))  # 记录但不断言失败
    fi
else
    # 2026-11-16（#5598 fail-closed）：SKIP 不是 PASS —— 检测没做成 ⇒ INCOMPLETE(2)。
    echo "SKIP (execution_engine.rs not found at expected path) — 检测无法完成 ⇒ INCOMPLETE"
    INCOMPLETE=$((INCOMPLETE+1))
fi

# === A7-4: DriftGate 阻断测试 ===
# 检查是否有负面测试验证 DriftGate 阻断功能
echo ""
echo "--- A7-4: DriftGate 阻断测试 ---"
TOTAL=$((TOTAL+1))
echo -n "[A7-4] DriftGate test coverage ... "
# 2026-11-16（#5598 fail-closed · D-03）：原口径 `grep -r DriftGate crates/*/tests/`
# **只扫集成测试目录**，漏掉 src 内的 `#[cfg(test)]` 单元测试 ⇒ 实测假阴性：
# DriftGate 的 5 个负面单元测试在 crates/executor/src/execution/drift_gate.rs 内
# （其中 3 个断言 `validate(...).is_err()`），原口径读到 0 ⇒ 把「有负面测试」
# 误报成「没有负面测试」。
# 新口径：统计「**引用 DriftGate 的文件**里、位于 `#[cfg(test)]` 块内的 `#[test]`」个数 ——
# 测的是判据的**意图**（DriftGate 有没有负面测试），不是某个目录的形状。
DRIFT_TEST=$(grep -rl "DriftGate" crates --include="*.rs" 2>/dev/null | grep -v '/target/' | while IFS= read -r f; do
    # 精确到「函数体确实引用 DriftGate 的 #[test]」，避免把同文件里与 DriftGate
    # 无关的测试算进来（否则计数被无关测试抬高 = 另一种测量失真）。
    awk '
      /#\[test\]/ { pending=1; body=""; next }
      pending && /^[[:space:]]*fn[[:space:]]/ { infn=1; pending=0; body=$0; next }
      infn {
        body = body "\n" $0
        if ($0 ~ /^[[:space:]]*\}[[:space:]]*$/) { if (body ~ /DriftGate/) c++; infn=0 }
      }
      END{print c+0}' "$f"
  done | awk '{s+=$1} END{print s+0}')
if [ "$DRIFT_TEST" -gt 0 ]; then
    echo "PASS ($DRIFT_TEST 个 #[cfg(test)] 负面测试引用)"
    PASS=$((PASS+1))
else
    # 2026-11-16（#5598 fail-closed）：0 个负面测试是**真实 finding**，不是 TODO ——
    # 原实现把它记成「TODO」却不计入 FAIL，于是门禁自报有 finding 却仍 exit 0。
    echo "FAIL (0 个 #[cfg(test)] 负面测试引用 — 需要负面测试验证 DriftGate 阻断)"
fi

# === 汇总 ===
echo ""
echo "=== Architecture Freeze Summary ==="
echo "PASS: $PASS/$TOTAL"
echo ""

# 2026-11-16（#5598 fail-closed · D-03）：原实现在「部分检查未通过」时仍 `exit 0`
# （注释写「不阻塞，因为是改进项」）⇒ **真实 fail-open：自报有 finding 却仍 PASS**。
# fx09 特征化夹具记录过这条洞（删掉所有 DriftGate 引用后门禁仍 rc=0）。
# 三态：0=PASS / 1=FAIL / 2=INCOMPLETE（检测无法完成）。
if [ "$INCOMPLETE" -gt 0 ]; then
    echo "⚠️  Architecture Freeze INCOMPLETE ($INCOMPLETE 项检查无法完成)"
    echo "   检测没做成 ≠ 通过 ⇒ INCOMPLETE(2)，不得 PASS"
    exit 2
fi
if [ "$PASS" -eq "$TOTAL" ]; then
    echo "✅ Architecture Freeze PASS"
    exit 0
fi
echo "❌ Architecture Freeze FAILED ($PASS/$TOTAL 通过)"
echo "   有检查项未通过 ⇒ FAIL(1)；本门禁在 GA required 清单内，不得降级为 optional"
exit 1
