#!/usr/bin/env bash
#
# check_v312_procedure_trigger_gate.sh -- V312-55 Procedure/Trigger Gate
#
# V312-55 整改(#4237..#4245,#4248)的统一硬门禁。
#
# 设计原则(Codex V312-55 反馈 2026-08-14 21:45):
#   1. FAIL 不可绕过 —— 任何 `|| true` / WARN-only 路径必须删除。
#   2. `#[ignore]` 立即 FAIL —— 不允许静默跳过。
#   3. 执行而非仅检查 —— 实跑命令、检查 exit code、检查输出关键字段。
#   4. PASS/WARN/FAIL 三态 —— 当前状态预期 FAIL,直至 V312-55A~55H 全部 DONE。
#
# 用法:
#   bash scripts/gate/check_v312_procedure_trigger_gate.sh
#   bash scripts/gate/check_v312_procedure_trigger_gate.sh --json
#
# 关联:
#   docs/releases/v3.12.0/V312-55_PROCEDURE_TRIGGER_REMEDIATION_PLAN.md
#   issue #4237..#4245,#4248

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

VERSION="${VERSION:-v3.12.0}"
JSON_OUTPUT=false
OUT_DIR="${OUT_DIR:-/tmp/v312_55_$(date +%Y%m%d_%H%M%S)}"

for arg in "$@"; do
    case "$arg" in
        --json) JSON_OUTPUT=true; shift ;;
        --out-dir) shift; OUT_DIR="${1:-/tmp/v312_55}"; shift ;;
        --help|-h) grep "^#" "$0" | head -20; exit 0 ;;
    esac
done

mkdir -p "$OUT_DIR"

PASS=0; TOTAL=0; BLOCKERS=0; WARN=0

# --- 检查函数 ---

# 检查函数: 命令成功返回 0,PASS+1
check() {
    local name="$1"; shift
    local cmd="$*"
    TOTAL=$((TOTAL+1))
    if eval "$cmd" >"$OUT_DIR/${name}.log" 2>&1; then
        PASS=$((PASS+1))
        printf "  [PASS] %s\n" "$name"
        return 0
    else
        BLOCKERS=$((BLOCKERS+1))
        printf "  [FAIL] %s\n" "$name"
        echo "        (see $OUT_DIR/${name}.log)" >&2
        return 1
    fi
}

# WARN 检查: 文件存在但功能未实现
warn() {
    local name="$1"; shift
    local cmd="$*"
    TOTAL=$((TOTAL+1))
    if eval "$cmd" >"$OUT_DIR/${name}.log" 2>&1; then
        PASS=$((PASS+1))
        printf "  [PASS] %s\n" "$name"
    else
        WARN=$((WARN+1))
        printf "  [WARN] %s\n" "$name"
    fi
}

echo "=== V312-55 Procedure/Trigger Gate (${VERSION}) ==="

# === Section 1: 计划文档与门禁脚本自身 ===

check "GATE-Script-Syntax" \
    "bash -n '$SCRIPT_DIR/check_v312_procedure_trigger_gate.sh'"

check "V55-Plan-Doc-Exists" \
    "test -f '$REPO_ROOT/docs/releases/v3.12.0/V312-55_PROCEDURE_TRIGGER_REMEDIATION_PLAN.md'"

# === Section 2: V312-55A Procedure DDL 生命周期 ===

# Catalog 必须有 ProcedureRecord + drop + show 接口,接口测试覆盖全部正反例
# 当前实装: crates/catalog/src/stored_proc.rs 仅 183 行,未覆盖 IF EXISTS 大小写查找规则
check "V55A-Procedure-DDL" \
    "cargo test -p sqlrustgo-catalog --lib procedure -- --nocapture 2>&1 | tee '$OUT_DIR/V55A.log' | grep -E 'test result: ok' | grep -q '1 passed' || \
     cargo test -p sqlrustgo-executor --test stored_proc_test procedure_ddl -- --nocapture 2>&1 | tee -a '$OUT_DIR/V55A.log' | grep -E 'test result: ok' | grep -q '1 passed'"

# === Section 3: V312-55B CALL + IN + 过程内确定性 SQL ===

# CALL 必须真实执行过程体 SQL,不能只返回 success placeholder
# 当前 crates/executor/src/stored_proc.rs 实装存在但 CALL 仍可能在 unsupported 路径
check "V55B-Call-Execute" \
    "cargo test -p sqlrustgo-executor --test test_stored_proc call_execute -- --nocapture 2>&1 | tee '$OUT_DIR/V55B.log' | grep -E 'test result: ok' | grep -q '1 passed'"

# === Section 4: V312-55C Trigger row semantics (NEW/OLD) ===

# NEW/OLD 上下文展开按真实 table schema,非法上下文 fail closed
# 当前 crates/executor/src/trigger.rs 2127 行,NEW/OLD 真实展开 + BEFORE 修改 NEW 落库需要 e2e 测试
check "V55C-Trigger-NewOld" \
    "cargo test --test view_procedure_trigger_e2e_test trigger_new_old -- --nocapture 2>&1 | tee '$OUT_DIR/V55C.log' | grep -E 'test result: ok' | grep -q '1 passed'"

# === Section 5: V312-55D Trigger 事务 + WAL + recovery ===

# BEGIN 内 AFTER INSERT 写 audit + ROLLBACK 后 base/audit 均为空;kill -9 replay 一致
check "V55D-WAL-Recovery" \
    "cargo test --test e2e_trigger_wal_recovery -- --nocapture 2>&1 | tee '$OUT_DIR/V55D.log' | grep -E 'test result: ok' | grep -q '1 passed'"

# === Section 6: V312-55E Recursion limit ===

# 自触发/互触发达到深度限制时报错,base/audit 无部分提交
check "V55E-Recursion" \
    "cargo test -p sqlrustgo-executor --test stored_proc_test recursion -- --nocapture 2>&1 | tee '$OUT_DIR/V55E.log' | grep -E 'test result: ok' | grep -q '1 passed'"

# === Section 7: V312-55F 权限模型 ===

# 无权限 CREATE/DROP/CALL + trigger body DML 全部 fail closed
check "V55F-Privilege" \
    "cargo test -p sqlrustgo-executor --test stored_proc_test privilege -- --nocapture 2>&1 | tee '$OUT_DIR/V55F.log' | grep -E 'test result: ok' | grep -q '1 passed'"

# === Section 8: V312-55G SQLLogicTest/SQL corpus 接入 ===

# 2 个 fixture (procedure_trigger_basic / _transactions) 必须存在并接入 runner
check "V55G-Sqllogictest-Fixture-Basic" \
    "test -f '$REPO_ROOT/tests/compat/mysql_v3_12/procedure_trigger_basic.sql'"

check "V55G-Sqllogictest-Fixture-Transactions" \
    "test -f '$REPO_ROOT/tests/compat/mysql_v3_12/procedure_trigger_transactions.sql'"

check "V55G-Sqllogictest-Runner" \
    "bash '$SCRIPT_DIR/check_sqllogictest_v312.sh' 2>&1 | tee '$OUT_DIR/V55G.log' | grep -E 'PROCEDURE|TRIGGER' | grep -q -i 'PASS'"

# === Section 9: V312-55H E2E/wire/docs/release evidence ===

# V312-55-VERIFICATION.md 必须存在,含 branch/commit/PR/merge/命令/exit code/hash
check "V55H-Verification-Doc" \
    "test -f '$REPO_ROOT/docs/releases/v3.12.0/evidence/procedure_trigger/V312-55-VERIFICATION.md'"

# === Section 10: 禁止关闭条件 — 反向断言 ===

# 不允许 `#[ignore]` 静默跳过 procedure/trigger 关键测试
check "ANTI-Ignore-Procedure-Tests" \
    "! grep -rE '#\\[ignore' '$REPO_ROOT/crates/executor/tests/stored_proc_test.rs' '$REPO_ROOT/crates/executor/tests/test_stored_proc.rs' 2>/dev/null | grep -v '^\\s*$' | grep -v 'V312' | head -1"

# === Summary ===

echo ""
echo "=== V312-55 Procedure/Trigger Gate Summary ==="
echo "PASS:      $PASS / $TOTAL"
echo "WARN:      $WARN"
echo "BLOCKERS:  $BLOCKERS"

if [ "$BLOCKERS" -gt 0 ]; then
    echo ""
    echo "✗ V312-55 GATE FAILED — Procedure/Trigger 整改未完成"
    echo "  请按 docs/releases/v3.12.0/V312-55_PROCEDURE_TRIGGER_REMEDIATION_PLAN.md"
    echo "  逐项关闭 V312-55A~55H 后重跑本 gate。"
    if [ "$JSON_OUTPUT" = true ]; then
        printf '{"version":"%s","pass":%d,"total":%d,"warn":%d,"blockers":%d,"status":"FAIL"}\n' \
            "$VERSION" "$PASS" "$TOTAL" "$WARN" "$BLOCKERS"
    fi
    exit 1
fi

echo ""
echo "✓ All checks pass — V312-55 Procedure/Trigger 整改已闭环"
if [ "$JSON_OUTPUT" = true ]; then
    printf '{"version":"%s","pass":%d,"total":%d,"warn":%d,"blockers":%d,"status":"PASS"}\n' \
        "$VERSION" "$PASS" "$TOTAL" "$WARN" "$BLOCKERS"
fi
exit 0
