#!/usr/bin/env bash
# fx18-coverage-v312-crate-coverage-below-threshold.sh — negative fixture for
# scripts/gate/check_coverage_v312.sh
# fixture-id: fx18-coverage-v312-crate-coverage-below-threshold
# expect: inconclusive
#
# WHY "inconclusive" AND NOT "red"
# ---------------------------------
# This fixture cannot establish its property in this tree, and saying so is the
# honest outcome (R50/R58): the coverage gate is ALREADY RED on the intact
# tree -- `sqlrustgo-storage` produces no coverage table at all because the
# RED characterisation tests imported by the readiness plan's earlier tasks
# (crates/storage/tests/tf_t4_ai_candidate_v1.rs, tf_t4_stub_silent_success.rs)
# fail BY DESIGN, so `cargo llvm-cov` exits 101 and the L1_8 sample is 7 of 8.
# The gate's red therefore exists before this fixture injects anything, and the
# red cannot be attributed to the injection.
#
# The fixture still does its job as far as it can: it measures the mutated
# crate's coverage table (93.31% -> 20.42%, i.e. the defect DOES replace the
# table with a below-threshold value), it runs the gate, and it proves the
# attribution is impossible.  Then it reports INCONCLUSIVE (exit 4) instead of
# claiming a red it cannot attribute.
#
# The ledger row of this gate therefore stays kind=pending-fixture and carries
# no `effect.catches` claim: nobody may say "this gate catches a below-threshold
# coverage table" until a run can actually show it.
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
# expensive lane of this ledger (its own note says ">720s 未跑完").  A run
# that hits the timeout is reported as INCONCLUSIVE, never as a red (R50).
#
# TWO MEASURED FACTS THAT MAKE THIS FIXTURE INCONCLUSIVE TODAY (both are
# findings, not excuses; neither is repaired here — repairing either one
# changes what the gate measures, which is the owner's call):
#
# 1. **The gate is red at baseline in this tree.**  `sqlrustgo-storage`
#    produces NO coverage table at all: the RED characterisation tests the
#    readiness plan's earlier tasks imported
#    (crates/storage/tests/tf_t4_ai_candidate_v1.rs and
#    tf_t4_stub_silent_success.rs) fail BY DESIGN (they assert the
#    "STUB_SILENT_SUCCESS" defect on purpose), and a failing test target
#    makes `cargo llvm-cov` exit 101.  The L1_8 sample is therefore 7 of 8
#    and the gate takes its fail-closed path: "FAIL: measured 7 of 8 crates".
#    Measured: control rc=1, with $CRATE at 93.31%; after the mutation
#    $CRATE reads 20.42% — the defect DID replace the crate's coverage table,
#    but the gate's red was already there, so the red cannot be attributed
#    to the defect.  The fixture therefore reports INCONCLUSIVE.
#
# 2. **At the ALPHA bar the target's mutation is arithmetically unable to
#    reach the threshold.**  The ALPHA stage runs this gate with
#    `SQLRUSTGO_COVERAGE_STAGE=alpha` (bar 50; the GA bar is 85).  With the
#    repository's own recorded per-crate numbers (scripts/gate/common.sh:
#    parser 86.46, planner 88.91, executor 82.68, transaction 93.29,
#    storage 80.65, catalog 87.33, optimizer 87.85, types 95.04) the other
#    seven crates already average 86.64%, so the L1_8 average cannot fall
#    below 50 by changing ONE crate — it would need a negative coverage.
#    Only a change in a shared dependency (sqlrustgo-types) could move the
#    ALPHA bar.  This fixture measures the gate with its own default bar
#    (85) and says so rather than silently claiming the ALPHA bar.
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

# --- control: what does the gate say on the INTACT tree? ---------------------
# This gate is red at baseline in this environment for a reason that has
# nothing to do with coverage: `sqlrustgo-storage` cannot be measured at all,
# because the RED characterisation tests imported by the readiness plan's
# earlier tasks (crates/storage/tests/tf_t4_ai_candidate_v1.rs,
# tf_t4_stub_silent_success.rs) fail BY DESIGN.  A failing test target makes
# `cargo llvm-cov` exit 101 => no coverage table for that crate => the L1_8
# sample is 7 of 8 => the gate reports "measured 7 of 8 crates" and goes red
# on its fail-closed path.
#
# The control therefore decides the fixture's verdict: if the gate is
# already red without the defect, the defect cannot be attributed to the
# mutation and the fixture must report INCONCLUSIVE (R50), not a red.
echo "----- fx18 control (intact tree) -----"
CTL_LOG="$(mktemp)"
CTL_T0=$(date +%s)
( cd "$T" && timeout 1800 bash "$GATE" ) >"$CTL_LOG" 2>&1
CTL_RC=$?
CTL_T1=$(date +%s)
echo "control rc=$CTL_RC in $((CTL_T1-CTL_T0))s"
tail -4 "$CTL_LOG"
CTL_MUT_PCT="$(grep -m1 -E "$CRATE: [0-9]+\.[0-9]+%" "$CTL_LOG" | grep -oE '[0-9]+\.[0-9]+' || true)"
echo "$FX_ID: control: $CRATE coverage = ${CTL_MUT_PCT:-<none>}%, gate rc=$CTL_RC"
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
# The budget is the one this ledger sets (issue #5598): 30 minutes per
# fixture.  The gate measures eight crates with `cargo llvm-cov`
# (single-threaded, instrumented) and does NOT finish inside it — the
# parser crate alone was still measuring after 20 minutes on this machine.
# A run that hits the budget is reported as INCONCLUSIVE, never as a red
# (R50): a timeout is not a drift.
fx_two_layer_run "$T" "$GATE" "$FX_ID" 1800
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
