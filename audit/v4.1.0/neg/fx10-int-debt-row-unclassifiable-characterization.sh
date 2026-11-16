#!/usr/bin/env bash
# fx10-int-debt-row-unclassifiable-characterization.sh — negative fixture for
# expect: red
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

# 2026-11-16（#5598 fail-closed · D-03）：本夹具原本是**特征化**夹具
# （expect: green-characterization），用来证明「债务行不可分类时门禁仍 rc=0」
# 这条 fail-open 真实存在（#5600 候选发现）。该洞已由 D-03 裁决闭合
# （列 4 非空且未知 ⇒ FAIL；FAILED_ITEMS 必须参与退出码判定）⇒ 期望翻转为 **red**：
# 债务行不可分类时门禁**必须** rc≠0。
if [ "$RC" -ne 0 ]; then
    echo "fx10-int-debt-row-unclassifiable-characterization: gate rc=$RC (want non-zero) -> FIXTURE-PASS (the gate detects the unclassifiable row)"
    exit 0
fi
echo "fx10-int-debt-row-unclassifiable-characterization: gate rc=0 while reporting the item as failed -> FIXTURE-FAIL (the fail-open hole is still open)"
exit 3
