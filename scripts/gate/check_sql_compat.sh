#!/usr/bin/env bash
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
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "=== R8: SQL Compatibility Check ==="
echo "Date: $(date)"
echo ""

cd "$PROJECT_ROOT"

# Ensure cargo is on PATH (CI runners may not have it in default PATH).
if ! command -v cargo >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
fi

CORPUS_THRESHOLD=80

echo "[1/2] Running SQL Corpus tests..."
# V312-30: removed `|| true` mask; propagate failure explicitly. P16 step
# 2.5 detects this anti-fab pattern; fail-explicit per V312-24 acceptance
# criteria ("known broken test binaries 不得继续靠 WARN-only 掩盖").
CORPUS_OUTPUT=$(cargo test -p sqlrustgo-sql-corpus -- --nocapture 2>&1)
CORPUS_EXIT=$?
if [ ${CORPUS_EXIT} -ne 0 ]; then
    echo "  FAIL: cargo test exit ${CORPUS_EXIT} (corpus test failed to build/run)"
    echo "  Output tail:"
    echo "${CORPUS_OUTPUT}" | tail -20 | sed 's/^/    /'
    exit 1
fi

echo "[2/2] Analyzing results..."

if echo "$CORPUS_OUTPUT" | grep -q "test result: ok"; then
    echo "SQL Corpus tests: PASS"
    echo ""
    echo "✅ R8: SQL Compatibility Check PASSED"
    echo "   Corpus pass rate >= $CORPUS_THRESHOLD%"
    exit 0
else
    echo "SQL Corpus tests: FAIL"
    echo ""
    echo "❌ R8: SQL Compatibility Check FAILED"
    echo "   SQL Corpus pass rate < $CORPUS_THRESHOLD%"
    exit 1
fi
