#!/usr/bin/env bash
# fx23-criterion6-reviewer-list-edited-by-non-owner.sh — negative fixture for
# the criterion-6 signer check in
# docs/refactor/v4.1.0-c/takeover/gen_verdict.py
# fixture-id: fx23-criterion6-reviewer-list-edited-by-non-owner
# expect: red
# fault: the owner-approved reviewer list gets a name, but the commit that
#        changed the block is authored by an identity that CODEOWNERS does not
#        register as an owner of the ruling document -- the "anyone can put
#        themselves on the list" bypass.
#
# NOT a gate: it never runs in ci_gate.sh, never changes the gate denominator
# (168).  It mutates a **scratch** checkout, runs the real generator there,
# and restores the tree.
#
# The key under test: `c6_reviewer_list_provenance_is_owner`
#   arm A (control) the list block is changed by an OWNER identity   -> True
#   arm B (fault)   the list block is changed by a NON-OWNER identity -> False
# The control arm is what makes this a fixture and not a tautology: a key that
# is always False would look like a working detector.
#
# Why the check exists (the finding this fixture locks down): the reviewer
# list is a Markdown table inside V410-OWNER-RULING.md.  Before the check,
# ANY commit that edited that table put a name into it, and criterion 6 then
# accepted that name as a qualified signer.

set -uo pipefail

T="${1:?usage: fx23-criterion6-reviewer-list-edited-by-non-owner.sh <scratch-tree>}"
FX_ID="fx23-criterion6-reviewer-list-edited-by-non-owner"
RULING="docs/refactor/v4.1.0-c/takeover/V410-OWNER-RULING.md"
GENERATOR="docs/refactor/v4.1.0-c/takeover/gen_verdict.py"
KEY="c6_reviewer_list_provenance_is_owner"
OWNER_NAME="yinglichina8848"
OWNER_EMAIL="yinglichina8848@guizhouminzuuniversity.edu.cn"
NONOWNER_NAME="probe-bot"
NONOWNER_EMAIL="probe-bot@example.invalid"

[ -d "$T" ] || { echo "$FX_ID: scratch tree '$T' is not a directory -> INCONCLUSIVE"; exit 4; }
ORIG="$(git -C "$T" rev-parse HEAD 2>/dev/null)" \
    || { echo "$FX_ID: '$T' is not a git checkout -> INCONCLUSIVE"; exit 4; }
[ -f "$T/$RULING" ] || { echo "$FX_ID: $RULING not found -> INCONCLUSIVE"; exit 4; }
[ -f "$T/$GENERATOR" ] || { echo "$FX_ID: $GENERATOR not found -> INCONCLUSIVE"; exit 4; }

restore() {
    git -C "$T" reset --hard "$ORIG" >/dev/null 2>&1 || true
    git -C "$T" clean -qfd -e audit/v4.1.0/runs/ >/dev/null 2>&1 || true
}
restore

# --- write the reviewer list (one named reviewer) ---------------------------
write_list() {
    python3 - "$T/$RULING" <<'PY'
import sys
path = sys.argv[1]
BEGIN = "<!-- OWNER_APPROVED_REVIEWERS:BEGIN -->"
END = "<!-- OWNER_APPROVED_REVIEWERS:END -->"
txt = open(path, encoding="utf-8").read()
if BEGIN not in txt or END not in txt:
    sys.exit("reviewer marker block not found")
block = txt.split(BEGIN, 1)[1].split(END, 1)[0]
lines = [l for l in block.splitlines()
         if not l.strip().startswith("| （**当前为空**")]
sep = next((i for i, l in enumerate(lines) if l.strip().startswith("|---")), None)
if sep is None:
    sys.exit("no table separator in the reviewer block")
lines.insert(sep + 1, "| 1 | `probe-bot` | 全量 | 2026-11-26 |")
txt = txt.replace(block, "\n".join(lines), 1)
open(path, "w", encoding="utf-8").write(txt)
PY
}

commit_list() {   # <name> <email> <message>
    git -C "$T" add "$RULING" >/dev/null 2>&1
    git -C "$T" -c user.name="$1" -c user.email="$2" \
        commit -q -m "$3" >/dev/null 2>&1
}

# --- run the real generator and read the key --------------------------------
run_generator() {   # <out.json> -> echoes the key value ("True"/"False"/"")
    python3 "$T/$GENERATOR" --repo "$T" --out "$1" --allow-dirty >/dev/null 2>&1
    python3 - "$1" "$KEY" <<'PY'
import json, sys
try:
    doc = json.load(open(sys.argv[1], encoding="utf-8"))
except Exception:
    print(""); raise SystemExit(0)
c6 = [c for c in doc.get("criteria", []) if c.get("id") == 6]
if not c6:
    print(""); raise SystemExit(0)
v = (c6[0].get("conditions") or {}).get(sys.argv[2])
print("" if v is None else ("True" if v else "False"))
PY
}

TMP="$(mktemp -d "${TMPDIR:-/tmp}/fx23.XXXXXX")"
trap 'rm -rf "$TMP"; restore' EXIT

# --- arm A: the list is changed by an OWNER identity (control) --------------
write_list || { echo "$FX_ID: could not write the reviewer list -> INCONCLUSIVE"; exit 4; }
commit_list "$OWNER_NAME" "$OWNER_EMAIL" "fx23: owner-authored reviewer list" \
    || { echo "$FX_ID: owner commit failed -> INCONCLUSIVE"; exit 4; }
CONTROL="$(run_generator "$TMP/control.json")"
echo "----- $FX_ID : arm A (owner-authored list) -----"
echo "control $KEY = ${CONTROL:-<unreadable>}"
if [ "$CONTROL" != "True" ]; then
    echo "$FX_ID: control arm did not reach $KEY = True -> INCONCLUSIVE"
    echo "        (the detector cannot be shown to work at all)"
    exit 4
fi

# --- arm B: the same list, but changed by a NON-OWNER identity (fault) -----
restore
write_list || { echo "$FX_ID: could not write the reviewer list -> INCONCLUSIVE"; exit 4; }
commit_list "$NONOWNER_NAME" "$NONOWNER_EMAIL" "fx23: non-owner edits the reviewer list" \
    || { echo "$FX_ID: non-owner commit failed -> INCONCLUSIVE"; exit 4; }
FAULT="$(run_generator "$TMP/fault.json")"
echo "----- $FX_ID : arm B (non-owner-authored list) -----"
echo "fault $KEY = ${FAULT:-<unreadable>}"
python3 - "$TMP/fault.json" <<'PY'
import json, sys
try:
    doc = json.load(open(sys.argv[1], encoding="utf-8"))
except Exception:
    raise SystemExit(0)
c6 = [c for c in doc.get("criteria", []) if c.get("id") == 6]
if c6:
    obs = c6[0].get("observed", "")
    for line in obs.splitlines():
        if "名单来源核对" in line:
            print(line[:400])
PY

restore
if [ "$FAULT" = "False" ]; then
    echo "$FX_ID: $KEY flipped to False when a non-owner edited the list -> FIXTURE-PASS (the bypass is blocked)"
    exit 0
fi
echo "$FX_ID: $KEY stayed ${FAULT:-<unreadable>} after a non-owner edited the list -> FIXTURE-FAIL (the bypass is NOT blocked)"
exit 3
