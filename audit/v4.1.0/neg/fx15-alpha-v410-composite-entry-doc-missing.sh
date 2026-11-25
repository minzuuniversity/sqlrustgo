#!/usr/bin/env bash
# fx15-alpha-v410-composite-entry-doc-missing.sh — negative fixture for
# scripts/gate/check_alpha_v410.sh
# fixture-id: fx15-alpha-v410-composite-entry-doc-missing
# expect: red
# fault: delete docs/releases/v4.1.0/STAGE.yaml — the file the ALPHA entry
#        component requires.  The composite must go red on its ENTRY
#        component.
#
# WHY THIS FIXTURE IS NOT "rc != 0"
# ---------------------------------
# The ALPHA composite is **already red** on the frozen base, for a reason
# that has nothing to do with this fixture: the quality component's Q1_FMT
# criterion fails on the test_factory material imported by the readiness
# plan's earlier tasks.  A fixture that only asserted "the composite exits
# non-zero" would therefore be satisfied by the pre-existing red and would
# prove nothing about the mutation.
#
# So this fixture asserts the **causal chain**:
#   arm A (control)  the entry component on the intact tree exits 0 — so the
#                    entry component is green before the fault
#   arm B (fault)   with STAGE.yaml deleted, the composite's ENTRY_STATUS
#                    is non-zero and the composite prints
#                    "STATUS: ALPHA GATE BLOCKED"
# The composite's own quality component may stay red; that is not what this
# fixture is about.  What it shows is that the composite *propagates* the
# entry component's verdict — a composite that swallowed it would print
# "ALPHA ENTRY PASS / ALPHA QUALITY BLOCKED" instead.
#
# Cost: the composite runs the quality component too (a release build), so
# this fixture is the expensive one of the seven.  The timeout is set
# generously; a run that hits it is reported as INCONCLUSIVE, never as a
# red (R50: a timeout is not a drift).
#
# Part of the v4.1.0 readiness plan (Task 8).  NOT a gate: it never runs in
# ci_gate.sh, never changes the gate denominator (168).  It mutates a
# **scratch** checkout, runs the real gate there, and the scratch tree is
# restored afterwards.

set -uo pipefail

T="${1:?usage: fx15-alpha-v410-composite-entry-doc-missing.sh <scratch-tree>}"
FX_ID="fx15-alpha-v410-composite-entry-doc-missing"
GATE="scripts/gate/check_alpha_v410.sh"
ENTRY="scripts/gate/check_alpha_entry_v4.10.sh"
DOC="docs/releases/v4.1.0/STAGE.yaml"

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=audit/v4.1.0/neg/neg_two_layer.sh
. "$HERE/neg_two_layer.sh"

# restore() is the shared one (neg_two_layer.sh): it puts the scratch tree back
# and does not delete the evidence run directory this fixture belongs to.
restore() {
    fx_restore "$T"
}
restore

# --- arm A: control (the entry component only — cheap) ----------------------
echo "----- fx15 arm A (control: entry component on the intact tree) -----"
CTL_LOG="$(mktemp)"
( cd "$T" && timeout 600 bash "$ENTRY" ) >"$CTL_LOG" 2>&1
CTL_RC=$?
echo "control entry rc=$CTL_RC (want 0)"
tail -4 "$CTL_LOG"
rm -f "$CTL_LOG"
if [ "$CTL_RC" -ne 0 ]; then
    echo "$FX_ID: the entry component is already red on the intact tree (rc=$CTL_RC) -> INCONCLUSIVE (the mutation would not be attributable)"
    restore
    exit 4
fi

# --- inject the fault --------------------------------------------------------
rm -f "$T/$DOC"

# --- run the real gate through the tri-state runner --------------------------
# The budget is 14400s (4 hours), raised from 7200s.  The archived manifests
# record this fixture hitting the budget twice — rc=124 at 5400s and again at
# 10800s — before it finally completed, so 7200s was below the measurement's
# own worst case.  The lane now warms the target before the fixtures run
# (verify_neg_fixtures.sh) and runs every cargo-consuming fixture before any
# fixture that touches Rust source, so the rebuild that used to eat the
# budget is gone.  The budget is still raised: a timeout here is
# INCONCLUSIVE, never a red (R50), and the cost of raising it is only wall
# clock.
fx_two_layer_run "$T" "$GATE" "$FX_ID" 14400
TWO_LAYER_RC=$?
echo "----- fx15 gate output (tail 16) -----"
tail -16 "$FX_RUN_DIR"/001-*.stdout 2>/dev/null || true
echo "----- end -----"

if [ "$TWO_LAYER_RC" -ne 0 ]; then
    echo "$FX_ID: could not run the two-layer check -> INCONCLUSIVE"
    restore
    rm -rf "$FX_RUN_DIR"
    exit 4
fi

GATE_TAIL="$(cat "$FX_RUN_DIR"/001-*.stdout 2>/dev/null || true)"
ENTRY_STATUS="$(printf '%s\n' "$GATE_TAIL" | grep -m1 -E '^ENTRY_STATUS: *[0-9]+' | grep -oE '[0-9]+$' || true)"
case "$GATE_TAIL" in
    *"STATUS: ALPHA GATE BLOCKED"*) BLOCKED=1 ;;
    *) BLOCKED=0 ;;
esac
echo "$FX_ID: composite ENTRY_STATUS=${ENTRY_STATUS:-<none>} blocked_marker=$BLOCKED"

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
if [ -z "$ENTRY_STATUS" ] || [ "$ENTRY_STATUS" -eq 0 ]; then
    echo "$FX_ID: the composite's ENTRY_STATUS is ${ENTRY_STATUS:-<none>} (want non-zero) -> FIXTURE-FAIL (the composite swallowed the entry component's red)"
    exit 3
fi
if [ "$BLOCKED" -ne 1 ]; then
    echo "$FX_ID: the composite did not print 'STATUS: ALPHA GATE BLOCKED' -> FIXTURE-FAIL"
    exit 3
fi

echo "$FX_ID: gate rc=$FX_GATE_RC (FAIL, not a timeout) + stage verdict=$FX_STAGE_VERDICT, ENTRY_STATUS=$ENTRY_STATUS -> FIXTURE-PASS (the composite propagates the entry component's red)"
exit 0
