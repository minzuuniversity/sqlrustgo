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

set -uo pipefail

T="${1:?usage: fx02-anti-ignore-registry-drift.sh <scratch-tree>}"
GATE="scripts/gate/check_anti_ignore_gate.sh"
EXPECT="red"

restore() {
    git -C "$T" checkout -- . >/dev/null 2>&1 || true
    git -C "$T" clean -qfd >/dev/null 2>&1 || true
}
restore

# --- 1. inject the fault -----------------------------------------------------
TARGET="$(cd "$T" && git ls-files 'tests/*.rs' | head -1)"
[ -n "$TARGET" ] || { echo "no tests/*.rs file to mutate"; exit 2; }
printf '\n#[ignore]\n#[test]\nfn zz_neg_fixture_probe_5578() {}\n' >> "$T/$TARGET"

# --- 2. run the real gate against the mutated tree ---------------------------
LOG="$(mktemp)"
cd "$T"
timeout 300 bash "$GATE" >"$LOG" 2>&1
RC=$?
restore

echo "----- fx02-anti-ignore-registry-drift : gate output (first 15 lines) -----"
sed -n '1,15p' "$LOG"
echo "----- end -----"
rm -f "$LOG"

if [ "$EXPECT" = "red" ]; then
    if [ "$RC" -ne 0 ]; then
        echo "fx02-anti-ignore-registry-drift: gate rc=$RC (want non-zero) -> FIXTURE-PASS (the gate detects the fault)"
        exit 0
    fi
    echo "fx02-anti-ignore-registry-drift: gate rc=0 (want non-zero) -> FIXTURE-FAIL (the gate cannot see the fault)"
    exit 3
fi

# characterization arm: the documented claim is that this gate *cannot* go red.
if [ "$RC" -eq 0 ]; then
    echo "fx02-anti-ignore-registry-drift: gate rc=0 under a fault that its own output marks as a finding -> UNFALSIFIABLE (as documented)"
    exit 0
fi
echo "fx02-anti-ignore-registry-drift: gate rc=$RC -> the 'unfalsifiable' claim is WRONG (the gate does go red)"
exit 3
