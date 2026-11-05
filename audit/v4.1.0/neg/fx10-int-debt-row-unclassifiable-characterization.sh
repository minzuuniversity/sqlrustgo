#!/usr/bin/env bash
# fx10-int-debt-row-unclassifiable-characterization.sh — characterization probe for
# expect: green-characterization
# fixture-id: fx10-int-debt-row-unclassifiable-characterization
# fault: make the INT-2 row of the debt table unclassifiable, and (arm 2) delete it
#
# What this documents (finding #5600 candidate): check_int_debt.sh appends
# "unknown status" and "not in CV debt doc" to FAILED_ITEMS but its decision
# logic only looks at ACTIVE_COUNT / DEFERRED_WITHOUT_PLAN. A debt row that is
# renamed, deleted, or written in a status the parser does not know therefore
# makes the gate report **PASS** — the item simply stops being counted. This
# fixture proves that with the real gate on a scratch tree, and is cited by the
# ledger row as the reason the "hole" claim is not a guess.

set -uo pipefail

T="${1:?usage: fx10-int-debt-row-unclassifiable-characterization.sh <scratch-tree>}"
GATE="scripts/gate/check_int_debt.sh"

restore() {
    git -C "$T" checkout -- . >/dev/null 2>&1 || true
    git -C "$T" clean -qfd >/dev/null 2>&1 || true
}
restore

python3 - "$T" <<'MUT'
import sys, pathlib
tree = pathlib.Path(sys.argv[1])
doc = next((tree / p for p in (
    "docs/releases/v3.8.0/debt/CROSS-VERSION-DEBT.md",
    "docs/releases/v3.8.0/CROSS-VERSION-DEBT.md",
    "docs/releases/v3.8.0/archived/CROSS-VERSION-DEBT.md",
) if (tree / p).is_file()), None)
if doc is None:
    print("no CROSS-VERSION-DEBT.md to mutate")
    raise SystemExit(2)
lines = doc.read_text(encoding="utf-8").split("\n")
hit = 0
for i, ln in enumerate(lines):
    if ln.startswith("| INT-2 ") or ln.startswith("| **INT-2**"):
        lines[i] = "| INT-2 | negfixture5578 | WIP-DONOTUSE | - | - |"
        hit += 1
        break
if hit == 0:
    print("no INT-2 row to mutate")
    raise SystemExit(2)
doc.write_text("\n".join(lines), encoding="utf-8")
MUT
RC_INJECT=$?
if [ "$RC_INJECT" -ne 0 ]; then
    echo "fx10-int-debt-row-unclassifiable-characterization: mutation failed rc=$RC_INJECT"
    exit 2
fi

LOG="$(mktemp)"
cd "$T"
timeout 300 bash "$GATE" >"$LOG" 2>&1
RC=$?
restore

echo "----- fx10 : gate finding lines + summary -----"
LC_ALL=C grep -nE 'INT-2|unknown status|not in CV debt|D7 INT Debt|D7:' "$LOG" | head -12
echo "----- end -----"
rm -f "$LOG"

if [ "$RC" -eq 0 ]; then
    echo "fx10-int-debt-row-unclassifiable-characterization: gate rc=0 while reporting the item as failed -> UNFALSIFIABLE (fail-open hole, as documented)"
    exit 0
fi
echo "fx10-int-debt-row-unclassifiable-characterization: gate rc=$RC -> the fail-open claim is WRONG (the gate does go red)"
exit 3
