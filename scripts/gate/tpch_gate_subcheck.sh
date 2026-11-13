#!/usr/bin/env bash
# tpch_gate_subcheck.sh — 各阶段门禁共用的 TPC-H 子检查（一份实现，七个调用方）。
#
# WHY（2026-11-13，见 PR #5772 报告 §8 与 PR #5773）：
#   7 道门禁各自内嵌同一段「TPC-H 22/22 maintained」子检查：
#     TPCH_PASSED=$(cargo test --test tpch_gate_test 2>&1 | grep -E "test result.*ok" | head -1)
#   在没有 TPC-H 夹具的宿主上，tpch_gate_test 自报
#     TPCH-NOT-RUN: tpch_gate_test reason=fixture_missing ...
#   后仍以 `test result: ok` 返回（0.03s，一条查询没跑）⇒ grep 只看 "ok"
#   ⇒ 七道门禁全部报 "[N/N] ✅ PASS: TPC-H gate (22/22) maintained"。
#   实测成员：check_p35_simd.sh（动态证实）、g12/g16/p22/p32/p33/p34（同一字节模板）。
#
# 本脚本把「能不能比、比没比过」分成三态，调用方按退出码诚实呈现：
#   0 = PASS       —— tpch_gate_test 退出 0 且没有 NOT-RUN 自报
#   2 = INCOMPLETE —— 没有夹具（fixture_missing）或测试自报 NOT-RUN；
#                     这不是漂移，调用方必须说 INCOMPLETE，不许说 PASS 也不许说 mismatch
#   1 = FAIL       —— tpch_gate_test 真红
#
# 用法：
#   bash scripts/gate/tpch_gate_subcheck.sh        # 打印一行 TPCH-SUBCHECK: <verdict> ...
#
# Environment:
#   TPCH_DATA_DIR  TPC-H .tbl 目录（缺省 ~/sqlrustgo-tpch/data；缺 8 个 .tbl 任一 ⇒ INCOMPLETE）
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

FIXTURE_DIR="${TPCH_DATA_DIR:-${HOME}/sqlrustgo-tpch/data}"
MISSING_COUNT=0
for tbl in customer lineitem nation orders part partsupp region supplier; do
    if [[ ! -f "${FIXTURE_DIR}/${tbl}.tbl" ]]; then
        MISSING_COUNT=$((MISSING_COUNT + 1))
    fi
done

if [[ "${MISSING_COUNT}" -gt 0 ]]; then
    echo "TPCH-SUBCHECK: INCOMPLETE reason=fixture_missing detail=missing ${MISSING_COUNT}/8 .tbl at ${FIXTURE_DIR}"
    exit 2
fi

OUT="$(cargo test --test tpch_gate_test 2>&1)"
RC=$?

if [[ "${RC}" -ne 0 ]]; then
    echo "TPCH-SUBCHECK: FAIL detail=tpch_gate_test exit ${RC}"
    echo "${OUT}" | tail -5
    exit 1
fi

# 夹具在但测试仍自报 NOT-RUN（例如 profile_mismatch）——同样不许当 PASS。
if echo "${OUT}" | grep -q "TPCH-NOT-RUN"; then
    REASON="$(echo "${OUT}" | grep -oE "reason=[a-z_]+" | head -1 | cut -d= -f2)"
    echo "TPCH-SUBCHECK: INCOMPLETE reason=${REASON:-undisclosed} detail=test disclosed NOT-RUN"
    exit 2
fi

echo "TPCH-SUBCHECK: PASS"
exit 0
