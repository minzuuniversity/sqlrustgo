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
# ADVISORY: a check that found something but is owner-adjudicated as
# non-blocking (SPEC-011 §5). Counted separately so the summary never
# claims "N/N passed" while a finding is outstanding.
ADVISORY=0
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
#
# 2026-11-23（GPT 评审 R1 · G1 行为级缺陷证明 · SPEC-011 符合性修复）
#
# 旧实现 grep 的是 `eng.execute(`。但 `eng.execute()` on an ExecutionEngine
# **正是 AD-002 允许的唯一 DML 入口** —— SPEC-011 §1.2 写得很清楚：
#   "✅ 允许: 所有 SQL 通过 ExecutionEngine::execute() 入口"
#   "❌ 禁止: 直接调 storage.insert/update/delete 绕过 ExecutionEngine"
# 而 SPEC-011 §3.3 给出的**验收口径**是：
#   "mysql-server 0 处 storage DML bypass
#    (grep -E 'storage\.(insert|update|delete|scan)' 0 matches)"
#
# 也就是说：规格要求的检测模式（storage DML bypass）从未被实现，
# 实现里放的是规格明确说**不要**检测的那个东西。后果是判据零鉴别力 ——
# 它在全部合规的树上报 INFO，在真有绕过的树上沉默。实测（权威树
# 8cbde5b6d3，2026-11-23）：旧判据在 mysql-server 报 "INFO (5 calls
# found)"，而这 5 处全部是合法入口（crates/mysql-server/src/lib.rs:6339
# / 6769 / 7729 / 7735 及一条注释）。
#
# 现在用 SPEC-011 §3.1 指定的模式实现，判定逻辑收敛到
# scripts/gate/architecture_freeze_bypass_detector.py（一处实现，可回归测试），
# 该实现额外处理两件 grep 做不到的事：
#   * 只看 **cargo 真正编译** 的源文件（crates/server/src/openclaw_endpoints.rs
#     未在任何 crate root 里声明为 module，cargo 从不编译它 —— 但另外三处
#     注释仍把它称作 "production site"）；
#   * 排除 `#[cfg(test)]` / `#[test]` 里的测试代码。
#
# SPEC-011 §5 风险表裁定：真发现 bypass 时保持 **INFO / 非阻断**。
# 因此这里计入 ADVISORY 而不是 PASS —— 门禁不再声称「5/5 全过」，
# 而是把发现如实列出（AFP：自报有 finding 却仍记作 PASS 是编造）。
BYPASS_DETECTOR="${PROJECT_ROOT}/scripts/gate/architecture_freeze_bypass_detector.py"
if [ ! -f "$BYPASS_DETECTOR" ]; then
    echo "SKIP (detector missing: ${BYPASS_DETECTOR}) — 检测无法完成 ⇒ INCOMPLETE"
    INCOMPLETE=$((INCOMPLETE+1))
else
    # Scope = the server-side crates, matching the pre-existing filter
    # (/mysql-server/, /network/, /server/). AD-002 constrains the *server*
    # crates; storage/executor/gmp/tools reach storage directly by design.
    SCOPE_CRATES=""
    for d in crates/*/src; do
        [ -d "$d" ] || continue
        case "$d" in
            */mysql-server/*|*/network/*|*/server/*)
                SCOPE_CRATES="$SCOPE_CRATES $(basename "$(dirname "$d")")"
                ;;
        esac
    done
    BYPASS_OUT=$(python3 "$BYPASS_DETECTOR" "$PROJECT_ROOT" $SCOPE_CRATES 2>&1)
    BYPASS_RC=$?
    if [ "$BYPASS_RC" -eq 2 ]; then
        echo "SKIP (detector could not complete) — 检测无法完成 ⇒ INCOMPLETE"
        echo "       $BYPASS_OUT"
        INCOMPLETE=$((INCOMPLETE+1))
    elif [ "$BYPASS_RC" -eq 0 ]; then
        echo "PASS (0 production storage DML bypass in server/network crates)"
        PASS=$((PASS+1))
    else
        BYPASS_N=$(printf '%s\n' "$BYPASS_OUT" | grep -c . )
        echo "INFO ($BYPASS_N storage DML bypass site(s) in server/network crates — AD-002 bypass; advisory per SPEC-011 §5, not blocking)"
        printf '%s\n' "$BYPASS_OUT" | sed 's/^/       /'
        ADVISORY=$((ADVISORY+1))
    fi
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
        # SPEC-011 §2.3 明确把 A7-3 列为 **out of scope**（留给 SPEC-012），
        # 所以这里不判 FAIL。但它**也不是 PASS** —— 旧实现两个分支都
        # `PASS=$((PASS+1))`，于是门禁在超限时仍然报 "PASS: N/N"，
        # 自报的内容与自己打印的 INFO 矛盾（AFP：自报有 finding 却记作
        # PASS）。现在计入 ADVISORY，汇总行如实显示。
        ADVISORY=$((ADVISORY+1))
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
if [ "$ADVISORY" -gt 0 ]; then
    # Never print a bare "N/N" when something was found but is not blocking.
    echo "ADVISORY: $ADVISORY item(s) found (non-blocking per SPEC-011 §5 — see the lines above)"
fi
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
    if [ "$ADVISORY" -gt 0 ]; then
        echo "✅ Architecture Freeze PASS (with $ADVISORY advisory finding(s) — not a clean 5/5; see above)"
    else
        echo "✅ Architecture Freeze PASS"
    fi
    exit 0
fi
echo "❌ Architecture Freeze FAILED ($PASS/$TOTAL 通过)"
echo "   有检查项未通过 ⇒ FAIL(1)；本门禁在 GA required 清单内，不得降级为 optional"
exit 1
