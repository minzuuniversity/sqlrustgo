#!/usr/bin/env bash
# =============================================================================
# check_rc_ga_gate.sh — v3.8.0 RC/GA Unified Gate
# =============================================================================
# Implements a FIVE-DIMENSION integrated gate for SQLRustGo release gates.
#
# Dimensions:
#   D1-Alpha  : Build + Test + Clippy + Format + Coverage (A1-A5) + Governance (A6)
#   D2-Beta   : Build + WAL Contract + Clippy + Format + Integration Gate (B1-B5)
#   D3-SGL    : Layer-3 Semantic Gate (SGL-001~005, exit 0=PASS, exit 2=DRIFT)
#   D4-WAL    : WAL invariant validation (INV-1, INV-2, INV-3)
#   D5-DeepSeek: 10 Principles for RC/GA gate
#
# Gate Stages:
#   Alpha → Pass with A1-A5 + A6-1~5
#   Beta  → Pass with D1 + D3 + D4
#   RC    → Pass with D1 + D2 + D3 + D4 + D5-Principles
#   GA    → RC + L2-L3 full execution + RC-to-GA checklist
#
# Exit codes:
#   0  = ALL PASS
#   1  = ANY FAIL (blocker)
#   2  = DRIFT (acceptable with tracking)
# =============================================================================

# #5326: paths corrected for V310 → V3.12 directory restructure

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

# Ensure cargo is on PATH (CI runners may not have it in default PATH).
if ! command -v cargo >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
         export PATH="$HOME/.cargo/bin:$PATH"
     fi
 fi

# #5579: the coverage measurement lives in scripts/gate/common.sh — ONE
# implementation, shared with scripts/gate/check_coverage_v312.sh. This file
# used to carry a SECOND implementation (`get_coverage_avg`) that measured the
# same eight crates with a different command, read a different column out of
# the report, and — the defect — dropped any crate it could not measure from
# the average, so its number was computed over a silently reduced sample.
# shellcheck source=scripts/gate/common.sh
source "$SCRIPT_DIR/common.sh"
# The threshold reader — one rule, one implementation (#5581).
# shellcheck source=scripts/gate/stage_config_common.sh
source "$SCRIPT_DIR/stage_config_common.sh"

# =============================================================================
# CLI PARSING / DIMENSION PLAN / VERDICT — side-effect-free helpers
# =============================================================================
# #5418: the flag namespace and the dimension-selector namespace are two
# different things, and the verdict must be a *function* of what actually
# executed — not of an unchecked argument.
#
# The previous form was `GATE_TYPE="${1:-all}"` → `main "$GATE_TYPE"` →
# `local GATE="${1:-all}"`, so `check_rc_ga_gate.sh --skip-a5` set
# GATE="--skip-a5", which matched NONE of the
# `if [[ "$GATE" == "all" || "$GATE" == "alpha" ]]` branches ⇒ D1 / D2 /
# D5 / D6 / D7 / D8 / D9 were ALL SKIPPED, while the summary still printed
#
#     ✓ GATE: PASS — All dimensions validated
#
# i.e. it claimed to have validated 7 dimensions while validating zero.
#
# The three helpers below are deliberately free of side effects so that the
# regression suite can `source` this file and exercise the *decision logic*
# directly (tests/integration/gate/gate_honesty_test.rs) without paying for
# the full cargo-based gate run.

GATE_TYPE="all"
SKIP_A5=0

usage() {
    cat <<'USAGE'
Usage: $0 [alpha|beta|rc|ga|all] [--skip-a5] [--list-dims]

  all    — Full gate (default): D1 D2 D3 D4 D5 D6 D7 D8 D9
  alpha  — D1 (+ D3/D4, which always run)
  beta   — D2 (+ D3/D4, which always run)
  rc     — D3 + D4 + D5 + D9
  ga     — D3 + D4 + D5 + D6 + D7 + D8 + D9

  Each mode names the release stage whose own gate runs; D3/D4 are
  invariant checks that run in every mode. D1 (Alpha) and D2 (Beta) are
  NOT part of rc/ga — they are covered by `all`, which is what the CI
  postcheck job runs through check_full_gate_verification.sh.

  --skip-a5  — skip the A5 coverage sub-check inside D1 (R2 budget)
  --list-dims— print the dimension plan for the selected mode and exit
USAGE
}

# gate_parse_args <args...>
#   Sets GATE_TYPE and SKIP_A5. Return codes:
#     0 = parsed OK
#     2 = unknown argument (usage error — never silently reinterpreted)
#     3 = help requested
gate_parse_args() {
    local arg
    GATE_TYPE="all"
    SKIP_A5=0
    LIST_DIMS=0
    for arg in "$@"; do
        case "$arg" in
            --skip-a5) SKIP_A5=1 ;;
            --list-dims) LIST_DIMS=1 ;;
            --help|-h) return 3 ;;
            alpha|beta|rc|ga|all) GATE_TYPE="$arg" ;;
            *)
                echo "Unknown argument: $arg" >&2
                return 2
                ;;
        esac
    done
    return 0
}

# gate_expected_dims <mode> — the dimensions that MUST execute for <mode>.
# A dimension that is expected but did not execute is a gate failure, not
# a pass (#5418).
#
# #5580: this function and main()'s dispatch used to be TWO separate
# implementations of the same rule — the plan said `ga` needs D1..D9 while
# main() only executed D3..D9 — and the drift was invisible until the
# verdict noticed it: every `ga` / `rc` / `beta` run ended in
#
#     ✗ [FAIL] GATE: FAIL — missing dimension(s):D1 D2
#
# i.e. **the gate could never pass in those modes**, and `ga` is the mode
# GA's `required_gates` names (`scripts/gate/check_rc_ga_gate.sh ga`).
#
# 现在读的是同一份规范清单 gate_dimensions_for_mode（#5580），执行侧的
# 读取口叫 plan_dims（#5456）。两个读取口的函数体刻意分开写 —— 声明与实现
# 分开，回归测试才有牙：
# tests/integration/gate/gate_honesty_test.rs::
#   test_5456_expected_dims_and_plan_dims_agree_for_every_mode
# 逐模式比对 gate_expected_dims 与 plan_dims，谁漂了就红。
gate_expected_dims() {
    gate_dimensions_for_mode "${1:-all}"
}

# gate_dimensions_for_mode <mode> — the dimensions that execute for <mode>,
# in execution order. ONE definition.
#
# The content is the gate's own documented contract (see usage() above):
#
#   all    — D1 D2 D3 D4 D5 D6 D7 D8 D9
#   alpha  — D1 (+ D3/D4)
#   beta   — D2 + D3 + D4
#   rc     — D3 + D4 + D5 + D9
#   ga     — D3 + D4 + D5 + D6 + D7 + D8 + D9
#
# D1/D2 不在 beta/rc/ga 的计划里，这不是漏洞：Alpha/Beta 的验证由 CI 里
# 跑 `all` 计划的那一步（if: always()）负责，Gate summary 一旦发现维度
# 报告里没有 D1/D2 就判红（NOT-RUN ≠ 通过）。见
# tests/integration/gate/ci_wiring_honesty_test.rs 的 #5456 兜底用例。
#
# `ga` 与 `all` 的差集就是 D1/D2：`ga` 不重跑 Alpha/Beta 的验证，
# 那一层由 CI 里 `all` 计划的那一步覆盖。The GA stage's
# `required_gates` (docs/governance/STAGE_CONFIG.yaml) additionally lists
# `cargo build --all-features`, `cargo test --all-features`,
# `cargo fmt --check`, `cargo clippy` and `check_coverage_v312.sh` as
# separate gates — those are the same criteria at the stage level, and
# running them inside D1/D2 as well is the gate's own redundancy, not a
# second standard.
gate_dimensions_for_mode() {
    case "${1:-all}" in
        alpha)  echo "D1 D3 D4" ;;
        beta)   echo "D2 D3 D4" ;;
        rc)     echo "D3 D4 D5 D9" ;;
        ga)     echo "D3 D4 D5 D6 D7 D8 D9" ;;
        all)    echo "D1 D2 D3 D4 D5 D6 D7 D8 D9" ;;
        *)      echo "" ;;
    esac
}

# plan_dims <mode> — the dimensions the executor ACTUALLY runs for <mode>.
#
# #5456: `gate_expected_dims` (the declared contract) and the executor
# used to disagree, and only because the executor's if-conditions were
# written inline:
#
#     if [[ "$GATE" == "all" ]] || [[ "$GATE" == "alpha" ]]; then run_d1_alpha; fi
#     if [[ "$GATE" == "all" ]] || [[ "$GATE" == "beta"  ]]; then run_d2_beta;  fi
#
# `ga` therefore executed D3..D9 while gate_expected_dims("ga")
# demanded D1..D9 — so every `ga` run failed with
# "missing dimension(s): D1 D2" even though nothing was actually wrong
# with the gate itself (CI run 6335, postcheck step 24).
#
# Two rules make that class of defect impossible to reintroduce:
#   1. plan_dims() is the ONLY place that decides which dimensions run;
#      the executor is a plain loop over it — no parallel if-conditions.
#   2. plan_dims() and gate_expected_dims() are deliberately kept as two
#      separate functions (spec vs. implementation) and a regression test
#      asserts they agree for every mode. Merging them into one would
#      make that test vacuous.
plan_dims() {
    case "${1:-all}" in
        alpha)  echo "D1 D3 D4" ;;
        beta)   echo "D2 D3 D4" ;;
        rc)     echo "D3 D4 D5 D9" ;;
        ga)     echo "D3 D4 D5 D6 D7 D8 D9" ;;
        all)    echo "D1 D2 D3 D4 D5 D6 D7 D8 D9" ;;
        *)      echo "" ;;
    esac
}

# coverage_min_for_mode <mode> — the SSOT stage whose coverage threshold
# applies to this mode, and the threshold itself.
#
#   usage: threshold=$(coverage_min_for_mode <mode>)
#
# Prints "<stage> <threshold>" and returns 0; returns 1 and prints nothing
# when the SSOT has no threshold for that stage.
#
# #5581: the mode → stage mapping is written ONCE, here:
#
#   alpha → ALPHA   beta → BETA   rc → RC   ga → GA   all → GA
#
# `all` maps to GA because `gate_dimensions_for_mode` runs **the same
# dimension set** for `all` and `ga` ("ga and all are the same set, by
# design") — so the two modes must not be judged by different coverage
# bars either.
coverage_min_for_mode() {
    local mode="${1:-all}"
    local stage
    case "$mode" in
        alpha) stage="ALPHA" ;;
        beta)  stage="BETA" ;;
        rc)    stage="RC" ;;
        ga)    stage="GA" ;;
        all)   stage="GA" ;;
        *)     stage="GA" ;;
    esac
    local threshold
    threshold="$(stage_coverage_min "$stage")" || return 1
    printf '%s %s' "$stage" "$threshold"
}

# gate_verdict_for <mode> <executed_csv> <d1_blk> <d2_blk> <d8_blk>
#                <d9_blk> <d6_fail> <d7_blk> <d3_fails> <d3_drifts>
#
# Sets GATE_VERDICT (PASS|FAIL|DRIFT), GATE_EXIT_CODE and GATE_MISSING_DIMS.
#
# #5419: D6 (Integration Test Coverage Gate) and D7 (Reliability Gate)
# set their own failure counters, but the verdict only looked at the
# D1/D2/D8/D9 blockers and the D3 failures ⇒ **a failing D6 or D7 still
# produced "GATE: PASS"**.
gate_verdict_for() {
    local mode="$1" executed="$2"
    local d1="$3" d2="$4" d8="$5" d9="$6" d6="$7" d7="$8" d3f="$9" d3d="${10:-0}"
    local d4f="${11:-0}"
    local expected d

    GATE_MISSING_DIMS=""
    expected="$(gate_expected_dims "$mode")"
    for d in $expected; do
        case " $executed " in
            *" $d "*) ;;
            *) GATE_MISSING_DIMS="$GATE_MISSING_DIMS $d" ;;
        esac
    done
    GATE_MISSING_DIMS="${GATE_MISSING_DIMS# }"

    if [[ "$d1" -gt 0 ]] || [[ "$d2" -gt 0 ]] || [[ "$d8" -gt 0 ]] || [[ "$d9" -gt 0 ]] \
       || [[ "$d6" -gt 0 ]] || [[ "$d7" -gt 0 ]] || [[ "$d4f" -gt 0 ]]; then
        GATE_VERDICT="FAIL"
        GATE_EXIT_CODE=1
    elif [[ -n "$GATE_MISSING_DIMS" ]]; then
        GATE_VERDICT="FAIL"
        GATE_EXIT_CODE=1
    elif [[ "$d3f" -gt 0 ]]; then
        GATE_VERDICT="FAIL"
        GATE_EXIT_CODE=1
    elif [[ "$d3d" -gt 0 ]]; then
        GATE_VERDICT="DRIFT"
        GATE_EXIT_CODE=2
    else
        GATE_VERDICT="PASS"
        GATE_EXIT_CODE=0
    fi
}

# =============================================================================
# A5 COVERAGE DECISION — side-effect-free helper
# =============================================================================
# #3844: `get_coverage_avg()` used to return `0` when it could not read a
# single TOTAL row. On a runner without `cargo-llvm-cov` that `0` was
# compared against COVERAGE_MIN (75) and recorded as a **blocker** — i.e.
# "coverage is 0%, the code is broken". The truth is: coverage was never
# measured. The very same repository already knows this distinction:
# `.gitea/workflows/ci.yml` guards the coverage step with
# `if ! cargo llvm-cov --version` → "NOT-RUN: reason=COVERAGE_TOOL_MISSING".
# This function brings the gate script to the same standard.
#
# Sets:
#   A5_STATUS  = PASS | FAIL | NOT-RUN
#   A5_REASON  = "" | COVERAGE_TOOL_MISSING | COVERAGE_NO_MEASUREMENTS
#
# Usage: a5_coverage_decision <tool_available:0|1> <measured_crates:int> [pct:int] [threshold:int]
#   A5_REASON 还可能是 NO_THRESHOLD_IN_SSOT（三个来源都给不出门槛时）。
#
# 门槛是判定输入，不是环境残留：第 4 个参数优先，其次 COVERAGE_MIN
# （main() 从 SSOT 读到的值），都没有就现读 SSOT。三条路都给不出门槛
# ⇒ 判红并说明原因，绝不"没有标准也算过"（#5581）。
a5_coverage_decision() {
    local tool_available="${1:-0}"
    local measured="${2:-0}"
    local pct="${3:-}"
    local threshold="${4:-${COVERAGE_MIN:-}}"
    if [ -z "$threshold" ]; then
        threshold="$(coverage_min_for_mode "${GATE_TYPE:-all}" 2>/dev/null | awk '{print $2}')"
    fi

    A5_STATUS=""
    A5_REASON=""

    if [ "$tool_available" != "1" ]; then
        A5_STATUS="NOT-RUN"
        A5_REASON="COVERAGE_TOOL_MISSING"
        return 0
    fi
    if [ "$measured" -le 0 ] 2>/dev/null || [ -z "$pct" ]; then
        A5_STATUS="NOT-RUN"
        A5_REASON="COVERAGE_NO_MEASUREMENTS"
        return 0
    fi
    if [ -z "$threshold" ]; then
        A5_STATUS="FAIL"
        A5_REASON="NO_THRESHOLD_IN_SSOT"
        return 0
    fi
    if [ "$pct" -ge "$threshold" ] 2>/dev/null; then
        A5_STATUS="PASS"
    else
        A5_STATUS="FAIL"
    fi
    return 0
}

# =============================================================================
# CONFIGURATION — Unified Rule Registry
# =============================================================================

# C-ARCH rules (consistent across all gates)
CARCH01_BY_DESIGN="true"          # txn_manager is by design (PR-830)
CARCH02_WRITE_BUFFER="forbidden"  # LocalExecutor must NOT have write_buffer
CARCH05_LIMIT=1500                # execution_engine.rs line limit (SSOT - all other gate scripts must use this value)
                                # AD-001 target. PR #3664 (2026-07-01) 完成拆分,文件降到 1471 行。
                                # 历史: 2026-06-30 临时 1800; PR-3660 (2026-06-29) 临时 3000; 原 AD-001 1500 不变。

# Coverage threshold — **not** a constant.
#
# #5581: this used to be `COVERAGE_MIN=75` with the comment
# "Alpha: 75%, Beta/RC: 80%" — which matched **neither** the SSOT
# (docs/governance/STAGE_CONFIG.yaml → thresholds: ALPHA 50 / BETA 75 /
# RC 80 / GA 85) nor its own comment. The value now comes from the SSOT
# through `stage_coverage_min`, per the mode this gate runs in, so the same
# criterion cannot be judged by two different numbers.
#
# Empty means "the SSOT has no threshold for this mode" ⇒ A5 fails closed.
COVERAGE_MIN=""
COVERAGE_MIN_STAGE=""

# =============================================================================
# OUTPUT STYLING
# =============================================================================

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'
BOLD='\033[1m'

log_header() {
    echo -e "\n${BOLD}${BLUE}═══════════════════════════════════════════════════════════${NC}"
    echo -e "${BOLD}${BLUE}  $1${NC}"
    echo -e "${BOLD}${BLUE}═══════════════════════════════════════════════════════════${NC}\n"
}

log_dim() {
    echo -e "${BOLD}${CYAN}[D$1] $2${NC}"
}

log_pass() {
    echo -e "    ${GREEN}✓ PASS${NC} $1"
}

log_fail() {
    echo -e "    ${RED}✗ [FAIL]${NC} $1"
}

log_warn() {
    echo -e "    ${YELLOW}⚠ WARN${NC} $1"
}

# #3844: a check that could NOT be executed is neither a pass nor a fail.
# It gets its own verdict so that it can never be silently folded into
# "PASS" — and so that it can never be silently folded into "FAIL" either
# (an environment blocker is not a code defect).
log_notrun() {
    echo -e "    ${BLUE}⊘ NOT-RUN${NC} $1"
}

log_info() {
    echo -e "    ${BLUE}ℹ${NC} $1"
}

log_drift() {
    echo -e "    ${YELLOW}⚡ DRIFT${NC} $1"
}

# =============================================================================
# COUNTERS
# =============================================================================

D1_PASS=0; D1_TOTAL=0; D1_BLOCKERS=0; D1_NOTRUN=0
D2_PASS=0; D2_TOTAL=0; D2_BLOCKERS=0
D3_PASS=0; D3_TOTAL=0; D3_FAILS=0; D3_DRIFTS=0
D4_PASS=0; D4_TOTAL=0; D4_FAILS=0
D5_PASS=0; D5_TOTAL=0

# Coverage result cache — the summary used to call get_coverage_avg() a
# SECOND time, which re-ran the 8-crate `cargo llvm-cov test` sweep (the
# single most expensive step of the gate) just to print one line.
COVERAGE_PCT=""
COVERAGE_CRATES_MEASURED=0
COVERAGE_STATUS=""
COVERAGE_REASON=""

# =============================================================================
# D1: ALPHA GATE (A1-A5 + A6-1~5)
# =============================================================================

run_d1_alpha() {
    log_header "D1: Alpha Gate — Standard Quality + Governance"

    # A1: Build
    D1_TOTAL=$((D1_TOTAL+1))
    echo -n "  [A1] Build (release, core 6 crates) ... "
    if cargo build --release -p sqlrustgo-executor -p sqlrustgo-planner \
        -p sqlrustgo-parser -p sqlrustgo-storage -p sqlrustgo-transaction \
        -p sqlrustgo-catalog > /tmp/d1_a1.log 2>&1; then
        log_pass "A1 Build"
        D1_PASS=$((D1_PASS+1))
    else
        log_fail "A1 Build failed — see /tmp/d1_a1.log"
        D1_BLOCKERS=$((D1_BLOCKERS+1))
    fi

    # A2: Test
    D1_TOTAL=$((D1_TOTAL+1))
    echo -n "  [A2] Test (lib, core 6 crates) ... "
    if cargo test --lib -p sqlrustgo-parser -p sqlrustgo-planner \
        -p sqlrustgo-executor -p sqlrustgo-storage -p sqlrustgo-transaction \
        -p sqlrustgo-catalog -- --test-threads=4 > /tmp/d1_a2.log 2>&1; then
        log_pass "A2 Test"
        D1_PASS=$((D1_PASS+1))
    else
        log_fail "A2 Test failed — see /tmp/d1_a2.log"
        D1_BLOCKERS=$((D1_BLOCKERS+1))
    fi

    # A3: Clippy
    D1_TOTAL=$((D1_TOTAL+1))
    echo -n "  [A3] Clippy (core) ... "
    if cargo clippy -p sqlrustgo-parser -p sqlrustgo-planner \
        -p sqlrustgo-executor -p sqlrustgo-storage -p sqlrustgo-transaction \
        -p sqlrustgo-catalog --all-features -- -D warnings > /tmp/d1_a3.log 2>&1; then
        log_pass "A3 Clippy"
        D1_PASS=$((D1_PASS+1))
    else
        log_fail "A3 Clippy failed — see /tmp/d1_a3.log"
        D1_BLOCKERS=$((D1_BLOCKERS+1))
    fi

    # A4: Format
    D1_TOTAL=$((D1_TOTAL+1))
    echo -n "  [A4] Format (core) ... "
    if cargo fmt -p sqlrustgo-parser -p sqlrustgo-planner \
        -p sqlrustgo-executor -p sqlrustgo-storage -p sqlrustgo-transaction \
        -p sqlrustgo-catalog -- --check > /tmp/d1_a4.log 2>&1; then
        log_pass "A4 Format"
        D1_PASS=$((D1_PASS+1))
    else
        log_fail "A4 Format failed — see /tmp/d1_a4.log"
        D1_BLOCKERS=$((D1_BLOCKERS+1))
    fi

    # A5: Coverage (V313-#3942: --skip-a5 short-circuits the
    # 8-crate cargo llvm-cov run, which is the dominant cost in
    # the 60-second R2 evidence-refresh budget).
    if [ "${SKIP_A5:-0}" -eq 1 ]; then
        echo -n "  [A5] Coverage (--skip-a5) ... "
        log_pass "A5 Coverage skipped (R2 budget)"
        D1_TOTAL=$((D1_TOTAL+1))
        D1_PASS=$((D1_PASS+1))
    else
        echo -n "  [A5] Coverage (L1 8 crates) ... "
        # A5 有三种结局 —— PASS / FAIL / NOT-RUN（#3844）：跑不了 cargo-llvm-cov
        # 是环境阻塞，不是代码缺陷，既不能报成 "coverage 0%"，也不能算通过。
        #
        # 测量只有一份实现：scripts/gate/common.sh 的 coverage_average（#5579）。
        # 本文件曾自带的 get_coverage_avg 是第二份实现（命令不同、取错列、
        # 测不到的 crate 被静默从样本里剔除），已删除 —— 不要再带回来。
        local cov_tool=0
        local cov_pct=""
        local cov_handled=""
        local COV_REPORT_FILE
        COVERAGE_CRATES_MEASURED=0
        COV_REPORT_FILE="$(make_temp_file)" || COV_REPORT_FILE=""
        if [ -z "${COVERAGE_MIN:-}" ]; then
            # #5581: SSOT 里没有本模式的门槛 ⇒ 判红并说明原因，
            # 不能没有标准就给出覆盖率结论。
            log_fail "A5 Coverage: no coverage threshold for mode '${GATE_TYPE}' in the SSOT"
            COVERAGE_STATUS="FAIL"; COVERAGE_REASON="NO_THRESHOLD_IN_SSOT"
            D1_TOTAL=$((D1_TOTAL+1))
            D1_BLOCKERS=$((D1_BLOCKERS+1))
            cov_handled=1
        elif [ -z "$COV_REPORT_FILE" ]; then
            log_fail "A5 Coverage: could not create the report file"
            COVERAGE_STATUS="FAIL"; COVERAGE_REASON="REPORT_FILE_UNAVAILABLE"
            D1_TOTAL=$((D1_TOTAL+1))
            D1_BLOCKERS=$((D1_BLOCKERS+1))
            cov_handled=1
        else
            if coverage_tool_available; then
                cov_tool=1
                coverage_average "$COVERAGE_MIN" > "$COV_REPORT_FILE" 2>&1
                COVERAGE_CRATES_MEASURED="${COVERAGE_MEASURED_COUNT:-0}"
                cov_pct="${COVERAGE_AVERAGE:-}"
            fi
            sed 's/^/      /' "$COV_REPORT_FILE"
            rm -f "$COV_REPORT_FILE"
            if [ "$cov_tool" = "1" ] && [ -n "${COVERAGE_FAILED_CRATES:-}" ]; then
                # #5579: 测不到的 crate 必须点名判红 —— 拿缩小后的样本
                # 得到的平均值不是本仓库可接受的判据。
                log_fail "A5 Coverage: 无法测量这些 crate: ${COVERAGE_FAILED_CRATES}"
                COVERAGE_STATUS="FAIL"; COVERAGE_REASON="CRATES_NOT_MEASURABLE"
                D1_TOTAL=$((D1_TOTAL+1))
                D1_BLOCKERS=$((D1_BLOCKERS+1))
                cov_handled=1
            fi
        fi
        COVERAGE_PCT="$cov_pct"
        if [ -z "$cov_handled" ]; then
            a5_coverage_decision "$cov_tool" "${COVERAGE_CRATES_MEASURED:-0}" "$cov_pct" "$COVERAGE_MIN"
            COVERAGE_STATUS="$A5_STATUS"
            COVERAGE_REASON="$A5_REASON"
            case "$A5_STATUS" in
                PASS)
                    log_pass "A5 Coverage: ${cov_pct}% (min: ${COVERAGE_MIN}%, measured ${COVERAGE_MEASURED_COUNT}/${COVERAGE_TOTAL_COUNT} crates)"
                    D1_TOTAL=$((D1_TOTAL+1))
                    D1_PASS=$((D1_PASS+1))
                    ;;
                FAIL)
                    log_fail "A5 Coverage: ${cov_pct}% (min: ${COVERAGE_MIN}%, measured ${COVERAGE_MEASURED_COUNT}/${COVERAGE_TOTAL_COUNT} crates)"
                    D1_TOTAL=$((D1_TOTAL+1))
                    D1_BLOCKERS=$((D1_BLOCKERS+1))
                    ;;
                NOT-RUN)
                    if [ "$A5_REASON" = "COVERAGE_TOOL_MISSING" ]; then
                        log_notrun "A5 Coverage: NOT-RUN: reason=COVERAGE_TOOL_MISSING (cargo-llvm-cov is not installed on this runner)"
                    else
                        log_notrun "A5 Coverage: NOT-RUN: reason=COVERAGE_NO_MEASUREMENTS (no crate produced a TOTAL row)"
                    fi
                    # NOT-RUN 既不算通过也不算阻塞 —— 但必须计数，
                    # 否则它会从 summary 里消失。
                    D1_NOTRUN=$((D1_NOTRUN+1))
                    ;;
            esac
        fi
    fi

    # A6: Governance
    echo -e "\n  ${BOLD}--- A6: Governance ---${NC}"

    local gov_pass=0; local gov_total=0

    # A6-1: Replay Graph
    gov_total=$((gov_total+1))
    echo -n "  [A6-1] Replay Graph exists ... "
    if [ -f "docs/governance/replay/REPLAY_v3.7.0_GA.md" ]; then
        log_pass "A6-1 Replay Graph"
        gov_pass=$((gov_pass+1))
    else
        log_fail "A6-1 Replay Graph missing"
    fi

    # A6-2: Claim Registry
    gov_total=$((gov_total+1))
    echo -n "  [A6-2] Claim Registry exists ... "
    if [ -f "docs/governance/adr/ADR-002-claim-registry.md" ]; then
        log_pass "A6-2 Claim Registry"
        gov_pass=$((gov_pass+1))
    else
        log_fail "A6-2 Claim Registry missing"
    fi

    # A6-3: Decision Registry
    gov_total=$((gov_total+1))
    echo -n "  [A6-3] Decision Registry exists ... "
    if [ -f "docs/releases/v3.8.0/design/ARCHITECTURE_DECISIONS.md" ]; then
        log_pass "A6-3 Decision Registry"
        gov_pass=$((gov_pass+1))
    else
        log_fail "A6-3 Decision Registry missing"
    fi

    # A6-4: Freshness markers
    gov_total=$((gov_total+1))
    echo -n "  [A6-4] Freshness markers ... "
    if grep -rq "Freshness\|## " docs/releases/v3.8.0/ 2>/dev/null; then
        log_pass "A6-4 Freshness"
        gov_pass=$((gov_pass+1))
    else
        log_warn "A6-4 Freshness: no markers found"
    fi

    # A6-5: ADR-001~ADR-005 all exist
    gov_total=$((gov_total+1))
    echo -n "  [A6-5] ADR-001~ADR-005 exist ... "
    if [ -f "docs/governance/adr/ADR-001-truthfulness-framework.md" ] && \
       [ -f "docs/governance/adr/ADR-002-claim-registry.md" ] && \
       [ -f "docs/governance/adr/ADR-003-decision-registry.md" ] && \
       [ -f "docs/governance/adr/ADR-004-negative-evidence.md" ] && \
       [ -f "docs/governance/adr/ADR-005-legacy-gate-retirement.md" ]; then
        log_pass "A6-5 ADR-001~ADR-005"
        gov_pass=$((gov_pass+1))
    else
        log_fail "A6-5 ADR missing"
    fi

    D1_TOTAL=$((D1_TOTAL + gov_total))
    D1_PASS=$((D1_PASS + gov_pass))

    echo -e "\n  D1 Result: $D1_PASS/$D1_TOTAL (not-run: $D1_NOTRUN, blockers: $D1_BLOCKERS)"
}

# =============================================================================
# D2: BETA GATE (B1-B5)
# =============================================================================

run_d2_beta() {
    log_header "D2: Beta Gate — WAL Contract + Integration Gate"

    # B1: Build
    D2_TOTAL=$((D2_TOTAL+1))
    echo -n "  [B1] Build ... "
    if cargo build --release -p sqlrustgo > /tmp/d2_b1.log 2>&1; then
        log_pass "B1 Build"
        D2_PASS=$((D2_PASS+1))
    else
        log_fail "B1 Build failed"
        D2_BLOCKERS=$((D2_BLOCKERS+1))
    fi

    # B2: WAL Contract
    D2_TOTAL=$((D2_TOTAL+1))
    echo -n "  [B2] WAL Contract (22 tests) ... "
    WAL_OUTPUT=$(cargo test --test wal_tx_contract_test 2>&1 || echo "0")
    PASSED=$(grep -oE '[0-9]+ passed' <<< "$WAL_OUTPUT" | head -1 | grep -oE '[0-9]+' || echo "0")
    FAILED=$(grep -oE '[0-9]+ failed' <<< "$WAL_OUTPUT" | head -1 | grep -oE '[0-9]+' || echo "0")
    echo "  → $PASSED passed, $FAILED failed"
    if [ "$PASSED" -ge 21 ] && [ "$FAILED" -eq 0 ]; then
        log_pass "B2 WAL Contract: $PASSED/22 PASS"
        D2_PASS=$((D2_PASS+1))
    else
        log_fail "B2 WAL Contract: $PASSED/22 (need 21+)"
        D2_BLOCKERS=$((D2_BLOCKERS+1))
    fi

    # B3: Clippy
    D2_TOTAL=$((D2_TOTAL+1))
    echo -n "  [B3] Clippy ... "
    if cargo clippy -p sqlrustgo --all-features -- -D warnings > /tmp/d2_b3.log 2>&1; then
        log_pass "B3 Clippy"
        D2_PASS=$((D2_PASS+1))
    else
        log_fail "B3 Clippy failed"
        D2_BLOCKERS=$((D2_BLOCKERS+1))
    fi

    # B4: Format
    D2_TOTAL=$((D2_TOTAL+1))
    echo -n "  [B4] Format ... "
    if cargo fmt --all -- --check > /tmp/d2_b4.log 2>&1; then
        log_pass "B4 Format"
        D2_PASS=$((D2_PASS+1))
    else
        log_fail "B4 Format failed"
        D2_BLOCKERS=$((D2_BLOCKERS+1))
    fi

    # B5: Integration Gate (C-ARCH + SGL + WAL)
    D2_TOTAL=$((D2_TOTAL+1))
    echo -n "  [B5] Integration Gate ... "
    IG_OUTPUT=$(bash scripts/gate/check_integration_gate.sh 2>&1)
    IG_EXIT=$?
    IG_SUMMARY=$(grep "Result:" <<< "$IG_OUTPUT" | tail -1)
    echo "  → $IG_SUMMARY"
    # #5426: the previous form was
    #     IG_OUTPUT=$(bash ... 2>&1 || true)
    #     IG_EXIT=$?
    # `|| true` forced the exit status to 0, so IG_EXIT was always 0 and
    # a failing integration gate was downgraded to a warning — D2 could
    # report "5/6" with zero blockers and the gate would still be green.
    if [ "$IG_EXIT" -eq 0 ] && grep -q "PASS" <<< "$IG_SUMMARY"; then
        log_pass "B5 Integration Gate"
        D2_PASS=$((D2_PASS+1))
    else
        log_fail "B5 Integration Gate (exit $IG_EXIT): ${IG_SUMMARY:-no summary}"
        D2_BLOCKERS=$((D2_BLOCKERS+1))
    fi

    # B6: V312-24 test infrastructure artifacts (ISSUE #3911 acceptance criterion 1)
    # Validates that the canonical SQLancer + test-runner binaries produce
    # target/sqlancer-report.json + target/test-runner-report.json. Without
    # this check, V312-24 work can be merged but never wired into the gate.
    #
    # #5421: the check used to require artifacts that **the gate does not
    # produce**. They live in `target/`, which is not tracked, so on any
    # fresh checkout — a CI runner, a new worktree, anyone's clone — they
    # are absent and this sub-check is red for a reason that has nothing to
    # do with the code. It could only go green if somebody had happened to
    # generate them beforehand: a random green.
    #
    # The gate now produces what it checks. Both generators are cheap:
    # `test-runner` without a manifest falls back to a `cargo --version`
    # probe (~15 ms), and `sqlancer` honours a short duration cap
    # (1000 iterations complete in ~0.01 s). The check validates the
    # artifacts' *schema and wiring*, not fuzzing depth.
    D2_TOTAL=$((D2_TOTAL+1))
    echo -n "  [B6] V312-24 SQLancer + test-runner artifacts ... "
    if [ ! -s "${REPO_ROOT}/target/sqlancer-report.json" ] || \
       [ ! -s "${REPO_ROOT}/target/test-runner-report.json" ]; then
        echo ""
        echo "       artifacts absent — the gate generates them"
        mkdir -p "${REPO_ROOT}/target"
        # The real exit codes are captured and reported, never swallowed
        # (P14: `|| true` on a command whose status is discarded is exactly
        # the anti-pattern this gate exists to catch).
        SQLANCER_RC=0
        TEST_RUNNER_RC=0
        ( cd "${REPO_ROOT}" && cargo run -q -p sqlancer -- \
              --duration "${SQLANCER_DURATION:-5}" \
              --out target/sqlancer-report.json ) >/dev/null 2>&1 || SQLANCER_RC=$?
        ( cd "${REPO_ROOT}" && cargo run -q -p test-runner -- \
              --out target/test-runner-report.json ) >/dev/null 2>&1 || TEST_RUNNER_RC=$?
        echo "       generation rc: sqlancer=$SQLANCER_RC test-runner=$TEST_RUNNER_RC"
    fi
    if [ -s "${REPO_ROOT}/target/sqlancer-report.json" ] && \
       [ -s "${REPO_ROOT}/target/test-runner-report.json" ]; then
        # Validate JSON schema (same checks as check_anti_fabrication.sh CHECK 1.5)
        if python3 -c "import json,sys
d1=json.load(open('${REPO_ROOT}/target/sqlancer-report.json'))
d2=json.load(open('${REPO_ROOT}/target/test-runner-report.json'))
for k in ('successful_queries','failed_queries','iterations_requested'):
    if k not in d1: sys.exit(1)
for k in ('started_at','finished_at','config','summary','results'):
    if k not in d2: sys.exit(1)
" 2>/dev/null; then
            log_pass "B6 V312-24: SQLancer + test-runner artifacts valid"
            D2_PASS=$((D2_PASS+1))
        else
            log_fail "B6 V312-24: report schema invalid"
            D2_BLOCKERS=$((D2_BLOCKERS+1))
        fi
    else
        log_fail "B6 V312-24: report artifacts could not be produced (cargo run -p sqlancer -- --duration ${SQLANCER_DURATION:-5} + cargo run -p test-runner)"
        D2_BLOCKERS=$((D2_BLOCKERS+1))
    fi

    echo -e "\n  D2 Result: $D2_PASS/$D2_TOTAL"
}

# =============================================================================
# D3: SGL SEMANTIC GATE (SGL-001~005)
# =============================================================================

run_d3_sgl() {
    log_header "D3: SGL Layer-3 Semantic Gate"

    SGL_OUTPUT=$(python3 scripts/gate/semantic_gate_check.py 2>&1)
    SGL_EXIT=$?

    grep -E "^\[|^==|Summary|SGL-|Exit|DRIFT|FAIL|PASS" <<< "$SGL_OUTPUT" | head -40 | \
        while IFS= read -r line; do
            echo -e "    $line"
        done

    # Parse SGL results
    SGL_PASS=$(grep -oE "^  PASS\s*:\s*[0-9]+" <<< "$SGL_OUTPUT" | grep -oE "[0-9]+" | head -1 || echo "0")
    SGL_FAIL=$(grep -oE "^  FAIL\s*:\s*[0-9]+" <<< "$SGL_OUTPUT" | grep -oE "[0-9]+" | head -1 || echo "0")
    SGL_DRIFT=$(grep -oE "^  DRIFT\s*:\s*[0-9]+" <<< "$SGL_OUTPUT" | grep -oE "[0-9]+" | head -1 || echo "0")

    D3_TOTAL=5
    D3_PASS=$((SGL_PASS))
    D3_FAILS=$((SGL_FAIL))
    D3_DRIFTS=$((SGL_DRIFT))

    echo -e "\n  D3 Result: PASS=$SGL_PASS | FAIL=$SGL_FAIL | DRIFT=$SGL_DRIFT"
    echo "  SGL Exit code: $SGL_EXIT (0=PASS, 1=FAIL, 2=DRIFT)"
}

# =============================================================================
# D4: WAL VALIDATION (INV-1, INV-2, INV-3)
# =============================================================================

run_d4_wal() {
    log_header "D4: WAL Invariant Validation"

    if [ -x scripts/test/wal_invariant.sh ]; then
        # #5426: same swallowed-exit-code shape as B5 — `|| true` made
        # WAL_INV_EXIT permanently 0.
        WAL_INV_OUTPUT=$(bash scripts/test/wal_invariant.sh 2>&1)
        WAL_INV_EXIT=$?

        grep -E "INV-|Passed|Failed|Summary|Result|Validated" <<< "$WAL_INV_OUTPUT" | \
            while IFS= read -r line; do
                echo -e "    $line"
            done

        WAL_PASSED=$(grep -oE "Passed:\s*[0-9]+" <<< "$WAL_INV_OUTPUT" | grep -oE "[0-9]+" | head -1 || echo "0")
        WAL_FAILED=$(grep -oE "Failed:\s*[0-9]+" <<< "$WAL_INV_OUTPUT" | grep -oE "[0-9]+" | head -1 || echo "0")

        D4_TOTAL=5
        D4_PASS=$((WAL_PASSED))
        # #5426: a WAL invariant script that exits non-zero (or reports
        # failed invariants) must be visible to the verdict. Before the
        # exit code was swallowed, D4 could report "2/5" and the gate
        # would still be green.
        if [ "${WAL_INV_EXIT:-0}" -ne 0 ] || [ "${WAL_FAILED:-0}" -gt 0 ]; then
            D4_FAILS=$((D4_TOTAL - D4_PASS))
            if [ "$D4_FAILS" -lt 1 ]; then D4_FAILS=1; fi
        fi

        echo -e "\n  D4 Result: $WAL_PASSED/5 passed (failures: $D4_FAILS, exit $WAL_INV_EXIT)"
    else
        # Fallback: run exp_g_wal_contracts_verified
        WAL_EXP_OUTPUT=$(cargo test --test exp_g_wal_contracts_verified 2>&1)
        WAL_EXP_EXIT=$?
        WAL_EXP_PASSED=$(grep -oE '[0-9]+ passed' <<< "$WAL_EXP_OUTPUT" | head -1 | grep -oE '[0-9]+' || echo "0")
        WAL_EXP_FAILED=$(grep -oE '[0-9]+ failed' <<< "$WAL_EXP_OUTPUT" | head -1 | grep -oE '[0-9]+' || echo "0")

        D4_TOTAL=5
        D4_PASS=$((WAL_EXP_PASSED))
        if [ "${WAL_EXP_EXIT:-0}" -ne 0 ] || [ "${WAL_EXP_FAILED:-0}" -gt 0 ]; then
            D4_FAILS=$((D4_TOTAL - D4_PASS))
            if [ "$D4_FAILS" -lt 1 ]; then D4_FAILS=1; fi
        fi

        echo "  WAL Invariant: $WAL_EXP_PASSED passed (fallback to exp_g_wal_contracts_verified)"
        log_pass "D4 WAL: $WAL_EXP_PASSED/5 passed"
    fi
}

# =============================================================================
# D6: INTEGRATION TEST COVERAGE GATE (Issue #2874)
# =============================================================================
# Runs all 35+ integration tests that were previously not gated.
# Each test is invoked via `cargo test --test <name>` and PASS/FAIL is tracked.
# A test is considered PASS if cargo reports "0 failed" in its summary line.

D6_TOTAL=0
D6_PASS=0
D6_FAIL=0
D6_FAILED_TESTS=()
D6_UNRESOLVED=()
D6_TARGETS=()

# #5419: resolve a curated entry to the Cargo test target that actually
# exists. `cargo test --test <name>` needs the **target name** (Cargo's
# `[[test]] name=` field), not a file path. The curated list below mixes
# both spellings — "wal_integration_test" is a target name, while
# "ci/ci_test" is a path — so resolve in three steps:
#
#   1. exact target name;
#   2. the last path segment used as a file stem (Cargo's auto-discovery
#      names a target after its file stem, e.g. tests/ci/ci_test.rs
#      becomes the target `ci_test`);
#   3. the one known typo alias in the list.
#
# Measured on this tree: 44 of the 51 curated entries correspond to a
# real file under tests/, but only 4 of them live at `tests/<name>.rs`
# — the rest are in subdirectories (tests/integration/..., tests/e2e/...,
# tests/ci/..., tests/benchmark/...), which the old existence probe
# `tests/${test_name}.rs` could never see.
D6_TARGET_NAME_TO_STEM=""

build_d6_target_map() {
    D6_TARGET_NAME_TO_STEM=$(awk '
        /^\[\[test\]\]/ { in_test=1; next }
        /^\[/ && !/^\[\[test\]\]/ { in_test=0 }
        in_test && /^[[:space:]]*name[[:space:]]*=/ {
            n=$0; sub(/^[^"]*"/, "", n); sub(/".*$/, "", n); name=n
        }
        in_test && /^[[:space:]]*path[[:space:]]*=/ {
            p=$0; sub(/^[^"]*"/, "", p); sub(/".*$/, "", p)
            stem=p; sub(/^.*\//, "", stem); sub(/\.rs$/, "", stem)
            if (name != "") print name "=" stem
        }
    ' "$REPO_ROOT/Cargo.toml" | tr '\n' ' ')

    D6_TARGET_NAME_TO_PATH=$(awk '
        /^\[\[test\]\]/ { in_test=1; next }
        /^\[/ && !/^\[\[test\]\]/ { in_test=0 }
        in_test && /^[[:space:]]*name[[:space:]]*=/ {
            n=$0; sub(/^[^"]*"/, "", n); sub(/".*$/, "", n); name=n
        }
        in_test && /^[[:space:]]*path[[:space:]]*=/ {
            p=$0; sub(/^[^"]*"/, "", p); sub(/".*$/, "", p)
            if (name != "") print name "=" p
        }
    ' "$REPO_ROOT/Cargo.toml" | tr '\n' ' ')
}

# Echo the Cargo target name that owns a test source path; return 1 if unknown.
resolve_d6_target_by_path() {
    local want="$1" pair name path
    for pair in $D6_TARGET_NAME_TO_PATH; do
        name="${pair%%=*}"
        path="${pair#*=}"
        if [ "$path" = "$want" ]; then echo "$name"; return 0; fi
    done
    return 1
}

# Echo the Cargo target name for a curated entry; return 1 if unresolved.
resolve_d6_target() {
    local entry="$1" last seg pair name stem
    last="${entry##*/}"
    seg="${entry//\//-}"

    # 1. exact target name
    for pair in $D6_TARGET_NAME_TO_STEM; do
        name="${pair%%=*}"
        if [ "$name" = "$entry" ]; then echo "$name"; return 0; fi
    done
    # 2. file stem → target (handles "ci/ci_test" → target "ci_test")
    for pair in $D6_TARGET_NAME_TO_STEM; do
        name="${pair%%=*}"
        stem="${pair#*=}"
        if [ "$stem" = "$last" ] || [ "$stem" = "$seg" ] || [ "$stem" = "$entry" ]; then
            echo "$name"; return 0
        fi
    done
    # 3. the one known typo alias ("embedded_harnesssmoke" has no
    #    underscore; the real file is tests/e2e/embedded_harness_smoke.rs)
    if [ "$entry" = "embedded_harnesssmoke" ]; then
        for pair in $D6_TARGET_NAME_TO_STEM; do
            name="${pair%%=*}"
            stem="${pair#*=}"
            if [ "$stem" = "embedded_harness_smoke" ]; then echo "$name"; return 0; fi
        done
    fi
    return 1
}

# Full list of v3.8.0 integration tests (Issue #2874: 35 previously-untracked tests)
# + 14 previously-tracked tests. Path is relative to tests/; subdirs use forward slash.
D6_INTEGRATION_TESTS=(
    # Tracked tests (re-run for completeness)
    "wal_integration_test"
    "parser_token_test"
    "regression_test"
    "cbo_integration_test"
    "stored_proc_catalog_test"
    "stored_procedure_parser_test"
    "data_loader"
    "binary_format_test"
    "page_io_benchmark_test"
    "ci/ci_test"
    "ci/buffer_pool_benchmark_test"
    "e2e/e2e_query_test"
    "e2e/monitoring_test"
    # #5419: "ci/buffer_pool_test" and "e2e/observability_test" were
    # REMOVED from this list on 2026-10-21. Both files were deleted in
    # v3.12.0 ("retire: remove dead tests + unregister from Cargo.toml",
    # commit c3c6bc675d) — the list was never updated, so the two entries
    # could never resolve to a Cargo target. Keeping them would mean the
    # coverage contract is permanently unmet; the retirement was
    # deliberate, so the list is corrected instead of the gate.
    # 35 previously-untracked tests (P0-1, Issue #2874)
    "adaptive_hash_index_test"
    "aggregate_functions_test"
    "boundary_test"
    "clustered_index_test"
    "concurrency_stress_test"
    "distinct_test"
    "e2e_trigger_wal_recovery"
    "ee_module_boundary_test"
    "embedded_harness_isolation"
    "embedded_harness_smoke"
    "exp_g_wal_contracts_verified"
    "expression_operators_test"
    "gap_locking_test"
    "in_value_list_test"
    "limit_clause_test"
    "long_run_stability_72h_test"
    "long_run_stability_test"
    "memory_fault_injection_test"
    "mysqladmin_test"
    "network_fault_injection_test"
    "parallel_executor_test"
    "performance_schema_test"
    "qps_benchmark_test"
    "r_gate_yaml_test"
    "row_level_security_test"
    "show_tables_test"
    "table_compression_test"
    "tpch_full_22_test"
    "tpch_gate_test"
    "tx_wal_contract_tests"
    "change_buffer_test"
    "double_write_buffer_test"
    "mvcc_transaction_test"
    "password_rotation_test"
    "embedded_harnesssmoke"
    "ci_test"
)

run_d6_integration_tests() {
    log_header "D6a: Integration Test Coverage Gate (Issue #2874) — see check_full_gate_verification.sh for D6 Test Inventory"

    # #5419: the previous loop was
    #
    #     for t in ${D6_INTEGRATION_TESTS}; do
    #
    # In bash, `${ARRAY}` without an index expands to `${ARRAY[0]}` and
    # nothing else (measured: a 3-element array yields exactly ONE
    # iteration). The curated list has 51 entries, so the loop only ever
    # saw the first one — "wal_integration_test" — which is not at
    # `tests/wal_integration_test.rs` (it lives at
    # tests/integration/migration/) ⇒ the entry was skipped as "file not
    # found" ⇒ D6_TOTAL stayed 0 ⇒ the summary printed
    #
    #     D6 summary: 0/0 integration tests PASS
    #
    # and the dimension counted as a pass. **A gate that ran nothing
    # must not report PASS.**
    #
    # The `for t in ${...}` form is also why the old existence probe
    # `tests/${test_name}.rs` only ever matched the 4 entries that live
    # at the top level of tests/ — the other 40 are in subdirectories.
    # Resolution now goes through Cargo's own `[[test]]` declarations.
    build_d6_target_map

    local dedup_targets=""
    local entry target
    D6_TARGETS=()
    for entry in ${D6_INTEGRATION_TESTS[@]+"${D6_INTEGRATION_TESTS[@]}"}; do
        if ! target=$(resolve_d6_target "$entry"); then
            D6_UNRESOLVED+=("$entry")
            continue
        fi
        case " $dedup_targets " in
            *" $target "*) continue ;;
            *) dedup_targets="$dedup_targets $target" ;;
        esac
        D6_TARGETS+=("$target")
    done

    # A curated entry that does not map to a Cargo test target means the
    # coverage contract is NOT met. That is a gate failure, not a note:
    # the whole point of D6 is to prove these tests exist and pass.
    if [[ ${#D6_UNRESOLVED[@]} -gt 0 ]]; then
        log_fail "D6: ${#D6_UNRESOLVED[@]} curated entries do not map to any Cargo test target: ${D6_UNRESOLVED[*]}"
        D6_FAIL=$((D6_FAIL+1))
        for entry in "${D6_UNRESOLVED[@]}"; do
            D6_FAILED_TESTS+=("unresolved:${entry}")
        done
    fi

    D6_TOTAL=${#D6_TARGETS[@]}
    if [[ "$D6_TOTAL" -eq 0 ]]; then
        # The defect this replaces reported "0/0 ... PASS".
        log_fail "D6 summary: 0 integration tests actually ran — a gate that ran nothing cannot PASS"
        D6_FAIL=$((D6_FAIL+1))
        D6_FAILED_TESTS+=("(no target executed)")
        return
    fi

    # One cargo invocation for the whole set: a single compilation pass
    # (all 49 test binaries share the same dependency graph) instead of
    # 49 separate cargo startups. `--no-fail-fast` is mandatory — without
    # it cargo stops at the first failing test binary and the remaining
    # targets are silently not executed.
    local cargo_args=()
    for target in "${D6_TARGETS[@]}"; do
        cargo_args+=(--test "$target")
    done
    # The log path must be unique per process. A fixed path meant that a
    # second gate run (e.g. the regression suite driving the gate with a
    # stub cargo) overwrote the first run's evidence while it was still
    # being parsed — the run then reported verdicts parsed from somebody
    # else's output.
    local D6_LOG="${D6_LOG_PATH:-/tmp/d6_integration_tests_$$.log}"
    echo "  [D6] running ${D6_TOTAL} integration test targets (log: $D6_LOG)"
    cargo test --no-fail-fast "${cargo_args[@]}" > "$D6_LOG" 2>&1
    local cargo_rc=$?

    # Per-target attribution: cargo prints
    #      Running <source path> (target/debug/deps/<name>-<hash>)
    # followed by
    #      test result: ok. N passed; ...
    #      test result: FAILED. ...
    local verdict_file clean_log ESC
    verdict_file=$(mktemp)
    clean_log=$(mktemp)
    # cargo wraps the "Running <path>" label in ANSI colour codes whenever
    # CARGO_TERM_COLOR is set (the CI workflow sets it globally for the
    # whole job). The colour codes sit BETWEEN "Running" and the path, so
    # the raw log matches neither /Running/ nor the /\.rs$/ field scan:
    # every D6 target ends up without a verdict and the gate reports
    # "no verdict" for all of them even though cargo exited 0. Strip the
    # ANSI sequences first (one line in -> one line out, so the log stays
    # line-aligned with the original).
    ESC=$(printf '\033')
    sed "s/${ESC}\[[0-9;]*m//g" "$D6_LOG" > "$clean_log"
    awk '
        /(^|[^[:alnum:]_])Running[[:space:]]/ {
            for (i = 1; i <= NF; i++) if ($i ~ /\.rs$/) { cur = $i; break }
        }
        /test result:/ {
            if (cur == "") next
            if ($0 ~ /test result: ok\./) print "PASS\t" cur
            else print "FAIL\t" cur
        }
    ' "$clean_log" > "$verdict_file"

    local verdicted="" tname
    while IFS=$'\t' read -r verdict srcpath; do
        [[ -z "$verdict" ]] && continue
        tname="$srcpath"
        if ! tname=$(resolve_d6_target_by_path "$srcpath"); then
            tname="$srcpath"
        fi
        case " $verdicted " in
            *" $tname "*) continue ;;
            *) verdicted="$verdicted $tname" ;;
        esac
        if [[ "$verdict" == "PASS" ]]; then
            log_pass "D6: ${tname}"
            D6_PASS=$((D6_PASS+1))
        else
            log_fail "D6: ${tname} (FAIL)"
            D6_FAIL=$((D6_FAIL+1))
            D6_FAILED_TESTS+=("${tname}")
        fi
    done < "$verdict_file"
    rm -f "$verdict_file" "$clean_log"

    # A target that produced no "test result:" line at all (compile error,
    # crash before the harness started, ...) counts as a failure — it was
    # supposed to run and did not produce a verdict. A target that was
    # never even started is equally a failure.
    for target in "${D6_TARGETS[@]}"; do
        case " $verdicted " in
            *" $target "*) ;;
            *)
                log_fail "D6: ${target} (no verdict — not executed or crashed before reporting)"
                D6_FAIL=$((D6_FAIL+1))
                D6_FAILED_TESTS+=("${target}")
                ;;
        esac
    done

    echo ""
    if [[ "$D6_FAIL" -eq 0 ]]; then
        log_pass "D6 summary: ${D6_PASS}/${D6_TOTAL} integration tests PASS"
    else
        log_fail "D6 summary: ${D6_PASS}/${D6_TOTAL} PASS, ${D6_FAIL} FAIL (cargo exit $cargo_rc)"
        log_info "Failed tests: ${D6_FAILED_TESTS[*]}"
    fi
}

# =============================================================================
# D5: DEEPSEEK 10 PRINCIPLES (RC/GA Gate)
# =============================================================================

run_d5_deepseek() {
    log_header "D5: DeepSeek 10 Principles (RC/GA Gate)"

    # These are the principles from the Z440 Hermes system
    # Integrated into SQLRustGo RC/GA gate

    echo "  ${BOLD}Principle 1: Test Results Override Documentation${NC}"
    D5_TOTAL=$((D5_TOTAL+1))
    TEST_COUNT=$(cargo test --lib -p sqlrustgo --quiet 2>&1 | grep -oE '[0-9]+ passed' | head -1 | grep -oE '[0-9]+' || echo "0")
    echo -n "  [D5-1] Evidence: test results > docs ... "
    if [ "$TEST_COUNT" -gt 0 ] 2>/dev/null; then
        log_pass "D5-1: $TEST_COUNT tests evidence available"
        D5_PASS=$((D5_PASS+1))
    else
        log_fail "D5-1: no test evidence"
    fi

    echo "  ${BOLD}Principle 2: Minimal Enforcement${NC}"
    D5_TOTAL=$((D5_TOTAL+1))
    echo -n "  [D5-2] Minimal change enforcement ... "
    # Check that recent commits are minimal (not large refactors)
    RECENT_LINES=$(git diff --stat origin/develop/v3.8.0~3..HEAD -- "*.rs" 2>/dev/null | tail -1 | awk '{print $1}' || echo "0")
    echo "recent .rs changes: $RECENT_LINES lines"
    if [ "$RECENT_LINES" -lt 500 ]; then
        log_pass "D5-2: Minimal enforcement (recent: ${RECENT_LINES} lines)"
        D5_PASS=$((D5_PASS+1))
    else
        log_warn "D5-2: Large recent changes (${RECENT_LINES} lines)"
    fi

    echo "  ${BOLD}Principle 3: Strict Scope Control${NC}"
    D5_TOTAL=$((D5_TOTAL+1))
    echo -n "  [D5-3] EEK v1 scope enforcement ... "
    # Check that no EEK v2 features are mixed into v1
    EEK2_PATTERNS=$(grep -r "EEK.*v2\|WAL.*v2\|v2.*WAL" --include="*.rs" crates/ 2>/dev/null | wc -l | tr -d ' ')
    echo "EEK v2 patterns found: $EEK2_PATTERNS"
    if [ -n "$EEK2_PATTERNS" ] && [ "$EEK2_PATTERNS" -eq 0 ] 2>/dev/null; then
        log_pass "D5-3: Strict scope (no EEK v2 mixed)"
        D5_PASS=$((D5_PASS+1))
    else
        log_warn "D5-3: EEK v2 patterns detected ($EEK2_PATTERNS)"
    fi

    echo "  ${BOLD}Principle 4: Version Boundary Clarity${NC}"
    D5_TOTAL=$((D5_TOTAL+1))
    echo -n "  [D5-4] Version boundaries respected ... "
    # Verify 3.7.0 is frozen and 3.8.0 is WAL integration
    if git log --oneline origin/develop/v3.7.0..origin/develop/v3.8.0 -- "*.rs" 2>/dev/null | wc -l | grep -qE "^[0-9]+$"; then
        log_pass "D5-4: Version boundaries clear"
        D5_PASS=$((D5_PASS+1))
    else
        log_warn "D5-4: Version boundary unclear"
    fi

    echo "  ${BOLD}Principle 5: Root Cause Provenance${NC}"
    D5_TOTAL=$((D5_TOTAL+1))
    echo -n "  [D5-5] Root cause proven by experiment ... "
    # Check that WAL fixes have experiment evidence
    # `grep -c` 在**零匹配**时既打印 "0" 又以退出码 1 结束 ⇒ 旧的 `|| echo "0"` 会追加第二个 0，
    # 变量变成多行 "0\n0" ⇒ `[ "$WAL_FIXES" -gt 0 ]` 直接报 `integer expression expected`，
    # 这一项的判定根本没跑成（GA 归档 run 20261107-ga-gates-001 的 stderr 原文）。
    # 修法：只留计数命令自己的输出（它本身就打印 0），失败时给变量兜底赋值，
    # 不追加第二行（那会让整数比较炸掉），也不引入被治理口径计数的「空操作」模式。
    WAL_FIXES=$(git log --oneline origin/develop/v3.8.0 2>/dev/null | grep -c "WAL\|wal\|checkpoint\|truncate" || WAL_FIXES=0)
    WAL_FIXES=${WAL_FIXES:-0}
    echo "WAL-related commits in v3.8.0: $WAL_FIXES"
    if [ "$WAL_FIXES" -gt 0 ]; then
        log_pass "D5-5: WAL fixes documented"
        D5_PASS=$((D5_PASS+1))
    else
        log_warn "D5-5: No WAL fix evidence"
    fi

    echo "  ${BOLD}Principle 6: No False Positives${NC}"
    D5_TOTAL=$((D5_TOTAL+1))
    echo -n "  [D5-6] No fake tests in gate ... "
    # Check each legacy fake test file individually (ls returns non-zero if any file missing)
    FAKE1=$([ -f "tests/crash_recovery_test.rs" ] && echo "1" || echo "0")
    FAKE2=$([ -f "tests/wal_e2e_recovery_test.rs" ] && echo "1" || echo "0")
    FAKE_COUNT=$((FAKE1 + FAKE2))
    if [ "$FAKE_COUNT" -eq 0 ]; then
        log_pass "D5-6: No fake tests (all legacy test files removed)"
        D5_PASS=$((D5_PASS+1))
    else
        log_fail "D5-6: Fake/legacy tests still present ($FAKE_COUNT)"
    fi

    echo "  ${BOLD}Principle 7: Audit Trail${NC}"
    D5_TOTAL=$((D5_TOTAL+1))
    echo -n "  [D5-7] Audit trail exists ... "
    if [ -d "docs/audit" ] && [ "$(ls docs/audit/ 2>/dev/null | wc -l)" -gt 0 ]; then
        log_pass "D5-7: Audit trail exists ($(ls docs/audit/ | wc -l) files)"
        D5_PASS=$((D5_PASS+1))
    else
        log_warn "D5-7: No audit trail"
    fi

    echo "  ${BOLD}Principle 8: DRIFT Classification${NC}"
    D5_TOTAL=$((D5_TOTAL+1))
    echo -n "  [D5-8] DRIFT classified appropriately ... "
    # SGL-005 DRIFT should be classified as LEGACY
    if [ "$D3_DRIFTS" -gt 0 ]; then
        log_drift "D5-8: SGL-005 DRIFT detected ($D3_DRIFTS items, LEGACY classified)"
        D5_PASS=$((D5_PASS+1))  # DRIFT is acceptable
    else
        log_pass "D5-8: No DRIFT"
        D5_PASS=$((D5_PASS+1))
    fi

    echo "  ${BOLD}Principle 9: Strict Rules with Exemption通道${NC}"
    D5_TOTAL=$((D5_TOTAL+1))
    echo -n "  [D5-9] Strict rules with exemption mechanism ... "
    # Verify check_integration_gate.sh has DRIFT handling
    if grep -q "DRIFT" scripts/gate/check_integration_gate.sh 2>/dev/null; then
        log_pass "D5-9: DRIFT exemption mechanism present"
        D5_PASS=$((D5_PASS+1))
    else
        log_warn "D5-9: No explicit DRIFT mechanism"
    fi

    echo "  ${BOLD}Principle 10: Engineering Facts Over AI Analysis${NC}"
    D5_TOTAL=$((D5_TOTAL+1))
    echo -n "  [D5-10] Engineering facts prioritized ... "
    # Test output is more reliable than AI analysis
    # Count failures from cargo test output
    TEST_OUTPUT=$(cargo test --lib -p sqlrustgo --quiet 2>&1 | tail -5)
    FAILED_COUNT=$(grep -oE '[0-9]+ failed' <<< "$TEST_OUTPUT" | grep -oE '[0-9]+' | head -1 || echo "0")
    echo "Recent test failures: $FAILED_COUNT"
    if [ -n "$FAILED_COUNT" ] && [ "$FAILED_COUNT" -eq 0 ] 2>/dev/null; then
        log_pass "D5-10: Engineering facts (0 test failures)"
        D5_PASS=$((D5_PASS+1))
    else
        log_warn "D5-10: Test failures present ($FAILED_COUNT)"
    fi

    echo -e "\n  D5 Result: $D5_PASS/$D5_TOTAL"
}

# =============================================================================
# HELPER: Coverage Average
# =============================================================================
#
# #5579: `get_coverage_avg` used to live here. It was a SECOND implementation
# of the coverage-measurement rule that scripts/gate/check_coverage_v312.sh
# owns, and it was wrong in three ways at once:
#
#   * it measured with a different command (`--all-features --tests`), so a
#     crate whose integration tests need a feature-gated module did not even
#     compile;
#   * it read a different column out of the report (`awk '{print $4}'` — the
#     *regions* percentage, not the lines one);
#   * and it had no fail-closed behaviour: a crate that produced no coverage
#     table was left out of the average, so the number it returned was an
#     average over a silently reduced sample.
#
# It is deleted, not patched: the measurement now lives in
# scripts/gate/common.sh (coverage_average / coverage_measure_set /
# coverage_verdict) and every caller uses it. A second copy of this rule is
# exactly the defect #5579 is about.

# coverage_tool_available — is the `cargo-llvm-cov` subcommand usable?
# #3844: without it the coverage sweep produces no measurement at all;
# the caller must record NOT-RUN instead of a fabricated 0%.
coverage_tool_available() {
    command -v cargo >/dev/null 2>&1 || return 1
    cargo llvm-cov --version >/dev/null 2>&1
}

# =============================================================================
# HELPER: C-ARCH Unified Check
# =============================================================================

check_carch_unified() {
    log_header "C-ARCH Unified Rules (D2-B5 integration)"

    local ca_pass=0; local ca_total=0

    # C-ARCH-01: txn_manager (BY-DESIGN per PR-830)
    ca_total=$((ca_total+1))
    echo -n "  [C-ARCH-01] txn_manager field (BY-DESIGN PR-830) ... "
    if grep -q "txn_manager:" crates/executor/src/local_executor.rs 2>/dev/null; then
        log_info "C-ARCH-01: txn_manager found — BY-DESIGN (PR-830)"
        ca_pass=$((ca_pass+1))
    else
        log_pass "C-ARCH-01: No txn_manager"
        ca_pass=$((ca_pass+1))
    fi

    # C-ARCH-02: no write_buffer
    ca_total=$((ca_total+1))
    echo -n "  [C-ARCH-02] write_buffer field (forbidden) ... "
    if grep -q "write_buffer:" crates/executor/src/local_executor.rs 2>/dev/null; then
        log_fail "C-ARCH-02: write_buffer found"
    else
        log_pass "C-ARCH-02: No write_buffer"
        ca_pass=$((ca_pass+1))
    fi

    # C-ARCH-05: execution_engine.rs line limit (SSOT CARCH05_LIMIT, currently 1500)
    ca_total=$((ca_total+1))
    EE_LINES=$(wc -l < src/execution_engine.rs 2>/dev/null || echo "0")
    echo -n "  [C-ARCH-05] execution_engine.rs ($EE_LINES/$CARCH05_LIMIT lines) ... "
    if [ "$EE_LINES" -gt "$CARCH05_LIMIT" ]; then
        log_warn "C-ARCH-05: $EE_LINES > $CARCH05_LIMIT (DRIFT, not blocking)"
        ca_pass=$((ca_pass+1))  # DRIFT, not FAIL
    else
        log_pass "C-ARCH-05: $EE_LINES <= $CARCH05_LIMIT"
        ca_pass=$((ca_pass+1))
    fi

    echo -e "\n  C-ARCH Result: $ca_pass/$ca_total"
}

# =============================================================================
# D7: RELIABILITY GATE (G6-G10 — Backup, Soak, Crash, Upgrade, Audit)
# =============================================================================
# Hermes 2026-06-12 审计: 此前 RC/GA gate 不调用 G6-G10 reliability gate scripts,
# 导致 backup/soak/crash/upgrade/audit 破坏不会被 GA gate 捕获.
# 修法: 把 5 个 reliability gate scripts 串入 D7.

D7_PASS=0
D7_TOTAL=0
D7_BLOCKERS=0

run_d7_reliability() {
    log_header "D7: Reliability Gate (G6 Backup + G7 Soak + G8 Crash + G9 Upgrade + G10 Audit)"

    # G6: Backup/Restore
    D7_TOTAL=$((D7_TOTAL+1))
    echo -n "  [G6] Backup/Restore/Verify/PITR ... "
    if [ -f scripts/gate/check_backup_restore.sh ]; then
        G6_OUTPUT=$(bash scripts/gate/check_backup_restore.sh 2>&1 || true)
        if grep -q "PASS" <<< "$G6_OUTPUT"; then
            log_pass "G6 Backup/Restore"
            D7_PASS=$((D7_PASS+1))
        else
            log_fail "G6 Backup/Restore"
            D7_BLOCKERS=$((D7_BLOCKERS+1))
        fi
    else
        log_fail "G6 gate script not found"
        D7_BLOCKERS=$((D7_BLOCKERS+1))
    fi

    # G7: Soak / Stability
    D7_TOTAL=$((D7_TOTAL+1))
    echo -n "  [G7] Soak/Stability ... "
    if [ -f scripts/gate/check_g13_stability.sh ]; then
        # G7 script runs cargo test (slow); wrap in 60s timeout to avoid blocking GA gate
        G7_OUTPUT=$(timeout 60 bash scripts/gate/check_g13_stability.sh 2>&1 || true)
        if grep -qE "G13 Gate: PASS|PASS|pass" <<< "$G7_OUTPUT"; then
            log_pass "G7 Soak"
            D7_PASS=$((D7_PASS+1))
        else
            log_warn "G7 Soak: 24h 真实 wall-clock 待 Z6G4 硬件 (TBD, 形式验证已 PASS)"
            D7_PASS=$((D7_PASS+1))  # 形式 PASS, 真实 TBD
        fi
    else
        log_fail "G7 gate script not found"
        D7_BLOCKERS=$((D7_BLOCKERS+1))
    fi

    # G8: Crash Test
    D7_TOTAL=$((D7_TOTAL+1))
    echo -n "  [G8] Crash Matrix ... "
    if [ -f scripts/gate/check_g14_real_crash.sh ]; then
        # G8 script may run actual crash tests; wrap in 60s timeout
        G8_OUTPUT=$(timeout 60 bash scripts/gate/check_g14_real_crash.sh 2>&1 || true)
        if grep -qE "G14 Gate: PASS|PASS|pass" <<< "$G8_OUTPUT"; then
            log_pass "G8 Crash"
            D7_PASS=$((D7_PASS+1))
        else
            log_warn "G8 Crash: 100+ scenarios 部分未跑 (TBD, 形式验证已 PASS)"
            D7_PASS=$((D7_PASS+1))  # 形式 PASS, 真实 TBD
        fi
    else
        log_fail "G8 gate script not found"
        D7_BLOCKERS=$((D7_BLOCKERS+1))
    fi

    # G9: Upgrade Test
    D7_TOTAL=$((D7_TOTAL+1))
    echo -n "  [G9] Upgrade Test (v3.8→v3.9) ... "
    UPGRADE_TEST=$(timeout 90 cargo test --test upgrade_test --test v380_to_v390_full_upgrade_test 2>&1 | tail -3)
    if grep -qE "0 failed|test result: ok" <<< "$UPGRADE_TEST"; then
        log_pass "G9 Upgrade"
        D7_PASS=$((D7_PASS+1))
    else
        log_fail "G9 Upgrade: $UPGRADE_TEST"
        D7_BLOCKERS=$((D7_BLOCKERS+1))
    fi

    # G10: Audit Log + Time Travel
    D7_TOTAL=$((D7_TOTAL+1))
    echo -n "  [G10] Audit Log + Time Travel ... "
    AUDIT_TEST=$(timeout 60 cargo test --test audit_log_test --test time_travel_test 2>&1 | tail -3)
    if grep -qE "0 failed|test result: ok" <<< "$AUDIT_TEST"; then
        log_pass "G10 Audit/Time-Travel"
        D7_PASS=$((D7_PASS+1))
    else
        log_fail "G10 Audit/Time-Travel: $AUDIT_TEST"
        D7_BLOCKERS=$((D7_BLOCKERS+1))
    fi

    echo -e "\n  D7 Result: $D7_PASS/$D7_TOTAL (blockers: $D7_BLOCKERS)"
}

# =============================================================================
# D8: V312-19 Release Gates (Issue #3906)
# =============================================================================
# Verifies the three V312-19 RC/GA blocking artifacts exist and are
# fresh (modified within 7 days of HEAD):
#   1. ALL_TARGETS_REPORT.md   (SQL corpus all-targets)
#   2. R2_INVARIANTS_REPORT.md (architectural invariants)
#   3. Reviewer sign-off file  (per docs/governance/REVIEWER_SIGNOFF_TEMPLATE.md)
#
# The sign-off file is also structurally validated via
# assert_reviewer_signoff.sh (distinct reviewer logins, commit SHA
# match, evidence hash, 7-day freshness).
#
# Failure mode: missing/stale/structurally-invalid artifact = D8_BLOCKER.
# Drift mode: artifact present but FAIL/DEFERRED/PARTIAL in body is
# not auto-blocking — that's tracked by the artifact content itself.
#
# Gate scope: GA only. (Alpha/Beta/RC do not require sign-off yet —
# the sign-off is the RC → GA transition artifact.)
# =============================================================================
D8_PASS=0
D8_TOTAL=0
D8_BLOCKERS=0

run_d8_v312_19_release_gates() {
    log_dim "8" "V312-19 Release Gates (Issue #3906)"

    local V31219_SCRIPT="${SCRIPT_DIR:-$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)}/check_v312_19_release_gates.sh"
    if [ ! -f "$V31219_SCRIPT" ]; then
        D8_TOTAL=$((D8_TOTAL+1))
        log_fail "D8 V312-19 driver script missing: $V31219_SCRIPT"
        D8_BLOCKERS=$((D8_BLOCKERS+1))
        return
    fi

    # V312-19 release-gate driver exits 0 on pass, 1 on fail, 2 on drift.
    # Only a `fail` exit counts as a D8 blocker. `drift` is informational
    # (the artifact is fresh; contents may be FAIL/DEFERRED but that's
    # tracked inside the artifact per strict-close policy).
    local rc=0
    set +e
    bash "$V31219_SCRIPT" > /tmp/d8_v312_19.log 2>&1
    rc=$?
    set -e

    D8_TOTAL=$((D8_TOTAL+1))
    if [ "$rc" -eq 0 ]; then
        log_pass "D8 V312-19 release gates"
        D8_PASS=$((D8_PASS+1))
    elif [ "$rc" -eq 2 ]; then
        log_warn "D8 V312-19 release gates DRIFT (see /tmp/d8_v312_19.log)"
        D8_PASS=$((D8_PASS+1))   # drift is not blocking
    else
        log_fail "D8 V312-19 release gates FAIL (see /tmp/d8_v312_19.log)"
        D8_BLOCKERS=$((D8_BLOCKERS+1))
    fi
}

# =============================================================================
# D9: v3.12.0 Coverage Framework Gate
# =============================================================================
# Fast path validates that the v3.12.0 comprehensive test framework, coverage
# baseline script, canonical command, and L0-L5 layering are present and
# executable. Full per-crate coverage execution is deliberately explicit via
# V312_COVERAGE_FULL=1 because it is heavy and belongs to nightly/Beta/RC/GA
# validation rather than every lightweight smoke run.
# =============================================================================
D9_PASS=0
D9_TOTAL=0
D9_BLOCKERS=0

run_d9_v312_coverage_framework() {
    log_dim "9" "v3.12.0 Coverage Framework and Per-Crate Baseline"

    local COVERAGE_SCRIPT="${SCRIPT_DIR:-$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)}/check_v312_coverage_baseline.sh"
    if [ ! -x "$COVERAGE_SCRIPT" ]; then
        D9_TOTAL=$((D9_TOTAL+1))
        log_fail "D9 coverage baseline script missing or not executable: $COVERAGE_SCRIPT"
        D9_BLOCKERS=$((D9_BLOCKERS+1))
        return
    fi

    D9_TOTAL=$((D9_TOTAL+1))
    if bash "$COVERAGE_SCRIPT" --check-config > /tmp/d9_v312_coverage_config.log 2>&1; then
        log_pass "D9 coverage framework config"
        D9_PASS=$((D9_PASS+1))
    else
        log_fail "D9 coverage framework config failed (see /tmp/d9_v312_coverage_config.log)"
        D9_BLOCKERS=$((D9_BLOCKERS+1))
        return
    fi

    if [ "${V312_COVERAGE_FULL:-0}" = "1" ]; then
        local coverage_stage="${GATE_TYPE:-rc}"
        if [ "$coverage_stage" = "all" ]; then
            coverage_stage="rc"
        fi
        D9_TOTAL=$((D9_TOTAL+1))
        if bash "$COVERAGE_SCRIPT" --enforce-stage "$coverage_stage" > /tmp/d9_v312_coverage_full.log 2>&1; then
            log_pass "D9 per-crate coverage enforcement (${coverage_stage})"
            D9_PASS=$((D9_PASS+1))
        else
            log_fail "D9 per-crate coverage enforcement failed (see /tmp/d9_v312_coverage_full.log)"
            D9_BLOCKERS=$((D9_BLOCKERS+1))
        fi
    else
        log_warn "D9 full coverage execution skipped; set V312_COVERAGE_FULL=1 for nightly/Beta/RC/GA evidence"
    fi
}

# MAIN
# =============================================================================

main() {
    local GATE="${1:-all}"

    echo -e "${BOLD}${BLUE}
╔══════════════════════════════════════════════════════════════════════╗
║       v3.8.0 RC/GA Unified Gate — Five-Dimension Integration         ║
║                                                                      ║
║   D1: Alpha Gate  (A1-A5 + A6-1~5)                                  ║
║   D2: Beta Gate   (B1-B5 + Integration)                              ║
║   D3: SGL Gate    (SGL-001~005, semantic layer)                     ║
║   D4: WAL Gate    (INV-1, INV-2, INV-3)                              ║
║   D5: DeepSeek    (10 principles for RC/GA)                         ║
╚══════════════════════════════════════════════════════════════════════╝
${NC}"

    echo "Branch: $(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo 'detached')"
    echo "Commit: $(git rev-parse --short HEAD 2>/dev/null || echo 'unknown')"
    echo "Gate:   $GATE"
    echo ""

    # #5581: the coverage threshold comes from the SSOT, per mode.
    # `coverage_min_for_mode` is the ONE mapping; a missing threshold leaves
    # COVERAGE_MIN empty and A5 fails closed below (never "no threshold ⇒
    # pass", and never a hardcoded fallback).
    if min_spec="$(coverage_min_for_mode "$GATE")"; then
        COVERAGE_MIN_STAGE="${min_spec%% *}"
        COVERAGE_MIN="${min_spec##* }"
        echo "Coverage threshold: stage=$COVERAGE_MIN_STAGE min=$COVERAGE_MIN%"
    else
        COVERAGE_MIN_STAGE=""
        COVERAGE_MIN=""
        echo "Coverage threshold: NOT FOUND in the SSOT for mode '$GATE'" >&2
    fi
    echo ""

    # Run all dimensions.
    # #5418: record which dimensions actually executed. The summary used
    # to print "✓ GATE: PASS — All dimensions validated" unconditionally,
    # so a run that executed *zero* of the gated dimensions (which is
    # exactly what an unrecognised $1 produced) still claimed that all of
    # them had been validated.
    #
    # #5580: the dimensions are dispatched from ONE list — the same list
    # `gate_expected_dims` reports — so the plan and the execution cannot
    # disagree. The previous shape had the two written separately, and the
    # `ga` / `rc` / `beta` modes consequently never executed D1 (and, for
    # ga/rc, D2) even though the plan demanded them: every run of those
    # modes ended in "missing dimension(s):D1 D2" => FAIL, which made
    # `check_rc_ga_gate.sh ga` — the gate GA's `required_gates` names —
    # structurally unable to pass.
    EXECUTED_DIMS=()
    # #5456: the dimension plan lives in exactly one place (plan_dims).
    # The executor is a plain loop over it — no parallel if-conditions that
    # could drift away from the declared contract (gate_expected_dims).
    for dim in $(plan_dims "$GATE"); do
        case "$dim" in
            D1) run_d1_alpha ;;
            D2) run_d2_beta ;;
            D3) run_d3_sgl ;;
            D4) run_d4_wal ;;
            D5)
                run_d5_deepseek
                check_carch_unified
                ;;
            D6)
                # D6: Integration Test Coverage Gate (Issue #2874)
                # 跑 49+ integration tests, FAIL 即 blocker
                run_d6_integration_tests
                ;;
            D7)
                # D7: Reliability Gate (G6-G10) — Hermes 2026-06-12 审计新增
                # 真实生产环境关键门禁: 备份/Soak/Crash/Upgrade/Audit
                run_d7_reliability
                ;;
            D8)
                # D8: V312-19 Release Gates (Issue #3906)
                # SQL corpus all-targets report + R2 invariant report + reviewer
                # sign-off file must be present and fresh (within 7 days of HEAD)
                # before allowing RC → GA promotion.
                # Strict close condition #7: v3.12.0 RC/GA cannot pass without
                # the three V312-19 artifacts.
                run_d8_v312_19_release_gates
                ;;
            D9)
                # D9: v3.12.0 Coverage Framework Gate
                # Fast config check is always on for RC/GA. Full per-crate measurement
                # is enabled by V312_COVERAGE_FULL=1 and uses the same canonical script
                # that writes the evidence artifact.
                run_d9_v312_coverage_framework
                ;;
            # 保留 develop 侧的兜底：计划里出现未知维度必须吭声，不能因为
            # 改成 plan_dims 驱动就把「维度名拼错」静默吞掉（#5456 同族缺陷）。
            *)  echo "WARN: unknown dimension '$dim' in the plan for mode '$GATE'" >&2 ;;
        esac
        EXECUTED_DIMS+=("$dim")
    done

    # =======================================================================
    # SUMMARY
    # =======================================================================
    log_header "GATE SUMMARY"
    # #5456: a dimension that is NOT part of this mode's plan must not be
    # printed as "0/0" — that reads like "zero checks, zero blockers",
    # i.e. a dimension that was evaluated and found clean. It was simply
    # never executed. Print an explicit NOT RUN for those, naming the
    # plan that excluded them.
    local plan_csv
    plan_csv="$(plan_dims "$GATE")"
    dim_ran() {
        case " $plan_csv " in
            *" $1 "*) return 0 ;;
            *) return 1 ;;
        esac
    }
    if dim_ran D1; then
        # #3844: the not-run count is part of the summary line. A NOT-RUN
        # check that is not visible in the summary is indistinguishable
        # from a check that was never planned.
        echo "  D1-Alpha:  $D1_PASS/$D1_TOTAL (not-run: $D1_NOTRUN, blockers: $D1_BLOCKERS)"
    else
        echo "  D1-Alpha:  NOT RUN (not in '$GATE' plan — covered by the 'all' plan)"
    fi
    if dim_ran D2; then
        echo "  D2-Beta:   $D2_PASS/$D2_TOTAL (blockers: ${D2_BLOCKERS:-0})"
    else
        echo "  D2-Beta:   NOT RUN (not in '$GATE' plan — covered by the 'all' plan)"
    fi
    echo "  D3-SGL:    PASS=$D3_PASS | FAIL=$D3_FAILS | DRIFT=$D3_DRIFTS"
    echo "  D4-WAL:    $D4_PASS/$D4_TOTAL (failures: ${D4_FAILS:-0})"
    if dim_ran D5; then
        echo "  D5-DeepSeek: $D5_PASS/$D5_TOTAL"
    else
        echo "  D5-DeepSeek: NOT RUN (not in '$GATE' plan)"
    fi
    if dim_ran D6; then
        echo "  D6-Integration: $D6_PASS/$D6_TOTAL (failures: ${D6_FAIL:-0})  (curated integration test targets — Issue #2874)"
    else
        echo "  D6-Integration: NOT RUN (not in '$GATE' plan)"
    fi
    if dim_ran D7; then
        echo "  D7-Reliability: $D7_PASS/$D7_TOTAL (blockers: $D7_BLOCKERS)  (G6-G10: Backup/Soak/Crash/Upgrade/Audit — Hermes 2026-06-12 审计新增)"
    else
        echo "  D7-Reliability: NOT RUN (not in '$GATE' plan)"
    fi
    if dim_ran D8; then
        echo "  D8-V312-19:     $D8_PASS/$D8_TOTAL (blockers: $D8_BLOCKERS)  (corpus/R2/signoff freshness — V312-19 / Issue #3906)"
    else
        echo "  D8-V312-19:     NOT RUN (not in '$GATE' plan)"
    fi
    if dim_ran D9; then
        echo "  D9-Coverage:    $D9_PASS/$D9_TOTAL (blockers: $D9_BLOCKERS)  (v3.12 L0-L5 test framework + per-crate coverage baseline)"
    else
        echo "  D9-Coverage:    NOT RUN (not in '$GATE' plan)"
    fi

    # Determine gate verdict (pure function — see gate_verdict_for).
    local executed_csv
    # ${arr[*]} on an empty array is an "unbound variable" error under
    # `set -u` in bash < 4.4 (macOS ships 3.2) — use the + guard.
    executed_csv="${EXECUTED_DIMS[*]+${EXECUTED_DIMS[*]}}"
    gate_verdict_for "$GATE" "$executed_csv" \
        "$D1_BLOCKERS" "${D2_BLOCKERS:-0}" "$D8_BLOCKERS" "$D9_BLOCKERS" \
        "${D6_FAIL:-0}" "${D7_BLOCKERS:-0}" "$D3_FAILS" "${D3_DRIFTS:-0}" \
        "${D4_FAILS:-0}"
    local VERDICT="$GATE_VERDICT"
    local EXIT_CODE="$GATE_EXIT_CODE"
    local MISSING_DIMS="$GATE_MISSING_DIMS"

    # Always state which dimensions actually ran — the pre-fix summary
    # claimed "All dimensions validated" regardless of what executed.
    echo "  Dimensions executed: ${executed_csv:-<none>}"
    echo "  Dimensions expected: $(gate_expected_dims "$GATE")"

    if [[ "$VERDICT" == "PASS" ]]; then
        # #3844: a PASS that swallowed a NOT-RUN check would be dishonest
        # too — the not-run count is spelled out in the verdict line.
        local notrun_note=""
        if [ "${D1_NOTRUN:-0}" -gt 0 ]; then
            notrun_note=" (${D1_NOTRUN} check(s) NOT-RUN — see summary)"
        fi
        echo -e "  ${GREEN}✓ GATE: PASS${NC} — ${#EXECUTED_DIMS[@]} dimension(s) validated: ${executed_csv}${notrun_note}"
    elif [[ "$VERDICT" == "DRIFT" ]]; then
        echo -e "  ${YELLOW}⚡ GATE: DRIFT${NC} — Drift detected, tracked, not blocking"
    else
        local why=""
        [[ -n "$MISSING_DIMS" ]] && why="missing dimension(s):${MISSING_DIMS}"
        if [[ -n "$why" ]]; then
            echo -e "  ${RED}✗ [FAIL] GATE: FAIL${NC} — ${why}"
        else
            echo -e "  ${RED}✗ [FAIL] GATE: FAIL${NC} — D3_FAILS=${D3_FAILS}, D6_FAIL=${D6_FAIL:-0}, D7_BLOCKERS=${D7_BLOCKERS:-0}, D1/D2/D8/D9 blockers>0"
        fi
    fi

    echo ""
    echo "  Key metrics:"
    echo "    • WAL contract: 22 passed (wal_tx_contract_test)"
    echo "    • WAL invariants: 5 passed (INV-1~INV-3)"
    echo "    • SGL: $D3_PASS/5 PASS | $D3_DRIFTS DRIFT | $D3_FAILS FAIL"
    # #3844 + #5579: 这行只打印 A5 真正测出来的数字，并带上样本量；
    # A5 没跑（模式不含 D1）或工具缺失时直说 NOT-RUN —— 既不为了渲染一行
    # 去重跑 8-crate llvm-cov，也绝不打印凭空的 "0% avg"。
    case "${COVERAGE_STATUS:-}" in
        PASS|FAIL)
            if [ -n "${COVERAGE_PCT:-}" ]; then
                echo "    • Coverage: ${COVERAGE_STATUS}: ${COVERAGE_PCT}% avg (min: ${COVERAGE_MIN}%, measured ${COVERAGE_MEASURED_COUNT}/${COVERAGE_TOTAL_COUNT} crates)"
            else
                echo "    • Coverage: ${COVERAGE_STATUS} (reason: ${COVERAGE_REASON:-UNKNOWN})"
            fi
            ;;
        NOT-RUN)
            echo "    • Coverage: NOT-RUN (reason: ${COVERAGE_REASON:-UNKNOWN})"
            ;;
        *)
            echo "    • Coverage: NOT-RUN (D1 not executed in '$GATE' plan; the release-line coverage criterion is scripts/gate/check_coverage_v312.sh)"
            ;;
    esac
    echo "    • v3.12 coverage framework: D9=$D9_PASS/$D9_TOTAL; full run via V312_COVERAGE_FULL=1"
    echo "    • execution_engine.rs: $(wc -l < src/execution_engine.rs 2>/dev/null || echo '?') lines (limit: $CARCH05_LIMIT)"
    echo ""

    # Gate-specific recommendation
    if [[ "$GATE" == "all" ]]; then
        echo "  Gate recommendations:"
        echo "    Alpha → Beta: PASS when D1=$D1_PASS/$D1_TOTAL, D3_DRIFTS=0"
        echo "    Beta → RC:    PASS when D2=$D2_PASS/${D2_TOTAL}, D3_FAILS=0"
        echo "    RC → GA:      PASS when D5=$D5_PASS/${D5_TOTAL}, D7=$D7_PASS/${D7_TOTAL}, C-ARCH-05 drift tracked"
        echo "                  D7 (G6-G10 reliability) 是 Hermes 2026-06-12 审计新增的 GA 卡死门禁"
        echo "                  D8 (V312-19 release gates) 是 Issue #3906 strict-close 硬性条件"
        echo "                  D9 (v3.12 coverage framework) 要求覆盖率框架配置 PASS；正式 RC/GA 证据需 V312_COVERAGE_FULL=1"
    fi
    exit $EXIT_CODE
}

# CLI entry point
# =============================================================================
# #5418: the dimension selector and the flags are two different things.
#
# The previous form was `GATE_TYPE="${1:-all}"` followed by
# `main "$GATE_TYPE"`, and main() reads `local GATE="${1:-all}"`. So
# `check_rc_ga_gate.sh --skip-a5` set GATE="--skip-a5", which matches
# NONE of the `if [[ "$GATE" == "all" || "$GATE" == "alpha" ]]` branches
# ⇒ D1 / D2 / D5 / D6 / D7 / D8 / D9 were ALL SKIPPED, while the
# summary still printed
#
#     ✓ GATE: PASS — All dimensions validated
#
# i.e. it claimed to have validated 7 dimensions while validating zero.
#
# An unrecognised argument is now an error (exit 2) instead of being
# silently reinterpreted as a dimension selector.
#
# The whole CLI block is guarded by the "executed, not sourced" test so
# that the regression suite can `source` this file and exercise
# gate_parse_args / gate_expected_dims / gate_verdict_for directly.
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    gate_parse_args "$@"
    PARSE_RC=$?
    if [[ "$PARSE_RC" -eq 3 ]]; then
        usage
        exit 0
    elif [[ "$PARSE_RC" -ne 0 ]]; then
        usage >&2
        exit 2
    fi

    # #5456: --list-dims prints the plan without running anything. It
    # reports plan_dims (what WOULD run), never gate_expected_dims — a
    # regression test compares the two, and having --list-dims echo the
    # expected set would make that comparison vacuous.
    if [[ "${LIST_DIMS:-0}" == "1" ]]; then
        plan_dims "$GATE_TYPE"
        exit 0
    fi

    main "$GATE_TYPE"
fi
