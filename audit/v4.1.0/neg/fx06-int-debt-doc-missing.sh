#!/usr/bin/env bash
# fx06-int-debt-doc-missing.sh — negative fixture for scripts/gate/check_int_debt.sh
# fixture-id: fx06-int-debt-doc-missing
# expect: red
# fault: delete the CROSS-VERSION-DEBT.md the gate reads for INT-1~INT-4
#
# Part of P1-3 (see docs/releases/v4.1.0/NEG_FIXTURES_v4.1.0.jsonl).
# NOT a gate: it never runs in ci_gate.sh, never changes the gate denominator.
# It mutates a **scratch** checkout, runs the real gate there, and checks that
# the gate's verdict actually moves. The scratch tree is restored after every
# run, so nothing here can leak into a commit.

set -uo pipefail

T="${1:?usage: fx06-int-debt-doc-missing.sh <scratch-tree>}"
GATE="scripts/gate/check_int_debt.sh"
EXPECT="red"

restore() {
    git -C "$T" checkout -- . >/dev/null 2>&1 || true
    git -C "$T" clean -qfd >/dev/null 2>&1 || true
}
restore

# --- 1. inject the fault -----------------------------------------------------
for f in docs/releases/v3.8.0/debt/CROSS-VERSION-DEBT.md \
         docs/releases/v3.8.0/CROSS-VERSION-DEBT.md \
         docs/releases/v3.8.0/archived/CROSS-VERSION-DEBT.md; do
    rm -f "$T/$f"
done

# --- 2. run the real gate against the mutated tree ---------------------------
LOG="$(mktemp)"
cd "$T"
timeout 300 bash "$GATE" >"$LOG" 2>&1
RC=$?
restore

echo "----- fx06-int-debt-doc-missing : gate output (first 15 lines) -----"
sed -n '1,15p' "$LOG"
echo "----- end -----"
rm -f "$LOG"

if [ "$EXPECT" = "red" ]; then
    if [ "$RC" -ne 0 ]; then
        echo "fx06-int-debt-doc-missing: gate rc=$RC (want non-zero) -> FIXTURE-PASS (the gate detects the fault)"
        exit 0
    fi
    echo "fx06-int-debt-doc-missing: gate rc=0 (want non-zero) -> FIXTURE-FAIL (the gate cannot see the fault)"
    exit 3
fi

# characterization arm: the documented claim is that this gate *cannot* go red.
if [ "$RC" -eq 0 ]; then
    echo "fx06-int-debt-doc-missing: gate rc=0 under a fault that its own output marks as a finding -> UNFALSIFIABLE (as documented)"
    exit 0
fi
echo "fx06-int-debt-doc-missing: gate rc=$RC -> the 'unfalsifiable' claim is WRONG (the gate does go red)"
exit 3
