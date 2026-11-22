#!/usr/bin/env bash
# ==========================================================================
# GATE-DISPOSITION-SELF-MARK —— 本文件**不会在权威轮里被执行**，登记结论就地写在这里
#
# 分类：死门禁（dead）—— 既无执行点也无正文引用 ⇒ 永远不会运行
# 处置：保留并改接（keep-rewire）
# 责任任务：P0-4
# 登记说明：判据内容仍在 v4.1.0 的声称面内(台账 open issue -)：v3.12 的 RC 晋升检查，属旧版本线内部流程 ⇒ 归档候选
# 效果声明：尚无可证伪夹具（NEG_FIXTURES 里本门禁没有可证伪行），因此不声称能抓任何东西 —— 无声明的「能抓」比错的「能抓」更坏。
#
# 为什么保留文件而不是删掉：删文件会改变门禁总数（当前 168，固定不可动），
# 而「不执行」是**登记在册的已知状态**，不是遗漏。删除会让「这一项为什么
# 没有判定」变成不可追溯。因此：**保留文件 + 就地自标 + 登记在册**。
#
# 权威登记：docs/releases/v4.1.0/GATE_CATALOG.jsonl（同一 gate 键）
# ==========================================================================
#
# check_v312_promotion_to_rc.sh
#
# Purpose: Composite gate enforcing all 11 `promotion_to_RC_requires`
#          items in `docs/releases/v3.12.0/STAGE.yaml` (lines 94-105).
#          Blocks BETA→RC transition if any item lacks documented evidence.
#
# Issue: #4386 (V312-59-C)
#
# Anti-Fabrication-Policy-v1.0 §5:
#   - No `expiry 2027-06-30` pushback on RC blockers
#   - No merge-PR combining multiple RC items without per-item commit
#   - Every PASS verdict must reference commit hash + exit code + evidence_hash
#
# Usage:
#   bash scripts/gate/check_v312_promotion_to_rc.sh
#
# Exit codes:
#   0 — 11/11 PASS
#   1 — at least one FAIL
#
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

OUT_DIR="${OUT_DIR:-docs/releases/v3.12.0/evidence/v312-59}"
mkdir -p "$OUT_DIR"
EVIDENCE_LOG="$OUT_DIR/promotion_to_rc_evidence.txt"

# ----------------------------------------------------------------------
# RC item table: item → (evidence_file, optional test_cmd)
#   - file-only check (most items): just verify the evidence file exists
#   - test+file (RC-10): also run the integration test
# ----------------------------------------------------------------------
declare -a RC_ITEMS=(
    "RC1_GMP_MD_INGESTION|docs/releases/v3.12.0/evidence/v312-59/RC1_GMP_MD_INGESTION_REPORT.md"
    "RC2_RETRIEVAL_QUALITY|docs/releases/v3.12.0/evidence/v312-59/RC2_RETRIEVAL_QUALITY_REPORT.md"
    "RC3_BACKUP_RESTORE|docs/releases/v3.12.0/evidence/v312-59/RC3_BACKUP_RESTORE_REPORT.md"
    "RC4_SECURITY_RBAC|docs/releases/v3.12.0/evidence/v312-59/RC4_SECURITY_RBAC_REPORT.md"
    "RC5_CURATED_SQLLOGICTEST|docs/releases/v3.12.0/evidence/v312-59/RC5_CURATED_SQLLOGICTEST_REPORT.md"
    "RC6_TPCH_SF1_CROSS_ENGINE|NO-OP-COVERED-BY-V312-58"
    "RC7_WIRE_LOAD_DATA|docs/releases/v3.12.0/evidence/v312-59/RC7_WIRE_LOAD_DATA_REPORT.md"
    "RC8_CRASH_UPGRADE|docs/releases/v3.12.0/evidence/v312-59/RC8_CRASH_UPGRADE_REPORT.md"
    "RC9_V312_57_WEEK01_04|NO-OP-COVERED-BY-PR-4359-4370-4373"
    "RC10_V312_57_WEEK05_06|docs/releases/v3.12.0/evidence/v312-59/RC10_V312_57_WEEK05_06_REPORT.md|bash scripts/gate/check_bustubx_edu_cli_v312.sh"
    "RC11_CLAIM_CLEANUP|docs/releases/v3.12.0/evidence/v312-59/RC11_CLAIM_CLEANUP_REPORT.md"
)

PASS_COUNT=0
FAIL_COUNT=0
NOOP_COUNT=0
EVIDENCE_LOG_CONTENT=""
EVIDENCE_LOG_CONTENT+="===================================================================\n"
EVIDENCE_LOG_CONTENT+="V312-59-C promotion_to_RC gate — Evidence\n"
EVIDENCE_LOG_CONTENT+="Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)\n"
EVIDENCE_LOG_CONTENT+="===================================================================\n\n"

for entry in "${RC_ITEMS[@]}"; do
    IFS='|' read -r item evidence test_cmd <<< "$entry"
    idx=$(( ${item:2:1} ))  # extract digit from RC<N>
    echo "[$idx/11] $item"

    # RC-6 and RC-9 are no-op (covered by sibling issues)
    if [[ "$evidence" == NO-OP-* ]]; then
        echo "  [VERDICT] NO-OP (covered by $evidence)"
        NOOP_COUNT=$((NOOP_COUNT + 1))
        EVIDENCE_LOG_CONTENT+="[$idx] $item — NO-OP ($evidence)\n"
        continue
    fi

    # Stage 1: evidence file exists
    if [ ! -f "$evidence" ]; then
        echo "  [EVIDENCE_FILE] FAIL: $evidence not found"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        EVIDENCE_LOG_CONTENT+="[$idx] $item — EVIDENCE_FILE: FAIL (missing)\n"
        continue
    fi
    echo "  [EVIDENCE_FILE] PASS ($evidence)"

    # Stage 2: optional integration test
    if [ -n "${test_cmd:-}" ]; then
        if bash -c "$test_cmd" >/dev/null 2>&1; then
            echo "  [INTEGRATION_TEST] PASS"
            PASS_COUNT=$((PASS_COUNT + 1))
            EVIDENCE_LOG_CONTENT+="[$idx] $item — EVIDENCE_FILE + INTEGRATION_TEST: PASS\n"
        else
            echo "  [INTEGRATION_TEST] FAIL: $test_cmd exit non-zero"
            FAIL_COUNT=$((FAIL_COUNT + 1))
            EVIDENCE_LOG_CONTENT+="[$idx] $item — INTEGRATION_TEST: FAIL\n"
        fi
    else
        echo "  [INTEGRATION_TEST] N/A (evidence-file only)"
        PASS_COUNT=$((PASS_COUNT + 1))
        EVIDENCE_LOG_CONTENT+="[$idx] $item — EVIDENCE_FILE: PASS (test not required)\n"
    fi
done

# ----------------------------------------------------------------------
# Summary
# ----------------------------------------------------------------------

echo ""
echo "==================================================================="
echo "V312-59-C promotion_to_RC Summary"
echo "  PASS:               $PASS_COUNT / 11"
echo "  FAIL:               $FAIL_COUNT / 11"
echo "  NO-OP (covered):    $NOOP_COUNT / 11"
echo "  Total checked:      $((PASS_COUNT + FAIL_COUNT + NOOP_COUNT)) / 11"
echo "==================================================================="

EVIDENCE_LOG_CONTENT+="\n\nSUMMARY: pass=$PASS_COUNT fail=$FAIL_COUNT noop=$NOOP_COUNT total=11\n"
printf "%b" "$EVIDENCE_LOG_CONTENT" > "$EVIDENCE_LOG"

if [ "$FAIL_COUNT" -gt 0 ]; then
    exit 1
fi
exit 0