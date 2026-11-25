#!/usr/bin/env bash
# fx18-coverage-v312-crate-coverage-below-threshold.sh — negative fixture for
# scripts/gate/check_coverage_v312.sh
# fixture-id: fx18-coverage-v312-crate-coverage-below-threshold
# expect: red
#
# WHAT CHANGED AND WHY THIS IS NOW A RED FIXTURE
# ------------------------------------------------
# The fixture used to be `expect: inconclusive` because the coverage gate was
# ALREADY RED on the intact tree -- `sqlrustgo-storage` produced no coverage
# table at all, because the RED characterisation tests imported by the
# readiness plan's earlier tasks
# (crates/storage/tests/tf_t4_ai_candidate_v1.rs, tf_t4_stub_silent_success.rs)
# fail BY DESIGN, so `cargo llvm-cov` exited 101 and the L1_8 sample was 7 of 8.
# The gate's red therefore existed before this fixture injected anything, and
# the red could not be attributed to the injection.
#
# That baseline red has now been REMOVED AT THE MEASUREMENT LAYER ONLY:
# `scripts/gate/common.sh` gained `COVERAGE_RED_CHARACTERISATION_TEST_FILES`,
# which excludes those two files' test functions from the coverage
# measurement (libtest `--skip`), so `sqlrustgo-storage` measures normally
# and the L1_8 sample is 8 of 8.
#
# WHAT THE EXCLUSION DOES NOT DO (R7):
#
#   * it does NOT turn the RED characterisation tests green -- they still run
#     and still fail (RED-LEDGER #5913/#5914);
#   * it does NOT relax any threshold;
#   * it does NOT touch `coverage_verdict` -- any other reason for a crate
#     being unmeasurable still FAILS the gate and still names the crate.
#
# So the control arm below must now be GREEN on the intact tree, and the
# mutation arm's red IS attributable to the injection.  If the control arm
# is not green, the fixture reports INCONCLUSIVE (exit 4) -- it never claims a
# red it cannot attribute.
#
# THE STAGE BAR IS PINNED TO `ga` (85), EXPLICITLY
# -----------------------------------------------
# This fixture measures the gate at the **GA** bar, not the ALPHA bar:
# `SQLRUSTGO_COVERAGE_STAGE=ga` is exported below and the gate output is
# asserted to contain `stage=ga`.  Two reasons:
#
#   * at the ALPHA bar (50) the target's mutation is arithmetically unable to
#     reach the threshold: the other seven crates already average ~86.64%, so
#     the L1_8 average cannot fall below 50 by changing ONE crate -- it would
#     need a negative coverage;
#   * the number that matters is the GA bar, because
#     `check_coverage_v312.sh` defaults to `ga` when the caller says nothing
#     (the strictest bar, by #5581's design) -- `scripts/gate/check_stage.sh`
#     is the only caller that overrides it, and it overrides it to the
#     DETECTED stage (alpha) for the ALPHA stage list.
#
# The `stage=ga` assertion pins WHICH bar the measurement was made at; it
# does not copy the threshold number (the SSOT owns the number).
#
# fault: replace one crate's coverage table with a value below the threshold
#        — sqlrustgo-transaction's measured line coverage is driven down by
#        adding code that no test ever executes, so its coverage table reads
#        a value far below the stage bar and the L1_8 average falls under it.
#
# WHY sqlrustgo-transaction
# -------------------------
# The L1_8 crate set is measured with one `cargo llvm-cov test -p <crate>`
# per crate, so a mutation inside one crate invalidates that crate's own
# instrumented build.  `sqlrustgo-transaction` is the only member of the set
# that **no other member depends on** (see `cargo metadata`: planner and
# optimizer depend on parser/storage/types, executor depends on
# catalog/parser/planner/storage/types), so mutating it re-compiles one
# crate instead of the whole instrumented chain.  That is what keeps the
# fixture inside its budget.
#
# WHY "add uncovered code" and not "edit a number"
# ------------------------------------------------
# The coverage table is not a file this repository owns: it is the TOTAL
# row of the `cargo llvm-cov` report for the crate, produced by the
# measurement itself.  There is nothing to "replace" textually — the only
# honest way to make a crate's table read a low number is to make the code
# uncovered.  The fixture therefore appends N public functions that no test
# calls, which drives the crate's line coverage down to a few percent and
# the L1_8 average below the stage bar.
#
# The gate must then say **"average X% < threshold Y%"** — the failure mode
# the ledger's target names ("把某个 crate 的覆盖率表替换成低于阈值的值 ⇒
# 必须变红").  A "measured N of 8 crates" verdict (the reduced-sample
# failure) would be a different failure and is rejected by this fixture.
#
# Cost: the gate measures eight crates with `cargo llvm-cov`, which is the
# expensive lane of this ledger.  A run that hits the budget is reported as
# INCONCLUSIVE, never as a red (R50).  The budget below is 3600s per arm
# (the archived runs recorded rc=124 at 1800s and at 5400s before the lane
# was warmed; the warm target plus the 3600s budget fits the measurement).
#
# Part of the v4.1.0 readiness plan (Task 8).  NOT a gate: it never runs in
# ci_gate.sh, never changes the gate denominator (168).  It mutates a
# **scratch** checkout, runs the real gate there, and the scratch tree is
# restored afterwards.

set -uo pipefail

T="${1:?usage: fx18-coverage-v312-crate-coverage-below-threshold.sh <scratch-tree>}"
FX_ID="fx18-coverage-v312-crate-coverage-below-threshold"
GATE="scripts/gate/check_coverage_v312.sh"
TARGET="crates/transaction/src/lib.rs"
CRATE="sqlrustgo-transaction"
# How much never-executed code to append.  The crate's own lines are in the
# low thousands, so this drives its coverage into single digits and the
# L1_8 average from ~87.8% to well under the 85% GA bar.
PAD=8000

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=audit/v4.1.0/neg/neg_two_layer.sh
. "$HERE/neg_two_layer.sh"

# restore() is the shared one (neg_two_layer.sh): it puts the scratch tree back
# and does not delete the evidence run directory this fixture belongs to.
restore() {
    fx_restore "$T"
}
restore

if [ ! -f "$T/$TARGET" ]; then
    echo "$FX_ID: $TARGET not found in the scratch tree -> INCONCLUSIVE"
    exit 4
fi

# The threshold bar is PINNED to `ga` (the strictest bar) for BOTH arms.
#
# `check_coverage_v312.sh` reads `SQLRUSTGO_COVERAGE_STAGE` and defaults to
# `ga` when it is unset — but "unset" is not a decision, it is an absence of
# one, and the caller must be able to see which bar the measurement was made
# at.  Exporting it explicitly here makes the bar part of the fixture's
# contract instead of an accident of the gate's default.  The threshold
# NUMBER is not copied into this file: the SSOT owns it
# (docs/governance/STAGE_CONFIG.yaml → thresholds → COVERAGE_MIN_GA).
export SQLRUSTGO_COVERAGE_STAGE=ga

# --- control: what does the gate say on the INTACT tree? ---------------------
# The control arm decides the fixture's verdict: if the gate is already red
# without the defect, the defect cannot be attributed to the mutation and the
# fixture must report INCONCLUSIVE (R50), not a red.
#
# The baseline red that used to make this arm red is gone: the RED-by-design
# characterisation tests are excluded from the coverage MEASUREMENT (not from
# the repository), so `sqlrustgo-storage` measures and the L1_8 sample is 8 of
# 8.  The control arm must therefore be GREEN (rc=0) on the intact tree.
echo "----- fx18 control (intact tree) -----"
CTL_LOG="$(mktemp)"
CTL_T0=$(date +%s)
( cd "$T" && timeout 3600 bash "$GATE" ) >"$CTL_LOG" 2>&1
CTL_RC=$?
CTL_T1=$(date +%s)
echo "control rc=$CTL_RC in $((CTL_T1-CTL_T0))s"
tail -4 "$CTL_LOG"
CTL_MUT_PCT="$(grep -m1 -E "$CRATE: [0-9]+\.[0-9]+%" "$CTL_LOG" | grep -oE '[0-9]+\.[0-9]+' || true)"
echo "$FX_ID: control: $CRATE coverage = ${CTL_MUT_PCT:-<none>}%, gate rc=$CTL_RC"

# The stage bar is PINNED, not inherited: the control arm must have measured
# at `stage=ga`.  This asserts WHICH bar, not the threshold number — the
# threshold's number lives in the SSOT and is deliberately not copied here.
if ! grep -q 'stage=ga' "$CTL_LOG"; then
    echo "$FX_ID: the control arm did not report 'stage=ga' -> INCONCLUSIVE (the threshold bar must be pinned)"
    grep -m1 'Coverage criterion' "$CTL_LOG" || true
    rm -f "$CTL_LOG"
    restore
    exit 4
fi
rm -f "$CTL_LOG"

# --- inject the fault --------------------------------------------------------
python3 - "$T/$TARGET" "$PAD" <<'PY'
import sys
path, pad = sys.argv[1], int(sys.argv[2])
src = open(path, encoding="utf-8").read()
chunk = ["\n// fx18: never-executed padding — drives this crate's coverage table down.\n"]
for i in range(pad):
    chunk.append("pub fn fx18_uncovered_%05d(x: i64) -> i64 { x.wrapping_mul(31).wrapping_add(%d) }\n" % (i, i))
open(path, "w", encoding="utf-8").write(src + "".join(chunk))
PY
if [ $? -ne 0 ]; then
    echo "$FX_ID: could not inject the fault -> INCONCLUSIVE"
    restore
    exit 4
fi
echo "$FX_ID: appended $PAD never-executed functions to $TARGET"

# --- run the real gate through the tri-state runner --------------------------
# The budget is the one this ledger sets (issue #5598), raised to 3600s per
# arm: the gate measures eight crates with `cargo llvm-cov`
# (single-threaded, instrumented) and the archived runs recorded rc=124 at
# 1800s.  A run that hits the budget is reported as INCONCLUSIVE, never as a
# red (R50): a timeout is not a drift.
fx_two_layer_run "$T" "$GATE" "$FX_ID" 3600
TWO_LAYER_RC=$?
echo "----- fx18 gate output (tail 30) -----"
tail -30 "$FX_RUN_DIR"/001-*.stdout 2>/dev/null || true
echo "----- end -----"

if [ "$TWO_LAYER_RC" -ne 0 ]; then
    echo "$FX_ID: could not run the two-layer check -> INCONCLUSIVE"
    restore
    rm -rf "$FX_RUN_DIR"
    exit 4
fi

GATE_TAIL="$(cat "$FX_RUN_DIR"/001-*.stdout 2>/dev/null || true)"
GATE_ERR="$(cat "$FX_RUN_DIR"/001-*.stderr 2>/dev/null || true)"

# the failure mode must be the AVERAGE under the threshold, not a reduced sample
AVERAGE_FAIL=0
case "$GATE_TAIL$GATE_ERR" in
    *"average"*"< threshold"*) AVERAGE_FAIL=1 ;;
esac
REDUCED_SAMPLE=0
case "$GATE_TAIL$GATE_ERR" in
    *"measured "*" of "*" crates"*) REDUCED_SAMPLE=1 ;;
esac
# the mutated crate's own number, read out of the per-crate detail
MUT_PCT="$(printf '%s\n' "$GATE_TAIL" | grep -m1 -E "$CRATE: [0-9]+\.[0-9]+%" | grep -oE '[0-9]+\.[0-9]+' || true)"
echo "$FX_ID: mutated crate percentage=${MUT_PCT:-<none>} average_fail=$AVERAGE_FAIL reduced_sample=$REDUCED_SAMPLE"

restore
rm -rf "$FX_RUN_DIR"

if [ "$FX_GATE_STATE" = "INCOMPLETE" ] || [ "$FX_GATE_RC" = "124" ]; then
    echo "$FX_ID: the gate layer is INCOMPLETE (exit=${FX_GATE_RC:-?}) - a timeout is NOT a red (R50) -> INCONCLUSIVE"
    restore
    rm -rf "$FX_RUN_DIR"
    exit 4
fi
if ! fx_is_red "$FX_GATE_STATE" "$FX_GATE_RC"; then
    echo "$FX_ID: gate layer state=$FX_GATE_STATE rc=$FX_GATE_RC (want FAIL with a non-124 rc) -> FIXTURE-FAIL"
    exit 3
fi
if [ "$FX_STAGE_VERDICT" != "FAIL" ]; then
    echo "$FX_ID: stage layer verdict=$FX_STAGE_VERDICT (want FAIL) -> FIXTURE-FAIL"
    exit 3
fi

# The verdict the target names is "average < threshold" — the L1_8 average
# falling under the stage bar because one crate's coverage table went below
# it.  Two things can prevent that verdict from being the cause of the red:
#
#   * the gate's RED already existed before the mutation (control rc != 0) —
#     then the mutation cannot be attributed to the red;
#   * the gate took its other fail-closed path (a reduced sample) instead.
#
# Either way the honest verdict is INCONCLUSIVE, not a red: the fixture would
# otherwise claim "this gate can see this defect" while the red it observed
# was already there.
if [ "$CTL_RC" -ne 0 ]; then
    echo "$FX_ID: the gate is ALREADY RED on the intact tree (control rc=$CTL_RC) -> the mutation cannot be attributed to the red -> INCONCLUSIVE"
    echo "$FX_ID: (for the record: the mutation did replace $CRATE's coverage table: ${CTL_MUT_PCT:-<none>}% -> ${MUT_PCT:-<none>}%)"
    exit 4
fi
if [ "$AVERAGE_FAIL" -ne 1 ]; then
    if [ "$REDUCED_SAMPLE" -eq 1 ]; then
        echo "$FX_ID: the gate failed with the REDUCED-SAMPLE verdict, not with 'average < threshold' -> INCONCLUSIVE (the target asks for a below-threshold coverage table)"
    else
        echo "$FX_ID: the gate did not report 'average < threshold' -> FIXTURE-FAIL"
        exit 3
    fi
    exit 4
fi
if [ -z "$MUT_PCT" ]; then
    echo "$FX_ID: the mutated crate's coverage number is not in the report -> FIXTURE-FAIL"
    exit 3
fi

echo "$FX_ID: gate rc=$FX_GATE_RC (FAIL, not a timeout) + stage verdict=$FX_STAGE_VERDICT, $CRATE coverage table = ${CTL_MUT_PCT:-<none>}% -> $MUT_PCT% -> FIXTURE-PASS (the gate detects a below-threshold coverage table)"
exit 0
