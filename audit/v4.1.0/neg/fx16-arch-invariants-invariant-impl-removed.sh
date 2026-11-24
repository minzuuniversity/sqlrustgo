#!/usr/bin/env bash
# fx16-arch-invariants-invariant-impl-removed.sh — negative fixture for
# scripts/gate/check_arch_invariants.sh
# fixture-id: fx16-arch-invariants-invariant-impl-removed
# expect: red
# fault: remove the implementation subject of one architectural invariant
#        (C-ARCH-05: src/execution_engine.rs) — the invariant's own
#        implementation is the file it constrains.
#
# WHY THIS INVARIANT AND NOT ANOTHER ONE
# --------------------------------------
# The gate checks five invariants (C-ARCH-01..05).  "摘掉一条 invariant 的
# 实现" is only *realisable* for one of them:
#
#   C-ARCH-01  LocalExecutor has NO `txn_manager:` field — its
#              implementation is an ABSENCE.  There is nothing to remove.
#   C-ARCH-02  same shape (no `write_buffer:` field).
#   C-ARCH-03  business crates may use storage directly — the gate reports it
#              as INFO, it has no FAIL path at all.
#   C-ARCH-04  no `execute("...")` outside the parser — again an absence.
#   C-ARCH-05  src/execution_engine.rs exists AND is < 1600 lines — its
#              implementation subject is a real file, so it CAN be removed.
#
# C-ARCH-05 is also the invariant with a fail-closed contract: a renamed,
# moved or deleted target used to be reported as PASS (the gate's own
# comment records the fix: `wc -l` of a MISSING file yields 0, and 0 is
# never > the limit).  So removing the target must turn the gate RED, not
# silently green — that is exactly the hole this fixture closes.
#
# Two arms, both required:
#   A control  on the intact tree the gate exits 0 (so the red in arm B is
#              attributable to the mutation, not to a pre-existing failure)
#   B fault   the target file is removed ⇒ the gate must fail closed
#
# Part of the v4.1.0 readiness plan (Task 8).  NOT a gate: it never runs in
# ci_gate.sh, never changes the gate denominator (168).  It mutates a
# **scratch** checkout, runs the real gate there, and the scratch tree is
# restored afterwards.

set -uo pipefail

T="${1:?usage: fx16-arch-invariants-invariant-impl-removed.sh <scratch-tree>}"
FX_ID="fx16-arch-invariants-invariant-impl-removed"
GATE="scripts/gate/check_arch_invariants.sh"
TARGET="src/execution_engine.rs"

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=audit/v4.1.0/neg/fx_two_layer.sh
. "$HERE/fx_two_layer.sh"

# restore() is the shared one (fx_two_layer.sh): it puts the scratch tree back
# and does not delete the evidence run directory this fixture belongs to.
restore() {
    fx_restore "$T"
}
restore

# --- arm A: control ----------------------------------------------------------
echo "----- fx16 arm A (control: intact tree) -----"
CTL_LOG="$(mktemp)"
( cd "$T" && timeout 300 bash "$GATE" ) >"$CTL_LOG" 2>&1
CTL_RC=$?
echo "control rc=$CTL_RC (want 0)"
tail -4 "$CTL_LOG"
rm -f "$CTL_LOG"
if [ "$CTL_RC" -ne 0 ]; then
    echo "$FX_ID: the gate is already red on the intact tree (rc=$CTL_RC) -> INCONCLUSIVE (the mutation would not be attributable)"
    restore
    exit 4
fi

# --- inject the fault --------------------------------------------------------
rm -f "$T/$TARGET"

# --- run the real gate through the tri-state runner --------------------------
fx_two_layer_run "$T" "$GATE" "$FX_ID" 600
TWO_LAYER_RC=$?
echo "----- fx16 gate output (tail 12) -----"
tail -12 "$FX_RUN_DIR"/001-*.stdout 2>/dev/null || true
echo "----- end -----"

if [ "$TWO_LAYER_RC" -ne 0 ]; then
    echo "$FX_ID: could not run the two-layer check -> INCONCLUSIVE"
    restore
    rm -rf "$FX_RUN_DIR"
    exit 4
fi

# the gate must have failed CLOSED on the removed invariant subject
GATE_TAIL="$(cat "$FX_RUN_DIR"/001-*.stdout 2>/dev/null || true)"
case "$GATE_TAIL" in
    *"C-ARCH-05"*"not found"*) CLOSED=1 ;;
    *"C-ARCH-05"*) CLOSED=0 ;;
    *) CLOSED=0 ;;
esac

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
if [ "$CLOSED" -ne 1 ]; then
    echo "$FX_ID: the gate did not report C-ARCH-05 as unevaluable -> FIXTURE-FAIL (it must fail closed, not pass)"
    exit 3
fi

echo "$FX_ID: gate rc=$FX_GATE_RC (FAIL, not a timeout) + stage verdict=$FX_STAGE_VERDICT -> FIXTURE-PASS (the gate detects the removed invariant)"
exit 0
