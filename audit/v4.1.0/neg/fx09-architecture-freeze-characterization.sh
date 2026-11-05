#!/usr/bin/env bash
# fx09-architecture-freeze-characterization.sh — negative fixture for scripts/gate/check_architecture_freeze.sh
# fixture-id: fx09-architecture-freeze-characterization
# expect: green-characterization
# fault: delete every DriftGate reference so the gate prints 'FAIL (0 references)'
#
# Part of P1-3 (see docs/releases/v4.1.0/NEG_FIXTURES_v4.1.0.jsonl).
# NOT a gate: it never runs in ci_gate.sh, never changes the gate denominator.
# It mutates a **scratch** checkout, runs the real gate there, and checks that
# the gate's verdict actually moves. The scratch tree is restored after every
# run, so nothing here can leak into a commit.

set -uo pipefail

T="${1:?usage: fx09-architecture-freeze-characterization.sh <scratch-tree>}"
GATE="scripts/gate/check_architecture_freeze.sh"
EXPECT="green-characterization"

restore() {
    git -C "$T" checkout -- . >/dev/null 2>&1 || true
    git -C "$T" clean -qfd >/dev/null 2>&1 || true
}
restore

# --- 1. inject the fault -----------------------------------------------------
grep -rl "DriftGate" "$T/crates" --include="*.rs" 2>/dev/null \
  | xargs -I{} sed -i '' 's/DriftGate/ZzRemovedByNegFixture5578/g' {}

# --- 2. run the real gate against the mutated tree ---------------------------
LOG="$(mktemp)"
cd "$T"
timeout 300 bash "$GATE" >"$LOG" 2>&1
RC=$?
restore

echo "----- fx09-architecture-freeze-characterization : gate output (first 15 lines) -----"
sed -n '1,15p' "$LOG"
echo "----- end -----"
rm -f "$LOG"

if [ "$EXPECT" = "red" ]; then
    if [ "$RC" -ne 0 ]; then
        echo "fx09-architecture-freeze-characterization: gate rc=$RC (want non-zero) -> FIXTURE-PASS (the gate detects the fault)"
        exit 0
    fi
    echo "fx09-architecture-freeze-characterization: gate rc=0 (want non-zero) -> FIXTURE-FAIL (the gate cannot see the fault)"
    exit 3
fi

# characterization arm: the documented claim is that this gate *cannot* go red.
if [ "$RC" -eq 0 ]; then
    echo "fx09-architecture-freeze-characterization: gate rc=0 under a fault that its own output marks as a finding -> UNFALSIFIABLE (as documented)"
    exit 0
fi
echo "fx09-architecture-freeze-characterization: gate rc=$RC -> the 'unfalsifiable' claim is WRONG (the gate does go red)"
exit 3
