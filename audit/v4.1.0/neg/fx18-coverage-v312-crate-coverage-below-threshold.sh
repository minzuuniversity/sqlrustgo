#!/usr/bin/env bash
# fx18-coverage-v312-crate-coverage-below-threshold.sh — negative fixture for
# scripts/gate/check_coverage_v312.sh
# fixture-id: fx18-coverage-v312-crate-coverage-below-threshold
# expect: red
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
# shellcheck source=audit/v4.1.0/neg/fx_two_layer.sh
. "$HERE/fx_two_layer.sh"

# restore() is the shared one (fx_two_layer.sh): it puts the scratch tree back
# and does not delete the evidence run directory this fixture belongs to.
restore() {
    fx_restore "$T"
}
restore

if [ ! -f "$T/$TARGET" ]; then
    echo "$FX_ID: $TARGET not found in the scratch tree -> INCONCLUSIVE"
    exit 4
fi

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

if ! fx_is_red "$FX_GATE_STATE" "$FX_GATE_RC"; then
    echo "$FX_ID: gate layer state=$FX_GATE_STATE rc=$FX_GATE_RC (want FAIL with a non-124 rc) -> FIXTURE-FAIL"
    exit 3
fi
if [ "$FX_STAGE_VERDICT" != "FAIL" ]; then
    echo "$FX_ID: stage layer verdict=$FX_STAGE_VERDICT (want FAIL) -> FIXTURE-FAIL"
    exit 3
fi
if [ "$AVERAGE_FAIL" -ne 1 ]; then
    if [ "$REDUCED_SAMPLE" -eq 1 ]; then
        echo "$FX_ID: the gate failed with the REDUCED-SAMPLE verdict, not with 'average < threshold' -> FIXTURE-FAIL (the target asks for a below-threshold coverage table)"
    else
        echo "$FX_ID: the gate did not report 'average < threshold' -> FIXTURE-FAIL"
    fi
    exit 3
fi
if [ -z "$MUT_PCT" ]; then
    echo "$FX_ID: the mutated crate's coverage number is not in the report -> FIXTURE-FAIL"
    exit 3
fi

echo "$FX_ID: gate rc=$FX_GATE_RC (FAIL, not a timeout) + stage verdict=$FX_STAGE_VERDICT, $CRATE coverage table = $MUT_PCT% -> FIXTURE-PASS (the gate detects a below-threshold coverage table)"
exit 0
