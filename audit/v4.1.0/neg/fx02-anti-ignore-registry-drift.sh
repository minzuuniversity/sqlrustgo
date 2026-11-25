#!/usr/bin/env bash
# fx02-anti-ignore-registry-drift.sh — negative fixture for scripts/gate/check_anti_ignore_gate.sh
# fixture-id: fx02-anti-ignore-registry-drift
# expect: red
# fault: add a real #[ignore] attribute that the registry does not register
#
# Part of P1-3 (see docs/releases/v4.1.0/NEG_FIXTURES_v4.1.0.jsonl).
# NOT a gate: it never runs in ci_gate.sh, never changes the gate denominator.
# It mutates a **scratch** checkout, runs the real gate there, and checks that
# the gate's verdict actually moves. The scratch tree is restored after every
# run, so nothing here can leak into a commit.
#
# 2026-11-24 (v4.1.0 readiness plan, Task 8) — the fixture was EXTENDED, not
# replaced.  Two things were added, and nothing that existed before was
# removed:
#
#   1. The two-layer evidence.  The gate layer (the gate's own row) and the
#      stage layer (the summariser's independent re-parse of the same
#      results.jsonl) now come from ONE `run_gates.py execute` run, which is
#      what the plan's R54 requires of every ALPHA fixture.  The default
#      behaviour is unchanged: inject the fault, run the REAL gate, assert
#      it goes red, restore.
#
#   2. A **discriminating** assertion.  This gate is already red on the
#      frozen base for an unrelated reason (the tree carries 136 real
#      #[ignore] attributes while the registry registers 135 — a pre-existing
#      drift that this task does not touch).  A fixture that only asserted
#      "rc != 0" would therefore be satisfied by that pre-existing red and
#      would prove nothing about the mutation.  So the fixture compares the
#      drift the gate reports BEFORE the fault with the drift it reports
#      AFTER it, and requires the injected #[ignore] to show up in the
#      number.  That is the difference between "the gate is red" and "the
#      gate sees my fault".

set -uo pipefail

T="${1:?usage: fx02-anti-ignore-registry-drift.sh <scratch-tree>}"
FX_ID="fx02-anti-ignore-registry-drift"
GATE="scripts/gate/check_anti_ignore_gate.sh"
EXPECT="red"

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=audit/v4.1.0/neg/neg_two_layer.sh
. "$HERE/neg_two_layer.sh"

# restore() is the shared one (neg_two_layer.sh): it puts the scratch tree back
# and does not delete the evidence run directory this fixture belongs to.
restore() {
    fx_restore "$T"
}
restore

# --- 0. control: what does the gate report on the intact tree? --------------
CTL_LOG="$(mktemp)"
( cd "$T" && timeout 600 bash "$GATE" ) >"$CTL_LOG" 2>&1
CTL_RC=$?
CTL_TREE="$(grep -m1 -oE 'tree has [0-9]+ real' "$CTL_LOG" | grep -oE '[0-9]+' || true)"
CTL_REG="$(grep -m1 -oE 'registers [0-9]+' "$CTL_LOG" | grep -oE '[0-9]+' || true)"
echo "----- fx02 control (intact tree) -----"
echo "control rc=$CTL_RC tree_ignores=${CTL_TREE:-<none>} registry=${CTL_REG:-<none>}"
rm -f "$CTL_LOG"
if [ -z "$CTL_TREE" ]; then
    echo "$FX_ID: the gate does not report a registry drift on the intact tree -> INCONCLUSIVE (nothing to compare against)"
    restore
    exit 4
fi

# --- 1. inject the fault -----------------------------------------------------
TARGET="$(cd "$T" && git ls-files 'tests/*.rs' | head -1)"
[ -n "$TARGET" ] || { echo "no tests/*.rs file to mutate"; exit 2; }
printf '\n#[ignore]\n#[test]\nfn zz_neg_fixture_probe_5578() {}\n' >> "$T/$TARGET"

# --- 2. run the real gate against the mutated tree ---------------------------
# The gate layer and the stage layer are read off ONE results.jsonl.
fx_two_layer_run "$T" "$GATE" "$FX_ID" 900
TWO_LAYER_RC=$?
echo "----- fx02-anti-ignore-registry-drift : gate output (first 15 lines) -----"
sed -n '1,15p' "$FX_RUN_DIR"/001-*.stdout 2>/dev/null || true
echo "----- end -----"

if [ "$TWO_LAYER_RC" -ne 0 ]; then
    echo "$FX_ID: could not run the two-layer check -> INCONCLUSIVE"
    restore
    exit 4
fi

FAULT_TREE="$(cat "$FX_RUN_DIR"/001-*.stdout "$FX_RUN_DIR"/001-*.stderr 2>/dev/null \
    | grep -m1 -oE 'tree has [0-9]+ real' | grep -oE '[0-9]+' || true)"
if [ -z "$FAULT_TREE" ]; then
    # the gate prints the recount on stdout and the drift verdict on stderr
    FAULT_TREE="$(cat "$FX_RUN_DIR"/001-*.stdout "$FX_RUN_DIR"/001-*.stderr 2>/dev/null \
        | grep -m1 -oE 'real #\[ignore\] attributes in tests/ \+ crates/ = [0-9]+' | grep -oE '[0-9]+$' || true)"
fi
FAULT_REG="$(cat "$FX_RUN_DIR"/001-*.stdout "$FX_RUN_DIR"/001-*.stderr 2>/dev/null \
    | grep -m1 -oE 'registers [0-9]+' | grep -oE '[0-9]+' || true)"
echo "$FX_ID: control tree=${CTL_TREE} registry=${CTL_REG} -> fault tree=${FAULT_TREE:-<none>} registry=${FAULT_REG:-<none>}"

rm -rf "$FX_RUN_DIR"
restore

if [ "$EXPECT" = "red" ]; then
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
    if [ -z "$FAULT_TREE" ]; then
        echo "$FX_ID: the gate's report carries no drift number after the fault -> FIXTURE-FAIL"
        exit 3
    fi
    if [ "$FAULT_TREE" -ne $((CTL_TREE + 1)) ]; then
        echo "$FX_ID: the injected #[ignore] did not show up in the gate's drift number (control ${CTL_TREE} -> fault ${FAULT_TREE}, want $((CTL_TREE + 1))) -> FIXTURE-FAIL (the gate cannot see the fault)"
        exit 3
    fi
    echo "$FX_ID: gate rc=$FX_GATE_RC (FAIL, not a timeout) + stage verdict=$FX_STAGE_VERDICT, drift ${CTL_TREE} -> ${FAULT_TREE} -> FIXTURE-PASS (the gate detects the fault)"
    exit 0
fi

# characterization arm: the documented claim is that this gate *cannot* go red.
if [ "$FX_GATE_RC" -eq 0 ]; then
    echo "$FX_ID: gate rc=0 under a fault that its own output marks as a finding -> UNFALSIFIABLE (as documented)"
    exit 0
fi
echo "$FX_ID: gate rc=$FX_GATE_RC -> the 'unfalsifiable' claim is WRONG (the gate does go red)"
exit 3
