#!/usr/bin/env bash
# fx24-criterion6-unreviewed-mass-marked.sh — negative fixture for
# the criterion-6 "UNREVIEWABLE set" check in
# docs/refactor/v4.1.0-c/takeover/gen_verdict.py
# fixture-id: fx24-criterion6-unreviewed-mass-marked
# expect: red
# fault: the L1 slice of TEST_COVERAGE_MATRIX.json is mass-marked
#        ASSERTION_UNREVIEWABLE (all but one case), the surviving case is
#        signed by a name that IS on the owner-approved reviewer list, and the
#        list itself is changed by the owner identity -- i.e. the complete
#        bypass: "43 cases are undecidable, the one that is decidable is
#        signed by me".
#
# NOT a gate: it never runs in ci_gate.sh, never changes the gate denominator
# (168).  It mutates a **scratch** checkout, runs the real generator there,
# and restores the tree.
#
# The key under test: `c6_unreviewed_set_within_owner_baseline`
#   arm A (control) the UNREVIEWABLE set is inside the owner baseline -> True
#   arm B (fault)   cases outside the baseline are marked UNREVIEWABLE  -> False
#
# Why the check exists: `judged_pass` excludes ASSERTION_UNREVIEWABLE, so
# shrinking that set is how a caller avoids having to produce a qualified
# signature at all.  The owner baseline (tests/baseline/
# oracle_unreviewed_registry.json) makes growing that set an explicit act.

set -uo pipefail

T="${1:?usage: fx24-criterion6-unreviewed-mass-marked.sh <scratch-tree>}"
FX_ID="fx24-criterion6-unreviewed-mass-marked"
RULING="docs/refactor/v4.1.0-c/takeover/V410-OWNER-RULING.md"
MATRIX="docs/releases/v4.1.0/refactoring/TEST_COVERAGE_MATRIX.json"
GENERATOR="docs/refactor/v4.1.0-c/takeover/gen_verdict.py"
KEY="c6_unreviewed_set_within_owner_baseline"
OWNER_NAME="yinglichina8848"
OWNER_EMAIL="yinglichina8848@guizhouminzuuniversity.edu.cn"

[ -d "$T" ] || { echo "$FX_ID: scratch tree '$T' is not a directory -> INCONCLUSIVE"; exit 4; }
ORIG="$(git -C "$T" rev-parse HEAD 2>/dev/null)" \
    || { echo "$FX_ID: '$T' is not a git checkout -> INCONCLUSIVE"; exit 4; }
for f in "$RULING" "$MATRIX" "$GENERATOR"; do
    [ -f "$T/$f" ] || { echo "$FX_ID: $f not found -> INCONCLUSIVE"; exit 4; }
done

restore() {
    git -C "$T" reset --hard "$ORIG" >/dev/null 2>&1 || true
    git -C "$T" clean -qfd -e audit/v4.1.0/runs/ >/dev/null 2>&1 || true
}
restore

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

commit_list() {
    git -C "$T" add "$RULING" >/dev/null 2>&1
    git -C "$T" -c user.name="$1" -c user.email="$2" \
        commit -q -m "$3" >/dev/null 2>&1
}

# Mark every L1 case but one as ASSERTION_UNREVIEWABLE, and sign the survivor.
mass_mark() {
    python3 - "$T/$MATRIX" <<'PY'
import json, sys
path = sys.argv[1]
doc = json.load(open(path, encoding="utf-8"))
l1 = [c for c in doc["cases"] if c.get("feature") in ("T1", "T2", "T3")]
if not l1:
    sys.exit("no L1 cases in the matrix")
for c in l1:
    c["assertion_review_verdict"] = "ASSERTION_UNREVIEWABLE"
    c["assertion_review_reason"] = "R9"
    c["assertion_reviewed_by"] = ""
    c["assertion_review_performed_by"] = "task7-executor/minimax-m3.1-flash"
keep = l1[0]
keep["assertion_review_verdict"] = "JUDGED_PASS"
keep["assertion_review_reason"] = ""
keep["assertion_reviewed_by"] = "probe-bot"
json.dump(doc, open(path, "w", encoding="utf-8"), ensure_ascii=False, indent=2)
PY
}

run_generator() {   # <out.json> -> echoes the key value
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

TMP="$(mktemp -d "${TMPDIR:-/tmp}/fx24.XXXXXX")"
trap 'rm -rf "$TMP"; restore' EXIT

# --- arm A: the UNREVIEWABLE set is inside the owner baseline (control) -----
write_list || { echo "$FX_ID: could not write the reviewer list -> INCONCLUSIVE"; exit 4; }
commit_list "$OWNER_NAME" "$OWNER_EMAIL" "fx24: owner-authored reviewer list" \
    || { echo "$FX_ID: owner commit failed -> INCONCLUSIVE"; exit 4; }
CONTROL="$(run_generator "$TMP/control.json")"
echo "----- $FX_ID : arm A (control) -----"
echo "control $KEY = ${CONTROL:-<unreadable>}"
if [ "$CONTROL" != "True" ]; then
    echo "$FX_ID: control arm did not reach $KEY = True -> INCONCLUSIVE"
    exit 4
fi

# --- arm B: the mass-marking (the bypass) -----------------------------------
mass_mark || { echo "$FX_ID: could not mass-mark the matrix -> INCONCLUSIVE"; exit 4; }
FAULT="$(run_generator "$TMP/fault.json")"
echo "----- $FX_ID : arm B (mass-marked UNREVIEWABLE) -----"
echo "fault $KEY = ${FAULT:-<unreadable>}"
python3 - "$TMP/fault.json" <<'PY'
import json, sys
try:
    doc = json.load(open(sys.argv[1], encoding="utf-8"))
except Exception:
    raise SystemExit(0)
c6 = [c for c in doc.get("criteria", []) if c.get("id") == 6]
if c6:
    for line in c6[0].get("observed", "").splitlines():
        if "不可判定" in line and "核对" in line:
            print(line[:400])
PY

restore
if [ "$FAULT" = "False" ]; then
    echo "$FX_ID: $KEY flipped to False when cases outside the owner baseline were marked UNREVIEWABLE -> FIXTURE-PASS (the bypass is blocked)"
    exit 0
fi
echo "$FX_ID: $KEY stayed ${FAULT:-<unreadable>} after the mass-marking -> FIXTURE-FAIL (the bypass is NOT blocked)"
exit 3
