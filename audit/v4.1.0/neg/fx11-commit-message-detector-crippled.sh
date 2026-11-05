#!/usr/bin/env bash
# fx11-commit-message-detector-crippled.sh — negative fixture for scripts/gate/check_commit_message.sh
# fixture-id: fx11-commit-message-detector-crippled
# expect: selftest-red
# fault: cripple the gate's OWN detector (the same code path --selftest uses),
#        then the gate's --selftest must go red. A selftest that still passes
#        after its detector is crippled proves nothing.
#
# Part of P1-3 (see docs/releases/v4.1.0/NEG_FIXTURES_v4.1.0.jsonl).
# NOT a gate: it never runs in ci_gate.sh, never changes the gate denominator.
# It mutates a **scratch** checkout, runs the real --selftest there, and
# restores the tree afterwards, so nothing here can leak into a commit.

set -uo pipefail

T="${1:?usage: fx11-commit-message-detector-crippled.sh <scratch-tree>}"
GATE="scripts/gate/check_commit_message.sh"

restore() {
    git -C "$T" checkout -- . >/dev/null 2>&1 || true
    git -C "$T" clean -qfd >/dev/null 2>&1 || true
}
restore

# --- 1. cripple the detector used by both the live check and --selftest ------
python3 - "$T" <<'INJECT'
import pathlib, sys
t = pathlib.Path(sys.argv[1])
p = t / "scripts/gate/check_commit_message.sh"
s = p.read_text(encoding="utf-8")
assert s.count('    if not m or m.group(1) not in allowed:') == 1, "anchor not unique in " + str(p)
p.write_text(s.replace('    if not m or m.group(1) not in allowed:', '    if False:  # crippled by fx11'), encoding="utf-8")
INJECT
RC_INJECT=$?
if [ "$RC_INJECT" -ne 0 ]; then
    echo "fx11-commit-message-detector-crippled: injection failed (anchor moved?) -> INCONCLUSIVE, not a red verdict"
    restore
    exit 4
fi

# --- 2. run the gate's own --selftest against the crippled detector ----------
LOG="$(mktemp)"
cd "$T"
timeout 300 bash "$GATE" --selftest >"$LOG" 2>&1
RC=$?
restore

echo "----- fx11-commit-message-detector-crippled : --selftest output (last 15 lines) -----"
tail -15 "$LOG"
echo "----- end -----"
rm -f "$LOG"

echo "NEUTERED_SELFCHECK_RC=$RC"
if [ "$RC" -ne 0 ]; then
    echo "fx11-commit-message-detector-crippled: --selftest rc=$RC under a crippled detector -> FIXTURE-PASS (the selftest is load-bearing)"
    exit 0
fi
echo "fx11-commit-message-detector-crippled: --selftest rc=0 although its detector was crippled -> FIXTURE-FAIL (the selftest-verified claim is vacuous)"
exit 3
