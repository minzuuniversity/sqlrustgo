#!/usr/bin/env bash
# Gate G1: TPC-H 22/22 baseline.
#
# Verifies that TPC-H Q1..Q22 all PASS and the combined deterministic
# result hash matches the v3.8.0 baseline. See:
#   - openspec/changes/g1-tpch-baseline/proposal.md
#   - openspec/changes/g1-tpch-baseline/design.md
#   - docs/releases/v3.9.0/plans/V390_TEST_PLAN.md §G1
#   - issue #3186
#
# Usage:
#   bash scripts/gate/check_g1_tpch_baseline.sh         # full run
#   bash scripts/gate/check_g1_tpch_baseline.sh --dry-run
#
# Exit codes:
#   0  PASS — all four sub-checks pass
#   1  FAIL — at least one sub-check failed (real drift, not a missing fixture)
#   2  INCOMPLETE — the comparison could not be performed (fixture missing,
#      unparsable baseline hash, or the comparator reported NOT-RUN).
#      Exit 2 must NEVER be reported as "hash mismatch": a NOT-RUN is not a drift.
#
# Environment:
#   TPCH_DATA_DIR     path to TPC-H .tbl files (default: ~/sqlrustgo-tpch/data)
#                     Missing ⇒ exit 2 (INCOMPLETE), because every sub-check would
#                     otherwise "pass" without executing a single query.
#   TPCH_TIMEOUT_S    per-query timeout seconds (default: 300)

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

HASH_FILE="${REPO_ROOT}/tests/tpch_hashes_v380.json"
PYTHON_SCRIPT="${REPO_ROOT}/scripts/gate/tpch_hash_compare.py"

DRY_RUN=0
if [[ "${1:-}" == "--dry-run" ]]; then
    DRY_RUN=1
fi

color_red()   { printf '\033[0;31m%s\033[0m' "$*"; }
color_green() { printf '\033[0;32m%s\033[0m' "$*"; }
color_yellow() { printf '\033[0;33m%s\033[0m' "$*"; }

step() { echo ""; echo "=== $* ==="; }
pass() { echo "  $(color_green PASS): $*"; }
fail() { echo "  $(color_red FAIL): $*"; }
incomplete() { echo "  $(color_yellow INCOMPLETE): $*"; }
warn() { echo "  $(color_yellow WARN): $*"; }

if [[ "${DRY_RUN}" == "1" ]]; then
    step "G1 dry-run"
    echo "  step 1: cargo test --test tpch_gate_test  (must be 22/22 PASS)"
    echo "  step 2: cargo test --test tpch_full_22_test  (must be 22/22 PASS)"
    echo "  step 3: cargo test --test tpch_hash_test  (must match baseline)"
    echo "  step 4: python3 ${PYTHON_SCRIPT} --check <hash-from-${HASH_FILE##*/}>  (must exit 0)"
    echo "  hash_file: ${HASH_FILE}"
    echo "  python:    ${PYTHON_SCRIPT}"
    exit 0
fi

OVERALL_RC=0
HASH_SHORT=""

# ---------------------------------------------------------------------------
# 0. Preflight
# ---------------------------------------------------------------------------
step "G1 preflight"

if ! command -v python3 >/dev/null 2>&1; then
    fail "python3 not on PATH"
    exit 1
fi
if ! command -v cargo >/dev/null 2>&1; then
    fail "cargo not on PATH"
    exit 1
fi
if [[ ! -f "${PYTHON_SCRIPT}" ]]; then
    fail "Python helper not found: ${PYTHON_SCRIPT}"
    exit 1
fi
if [[ ! -f "${HASH_FILE}" ]]; then
    fail "Hash baseline not found: ${HASH_FILE}"
    exit 1
fi

EXPECTED_HASH="$(python3 -c "
import json, sys
try:
    with open('${HASH_FILE}') as f:
        d = json.load(f)
    h = d.get('tpc_h_hash_sha256', '')
    if not isinstance(h, str) or len(h) != 64:
        sys.exit('hash missing or wrong length')
    print(h)
except Exception as e:
    sys.exit(f'read error: {e}')
" 2>/dev/null)" || {
    fail "could not parse ${HASH_FILE} (placeholder or malformed?)"
    warn "  action: run \\`python3 ${PYTHON_SCRIPT} --capture\\` and commit the hash"
    exit 2
}

HASH_SHORT="${EXPECTED_HASH:0:8}"
pass "hash file parsed: ${HASH_SHORT}..."

# ---------------------------------------------------------------------------
# 0b. TPC-H fixture preflight（诚实三态：缺夹具 = INCOMPLETE）
#
# 2026-11-13 实测：没有夹具时 4 个子检查里有 3 个报 PASS（"40 passed, 0.03s" /
# "50 passed, 0.04s"，一条查询都没跑），第 4 个把 NOT-RUN 译成 "hash mismatch"。
# 门禁宁可判 INCOMPLETE，也不许对着空数据判绿，也不许把 NOT-RUN 说成漂移。
# ---------------------------------------------------------------------------
FIXTURE_DIR="${TPCH_DATA_DIR:-${HOME}/sqlrustgo-tpch/data}"
MISSING_TBL=()
MISSING_COUNT=0
for tbl in customer lineitem nation orders part partsupp region supplier; do
    if [[ ! -f "${FIXTURE_DIR}/${tbl}.tbl" ]]; then
        MISSING_TBL+=("${tbl}.tbl")
        MISSING_COUNT=$((MISSING_COUNT + 1))
    fi
done

if [[ "${MISSING_COUNT}" -gt 0 ]]; then
    step "G1 INCOMPLETE: TPC-H fixture missing"
    echo "  fixture_dir: ${FIXTURE_DIR}"
    echo "  missing:     ${MISSING_TBL[*]} (${MISSING_COUNT}/8)"
    echo "  reason:      fixture_missing — 没有数据就无法比对基线哈希"
    incomplete "4 个子检查一次都不执行（它们只会在空数据上各自'通过'）"
    echo ""
    echo "G1 INCOMPLETE: TPC-H fixture missing at ${FIXTURE_DIR}; set TPCH_DATA_DIR"
    echo "               to a directory holding the 8 .tbl files. This is NOT a baseline drift."
    exit 2
fi

pass "fixture present: ${FIXTURE_DIR} (8/8 .tbl)"

# ---------------------------------------------------------------------------
# 0c. 把夹具交给 Rust 侧（2026-11-17）
# ---------------------------------------------------------------------------
# 本脚本（shell）的解析链是 TPCH_DATA_DIR → ~/sqlrustgo-tpch/data；
# Rust 侧（tests/common/tpch_fixture.rs::resolve_fixture_dir）的解析链是
# TPCH_DATA_DIR → 仓库内候选目录，**不含** ~/sqlrustgo-tpch/data。
#
# 不交接的后果（实测）：预检说「8/8 .tbl 都在」⇒ 本门禁准备做真实比对，
# 但 cargo 侧拿不到夹具 ⇒ `TPCH-NOT-RUN: ... reason=fixture_missing` ⇒
# 比对根本没发生。step 3 会把这条 NOT-RUN 当成「测试执行了」而报 PASS，
# step 4 的比较器才把它译成 INCOMPLETE —— 也就是说，**比较从未发生，
# 门禁却走到了结论**。AFP：没有实跑证据不得声称通过。
#
# TPCH_DATA_DIR 是 Rust 侧文档化的交接契约（CI 也用它递运行时夹具），
# 在这里导出，shell 验过的目录才会真的被 Rust 用上。
export TPCH_DATA_DIR="${FIXTURE_DIR}"

# ---------------------------------------------------------------------------
# 跑一个 TPC-H 测试目标，并把结果**诚实**地分成三类
# ---------------------------------------------------------------------------
# 返回码：0 = 真跑了且通过；1 = 真红；2 = 没跑成（自报 NOT-RUN）。
#
# 为什么不能只看退出码：这些测试在缺夹具时打印
# `TPCH-NOT-RUN: ... reason=fixture_missing` 之后**仍以 0 退出**（一条查询
# 都没跑）。只看退出码 ⇒「没跑」被记成「跑过了」⇒ 门禁对着一份**从未产生
# 的**比对下了结论。2026-11-17 实测：G1 的 step 3 就是这么把一次 NOT-RUN
# 说成「tpch_hash_test executed」的。
#
# 输出先全量落盘再判定：NOT-RUN 标记可能出现在很前面，`| tail -10`
# 会把它截掉 —— 截掉之后 grep 不到，空跑就会被当成通过。
run_tpch_test() {
    local target="$1"
    shift
    local out rc reason
    out="$(mktemp "${TMPDIR:-/tmp}/tpch_gate_out.XXXXXX")"
    timeout 1800 env "$@" cargo test --test "${target}" > "${out}" 2>&1
    rc=$?
    tail -10 "${out}"
    if [ "${rc}" -ne 0 ]; then
        # 诊断增量（2026-11-18）：Rust 的 assert_eq! 把实算哈希打在 panic 的
        # `left:` 行，上面的 `tail -10` 会把它截掉，而这里接着又把完整输出
        # 删掉 —— 「实算是多少」这条最关键的诊断信息被销毁两次，事后连取证
        # 都做不到（权威轮 013 的归档 stdout 就是这样只剩一句 hash mismatch）。
        # 在删除之前把那两行打出来：既有输出一字未动，只在真红时多打几行。
        if grep -qE '^[[:space:]]*left:' "${out}" 2>/dev/null; then
            echo "  computed vs baseline (from the Rust assertion):"
            grep -E '^[[:space:]]*(left|right):' "${out}" | sed 's/^/    /'
            echo "  locate it with: cargo test --test tpch_hash_test v380_candidate_dump -- --exact --nocapture"
        fi
        rm -f "${out}"
        return 1
    fi
    if grep -q "TPCH-NOT-RUN" "${out}"; then
        reason="$(grep -oE "reason=[a-z_]+" "${out}" | head -1 | cut -d= -f2)"
        echo "  NOT-RUN reason=${reason:-undisclosed} (see output above)"
        rm -f "${out}"
        return 2
    fi
    rm -f "${out}"
    return 0
}

# ---------------------------------------------------------------------------
# 1. tpch_gate_test (22/22 inline)
# ---------------------------------------------------------------------------
step "G1 step 1/4: cargo test --test tpch_gate_test"
run_tpch_test tpch_gate_test
TEST_RC=$?
case "${TEST_RC}" in
    0)
        pass "tpch_gate_test executed (see output above for the 22/22 line)"
        ;;
    2)
        incomplete "tpch_gate_test did NOT run (the test disclosed NOT-RUN) — 22/22 was not verified"
        if [ "${OVERALL_RC}" -eq 0 ]; then OVERALL_RC=2; fi
        ;;
    *)
        fail "tpch_gate_test exited non-zero (see output above)"
        OVERALL_RC=1
        ;;
esac

# ---------------------------------------------------------------------------
step "G1 step 2/4: cargo test --test tpch_full_22_test"
run_tpch_test tpch_full_22_test TPCH_FORCE=1
TEST_RC=$?
case "${TEST_RC}" in
    0)
        pass "tpch_full_22_test executed"
        ;;
    2)
        incomplete "tpch_full_22_test did NOT run (the test disclosed NOT-RUN) — 22/22 was not verified"
        if [ "${OVERALL_RC}" -eq 0 ]; then OVERALL_RC=2; fi
        ;;
    *)
        fail "tpch_full_22_test exited non-zero (see output above)"
        OVERALL_RC=1
        ;;
esac

# ---------------------------------------------------------------------------
# 3. tpch_hash_test (Rust regression test)
step "G1 step 3/4: cargo test --test tpch_hash_test"
run_tpch_test tpch_hash_test
TEST_RC=$?
case "${TEST_RC}" in
    0)
        pass "tpch_hash_test executed"
        ;;
    2)
        incomplete "tpch_hash_test did NOT run (the test disclosed NOT-RUN) — the hash was NOT compared"
        if [ "${OVERALL_RC}" -eq 0 ]; then OVERALL_RC=2; fi
        ;;
    *)
        fail "tpch_hash_test exited non-zero"
        OVERALL_RC=1
        ;;
esac

# ---------------------------------------------------------------------------
# 4. Python --check (independent verification)
# ---------------------------------------------------------------------------
step "G1 step 4/4: python3 tpch_hash_compare.py --check ${HASH_SHORT}..."
timeout 1800 python3 "${PYTHON_SCRIPT}" --check "${EXPECTED_HASH}" 2>&1
COMPARE_RC=$?
case "${COMPARE_RC}" in
    0)
        pass "python3 --check passed (hash matches baseline)"
        ;;
    2)
        # 比较器用 2 表示 NOT-RUN（夹具缺失 / profile 不匹配 / 未披露）——
        # 这不是「hash 不匹配」，把它说成漂移会制造一次不存在的回归。
        incomplete "python3 --check could not be performed (NOT-RUN; see reason above)"
        if [[ "${OVERALL_RC}" -eq 0 ]]; then OVERALL_RC=2; fi
        ;;
    *)
        fail "python3 --check failed (hash mismatch)"
        if [[ "${OVERALL_RC}" -eq 0 ]]; then OVERALL_RC=1; fi
        ;;
esac

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
echo ""
if [[ "${OVERALL_RC}" -eq 0 ]]; then
    echo "G1 PASS: TPC-H 22/22 baseline verified, hash=${HASH_SHORT}..."
elif [[ "${OVERALL_RC}" -eq 2 ]]; then
    echo "G1 INCOMPLETE: a sub-check could not be performed (see INCOMPLETE lines above)"
else
    echo "G1 FAIL: at least one sub-check failed (see above)"
fi
exit "${OVERALL_RC}"
